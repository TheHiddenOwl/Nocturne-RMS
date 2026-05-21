use async_trait::async_trait;
use protocol::models::{CommandRequest, SystemInfo, MemoryInfo, ProcessInfo};
use serde_json::{json, Value};
use sysinfo::{System, Networks, Components, Disks, Users, ProcessesToUpdate};
use crate::commands::Command;

pub struct SystemInfoCommand;

#[async_trait]
impl Command for SystemInfoCommand {
    fn name(&self) -> &str {
        "systeminfo"
    }

    async fn execute(&self, _payload: Value) -> anyhow::Result<Value> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let hostname = System::host_name().unwrap_or_default();
        let username = "unknown".to_string(); // In a real app, use whoami or similar
        let os_version = System::long_os_version().unwrap_or_default();

        let uptime_secs = System::uptime();
        let uptime = format!("{} days, {} hours, {} minutes",
            uptime_secs / 86400,
            (uptime_secs % 86400) / 3600,
            (uptime_secs % 3600) / 60);

        let cpu_info = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default();

        let memory_info = MemoryInfo {
            total_memory: sys.total_memory(),
            used_memory: sys.used_memory(),
        };

        let mut ip_addresses = Vec::new();
        let networks = Networks::new_with_refreshed_list();
        for (_, data) in &networks {
            for ip in data.ip_networks() {
                ip_addresses.push(ip.addr.to_string());
            }
        }

        let processes = sys.processes().iter().map(|(pid, process)| {
            ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
            }
        }).collect();

        let info = SystemInfo {
            hostname,
            username,
            os_version,
            uptime,
            cpu_info,
            memory_info,
            ip_addresses,
            processes,
            domain_name: "unknown".to_string(), // Can be retrieved with windows-specific calls
        };

        Ok(serde_json::to_value(info)?)
    }
}
