pub mod api;
pub mod auth;
pub mod tls;
pub mod websocket;

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        State,
    },
    middleware,
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use protocol::{models::{Message, Heartbeat}, API_KEY_HEADER, AGENT_ID_HEADER};
use websocket::SessionManager;
use tokio::sync::mpsc;
use futures_util::{StreamExt, SinkExt};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server_addr: String,
    pub api_keys: Vec<String>,
    pub cert_path: String,
    pub key_path: String,
    pub heartbeat_interval_secs: u64,
}

struct AppState {
    session_manager: Arc<SessionManager>,
    config: Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let settings = config::Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::Environment::with_prefix("APP"))
        .build()?;

    let config: Config = if let Ok(c) = settings.try_deserialize() {
        c
    } else {
        log::warn!("Using default configuration");
        Config {
            server_addr: "0.0.0.0:443".to_string(),
            api_keys: vec!["test-api-key".to_string()],
            cert_path: "certs/cert.pem".to_string(),
            key_path: "certs/key.pem".to_string(),
            heartbeat_interval_secs: 30,
        }
    };

    tls::load_or_generate_certs(Path::new(&config.cert_path), Path::new(&config.key_path))?;

    let session_manager = Arc::new(SessionManager::new());
    let state = Arc::new(AppState {
        session_manager,
        config: config.clone(),
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .layer(middleware::from_fn_with_state(state.clone(), |state: State<Arc<AppState>>, req, next| {
            auth::auth_middleware(req, next, state.config.api_keys.clone())
        }))
        .with_state(state);

    let rustls_config = RustlsConfig::from_pem_file(
        &config.cert_path,
        &config.key_path,
    )
    .await?;

    let addr: SocketAddr = config.server_addr.parse()?;
    log::info!("Server listening on WSS://{}", config.server_addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let agent_id = req
        .headers()
        .get(AGENT_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    ws.on_upgrade(move |socket| handle_socket(socket, state, agent_id))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, agent_id: String) {
    log::info!("Client connected: {}", agent_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    state.session_manager.add_session(agent_id.clone(), tx);

    // Send loop
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if sender.send(WsMessage::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Receive loop
    let session_manager_recv = state.session_manager.clone();
    let agent_id_recv = agent_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let WsMessage::Text(text) = msg {
                if let Ok(protocol_msg) = serde_json::from_str::<Message>(&text) {
                    match protocol_msg {
                        Message::Response(resp) => {
                            log::info!("Received response from {}: {:?}", agent_id_recv, resp);
                        }
                        Message::Heartbeat(hb) => {
                            let now = chrono::Utc::now().timestamp_millis();
                            let latency = (now - hb.timestamp) as u64;
                            session_manager_recv.update_activity(&agent_id_recv, Some(latency));
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    // Heartbeat loop
    let session_manager_hb = state.session_manager.clone();
    let agent_id_hb = agent_id.clone();
    let hb_interval = state.config.heartbeat_interval_secs;

    let mut hb_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(hb_interval));
        loop {
            interval.tick().await;
            let tx = {
                let sessions = session_manager_hb.sessions.read().unwrap();
                sessions.get(&agent_id_hb).map(|s| s.tx.clone())
            };

            if let Some(tx) = tx {
                let hb = Message::Heartbeat(Heartbeat {
                    timestamp: chrono::Utc::now().timestamp_millis(),
                });
                if tx.send(hb).is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => { recv_task.abort(); hb_task.abort(); }
        _ = (&mut recv_task) => { send_task.abort(); hb_task.abort(); }
    }

    state.session_manager.remove_session(&agent_id);
    log::info!("Client disconnected: {}", agent_id);
}
