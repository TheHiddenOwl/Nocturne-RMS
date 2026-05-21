pub mod commands;
pub mod reconnect;
pub mod transport;
pub mod windows;

use transport::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server_url: String,
    pub api_key: String,
    pub agent_id: String,
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
            server_url: "wss://localhost:443/ws".to_string(),
            api_key: "test-api-key".to_string(),
            agent_id: "agent-001".to_string(),
        }
    };

    let client = Client::new(config.server_url, config.api_key, config.agent_id);
    client.run().await;

    Ok(())
}
