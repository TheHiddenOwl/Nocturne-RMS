use async_trait::async_trait;
use protocol::models::{SystemInfo, MemoryInfo, ProcessInfo, CommandType};
use serde_json::Value;
use sysinfo::{System, Networks};
use crate::commands::Command;

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::SystemInformation::{GetComputerNameExW, ComputerNameDnsDomain};

pub struct SystemInfoCommand;

#[async_trait]
impl Command for SystemInfoCommand {
    fn command_type(&self) -> CommandType {
        CommandType::SystemInfo
    }

    async fn execute(&self, _payload: Value) -> anyhow::Result<Value> {
        let mut sys = System::new_all();

        // First refresh for CPU baseline
        sys.refresh_all();
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        // Second refresh to get CPU usage
        sys.refresh_all();

        let hostname = System::host_name().unwrap_or_default();

        #[cfg(target_os = "windows")]
        let username = std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string());
        #[cfg(not(target_os = "windows"))]
        let username = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

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

        let mut processes: Vec<ProcessInfo> = sys.processes().iter().map(|(pid, process)| {
            ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
            }
        }).collect();

        // Sort by CPU usage descending and take top 50
        let mut process_cpu: Vec<(u32, f32)> = sys.processes().iter()
            .map(|(pid, p)| (pid.as_u32(), p.cpu_usage()))
            .collect();
        process_cpu.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_pids: std::collections::HashSet<u32> = process_cpu.iter()
            .take(50)
            .map(|(pid, _)| *pid)
            .collect();

        processes.retain(|p| top_pids.contains(&p.pid));
        // Also sort the final list by CPU usage for convenience
        let pid_to_cpu: std::collections::HashMap<u32, f32> = process_cpu.into_iter().collect();
        processes.sort_by(|a, b| {
            let cpu_a = pid_to_cpu.get(&a.pid).unwrap_or(&0.0);
            let cpu_b = pid_to_cpu.get(&b.pid).unwrap_or(&0.0);
            cpu_b.partial_cmp(cpu_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        #[allow(unused_mut)]
        let mut domain_name = "unknown".to_string();
        #[cfg(target_os = "windows")]
        {
            let mut buffer = [0u16; 256];
            let mut size = buffer.len() as u32;
            unsafe {
                if GetComputerNameExW(ComputerNameDnsDomain, buffer.as_mut_ptr(), &mut size) != 0 {
                    domain_name = String::from_utf16_lossy(&buffer[..size as usize]);
                }
            }
        }

        let info = SystemInfo {
            hostname,
            username,
            os_version,
            uptime,
            cpu_info,
            memory_info,
            ip_addresses,
            processes,
            domain_name,
        };

        Ok(serde_json::to_value(info)?)
    }
}
