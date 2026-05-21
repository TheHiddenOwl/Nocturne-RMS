use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use protocol::{models::{Message, CommandResponse}, API_KEY_HEADER, AGENT_ID_HEADER};
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{
        client::IntoClientRequest,
        http::HeaderValue,
        protocol::WebSocketConfig,
    },
};
use crate::commands::CommandRegistry;
use crate::reconnect::ReconnectStrategy;

pub struct Client {
    server_url: String,
    api_key: String,
    agent_id: String,
    command_registry: Arc<CommandRegistry>,
}

impl Client {
    pub fn new(server_url: String, api_key: String, agent_id: String) -> Self {
        Self {
            server_url,
            api_key,
            agent_id,
            command_registry: Arc::new(CommandRegistry::new()),
        }
    }

    pub async fn run(&self) {
        let mut reconnect_strategy = ReconnectStrategy::new(
            Duration::from_secs(1),
            Duration::from_secs(60),
        );

        loop {
            log::info!("Connecting to {}...", self.server_url);

            match self.connect_and_handle().await {
                Ok(_) => {
                    log::info!("Connection closed gracefully.");
                    reconnect_strategy.reset();
                }
                Err(e) => {
                    log::error!("Connection error: {}. Reconnecting...", e);
                    let delay = reconnect_strategy.next_delay();
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    async fn connect_and_handle(&self) -> anyhow::Result<()> {
        let mut request = self.server_url.clone().into_client_request()?;
        request.headers_mut().insert(API_KEY_HEADER, HeaderValue::from_str(&self.api_key)?);
        request.headers_mut().insert(AGENT_ID_HEADER, HeaderValue::from_str(&self.agent_id)?);

        // For testing, we accept any certificate.
        let connector = tokio_tungstenite::Connector::Rustls(Arc::new(
            rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(NoCertificateVerification))
                .with_no_client_auth(),
        ));

        let (ws_stream, _) = connect_async_tls_with_config(
            request,
            Some(WebSocketConfig::default()),
            false,
            Some(connector),
        ).await?;

        log::info!("Connected successfully!");

        let (mut sender, mut receiver) = ws_stream.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

        // Send task
        let mut send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = serde_json::to_string(&msg).unwrap();
                if sender.send(tokio_tungstenite::tungstenite::Message::Text(json)).await.is_err() {
                    break;
                }
            }
        });

        // Handle incoming messages
        while let Some(msg) = receiver.next().await {
            let msg = msg?;
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                if let Ok(protocol_msg) = serde_json::from_str::<Message>(&text) {
                    match protocol_msg {
                        Message::Command(req) => {
                            let registry = self.command_registry.clone();
                            let tx_inner = tx.clone();
                            tokio::spawn(async move {
                                let request_id = req.request_id;
                                let result = registry.handle(req).await;
                                let response = match result {
                                    Ok(data) => CommandResponse {
                                        request_id,
                                        status: "ok".to_string(),
                                        data,
                                    },
                                    Err(e) => CommandResponse {
                                        request_id,
                                        status: "error".to_string(),
                                        data: serde_json::json!({ "error": e.to_string() }),
                                    },
                                };
                                let _ = tx_inner.send(Message::Response(response));
                            });
                        }
                        Message::Heartbeat(hb) => {
                            // Respond to heartbeat
                            let _ = tx.send(Message::Heartbeat(hb));
                        }
                        _ => {}
                    }
                }
            }
        }

        send_task.abort();
        Ok(())
    }
}

// Helper for accepting all certificates
#[derive(Debug)]
struct NoCertificateVerification;

impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
