use async_trait::async_trait;
use protocol::models::CommandType;
use serde_json::{json, Value};
use crate::commands::Command;
#[cfg(target_os = "windows")]
use std::process::Command as StdCommand;

pub struct RebootCommand;

#[async_trait]
impl Command for RebootCommand {
    fn command_type(&self) -> CommandType {
        CommandType::Reboot
    }

    async fn execute(&self, _payload: Value) -> anyhow::Result<Value> {
        #[cfg(target_os = "windows")]
        {
            log::info!("Reboot triggered. Executing shutdown /r /t 30");
            StdCommand::new("shutdown")
                .args(["/r", "/t", "30"])
                .spawn()?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            log::info!("Reboot command received, but not on Windows. Skipping actual reboot.");
        }

        Ok(json!({"status": "reboot_initiated"}))
    }
}
