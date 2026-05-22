use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Command(CommandRequest),
    Response(CommandResponse),
    Heartbeat(Heartbeat),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CommandType {
    SystemInfo,
    Reboot,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandRequest {
    pub request_id: Uuid,
    pub command: CommandType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandResponse {
    pub request_id: Uuid,
    pub status: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Heartbeat {
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub username: String,
    pub os_version: String,
    pub uptime: String,
    pub cpu_info: String,
    pub memory_info: MemoryInfo,
    pub ip_addresses: Vec<String>,
    pub processes: Vec<ProcessInfo>,
    pub domain_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryInfo {
    pub total_memory: u64,
    pub used_memory: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use serde_json::json;

    #[test]
    fn test_message_serialization() {
        let req = Message::Command(CommandRequest {
            request_id: Uuid::new_v4(),
            command: CommandType::SystemInfo,
            payload: json!({}),
        });

        let json = serde_json::to_string(&req).unwrap();
        let de: Message = serde_json::from_str(&json).unwrap();

        if let Message::Command(c) = de {
            assert_eq!(c.command, CommandType::SystemInfo);
        } else {
            panic!("Wrong message type");
        }
    }
}
