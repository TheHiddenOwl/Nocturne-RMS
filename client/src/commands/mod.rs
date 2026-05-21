pub mod systeminfo;
pub mod reboot;

use async_trait::async_trait;
use protocol::models::CommandRequest;
use serde_json::Value;
use std::collections::HashMap;

#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, payload: Value) -> anyhow::Result<Value>;
}

pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };
        registry.register(Box::new(systeminfo::SystemInfoCommand));
        registry.register(Box::new(reboot::RebootCommand));
        registry
    }

    pub fn register(&mut self, command: Box<dyn Command>) {
        self.commands.insert(command.name().to_string(), command);
    }

    pub async fn handle(&self, req: CommandRequest) -> anyhow::Result<Value> {
        if let Some(cmd) = self.commands.get(&req.command) {
            cmd.execute(req.payload).await
        } else {
            Err(anyhow::anyhow!("Unknown command: {}", req.command))
        }
    }
}
