# Nocturne Remote Management System (RMS)

Nocturne-RMS is a secure, high-performance remote management system specifically designed for Windows hosts. It provides a robust architecture for executing commands and monitoring system health via a centralized control server.

## Overview

The system consists of three main components:
- **Server**: An Axum-based HTTPS/WSS server that manages agent sessions and provides a REST API for command execution.
- **Client (Agent)**: A Windows-targeted background service that maintains a persistent connection to the server.
- **Protocol**: A shared library defining the strongly-typed JSON message schema used across the workspace.

## Features

- **Secure Transport**: All communications are encrypted via TLS (rustls) with automatic self-signed certificate generation for simplified deployment.
- **Strong Authentication**: API key validation via `X-API-KEY` headers for all endpoints and WebSocket handshakes.
- **Resilient Connectivity**: Agents implement an exponential backoff strategy with jitter to maintain connectivity under unstable network conditions.
- **Modular Commands**: Registry-based command system allowing for easy extension of management capabilities.
- **Graceful Shutdown**: Implements a 30-second message flushing period during disconnection to ensure critical responses and logs are delivered.

---

## Quick Start

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable)
- Windows or Linux (for the Client/Agent component)

### 1. Build the Workspace
Clone the repository and build all components:
```bash
cargo build --release
```

### 2. Configure and Run the Server
The server automatically generates TLS certificates in `certs/` if they do not exist.

1. Navigate to the server directory:
   ```bash
   cd server
   ```
2. Create `config.toml` (or use the provided `config.toml.example`):
   ```toml
   server_addr = "0.0.0.0:443"
   api_keys = ["your-secure-api-key"]
   ```
3. Run the server:
   ```bash
   cargo run --release
   ```

### 3. Configure and Run the Client
1. Navigate to the client directory:
   ```bash
   cd client
   ```
2. Create `config.toml`:
   ```toml
   server_url = "wss://<server-ip>:443/ws"
   api_key = "your-secure-api-key"
   agent_id = "windows-host-01"
   ```
3. Run the client:
   ```bash
   cargo run --release
   ```

---

## Configuration

### Server (`server/config.toml`)
| Key | Description | Default |
|-----|-------------|---------|
| `server_addr` | Socket address for the server to bind to | `0.0.0.0:443` |
| `api_keys` | Array of authorized API keys for agents and control API | `["test-api-key"]` |
| `cert_path` | Path to the TLS certificate file | `certs/cert.pem` |
| `key_path` | Path to the TLS private key file | `certs/key.pem` |
| `heartbeat_interval_secs` | Interval for server-initiated WebSocket heartbeats | `30` |

### Client (`client/config.toml`)
| Key | Description | Default |
|-----|-------------|---------|
| `server_url` | Full WSS URL of the Nocturne server | `wss://localhost:443/ws` |
| `api_key` | Authentication key matching the server's configuration | `test-api-key` |
| `agent_id` | Unique string identifying this host to the server | `agent-001` |

---

## API Documentation

### Authentication
Every request must include the `X-API-KEY` header.
WebSocket connections must also include the `X-AGENT-ID` header.

### Control API (REST)
Used by administrators to send commands to connected agents.

**Endpoint**: `POST /api/agents/:agent_id/command`

**Request Body**:
```json
{
  "command": "system_info",
  "payload": {}
}
```
*Supported Commands*: `system_info`, `reboot`. (Note: Command names in JSON are `snake_case`).

**Response (200 OK)**:
```json
{
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "ok",
  "data": {
    "hostname": "DESKTOP-R2D2",
    "username": "Admin",
    "os_version": "Windows 11 Pro",
    "uptime": "2 days, 4 hours, 12 minutes",
    "cpu_info": "12th Gen Intel(R) Core(TM) i7-12700H",
    "memory_info": {
      "total_memory": 34359738368,
      "used_memory": 12884901888
    },
    "ip_addresses": ["192.168.1.15"],
    "processes": [
      { "pid": 1234, "name": "explorer.exe" }
    ],
    "domain_name": "CORP.LOCAL"
  }
}
```

**Errors**:
- `401 Unauthorized`: Missing or invalid `X-API-KEY`.
- `404 Not Found`: Agent with specified `agent_id` is not currently connected.
- `504 Gateway Timeout`: Agent connected but failed to respond within 30 seconds.

### WebSocket Protocol (JSON)
All messages are wrapped in a typed envelope:
```json
{
  "type": "command | response | heartbeat",
  ...
}
```

#### Command Message (Server -> Client)
```json
{
  "type": "command",
  "request_id": "uuid",
  "command": "system_info | reboot",
  "payload": {}
}
```

#### Response Message (Client -> Server)
```json
{
  "type": "response",
  "request_id": "uuid",
  "status": "ok | error",
  "data": {}
}
```

---

## Technical Deep Dive

### Resilience & Transport
- **Reconnect Strategy**: The client uses exponential backoff: $delay = base\_delay \times 2^{attempts} + jitter$.
  - `base_delay`: 1 second.
  - `max_delay`: 60 seconds.
  - `jitter`: 0-1000 milliseconds.
- **Graceful Shutdown**: Both client and server implement a 30-second wait period upon receiving a shutdown signal. During this time, the message queue is flushed to ensure that any pending command responses are transmitted before the socket closes.

### Windows-Specific Implementation
- **Cross-Platform Implementation**:
  - Uses the `sysinfo` crate for cross-platform metrics.
  - On Windows, invokes `GetComputerNameExW` via `windows-sys` to retrieve the DNS domain name.
  - On Linux, uses `hostname -d` to retrieve the domain name.
  - Collects the top 50 processes by CPU usage to provide actionable telemetry.
- **Reboot Command**:
  - On Windows, triggers `shutdown /r /t 30` via `std::process::Command`.
  - On Linux, triggers `shutdown -r now`.
  - The client is designed to send the `ok` response back to the server *immediately before* executing the shutdown command to avoid connection loss during the response phase.

---

## Security Considerations

1. **Self-Signed Certificates**: By default, the server generates a self-signed certificate. While sufficient for internal testing, production deployments should use certificates from a trusted Certificate Authority (CA).
2. **Key Management**: Ensure `config.toml` files are protected with appropriate filesystem permissions.
3. **Transport Security**: The client is currently configured to skip certificate verification for ease of use with the auto-generated certificates. For high-security environments, this should be updated to verify against a known CA fingerprint.

---

## Troubleshooting

- **Connection Failures**:
  - Ensure the server is listening on the expected port (default 443).
  - Verify that the Windows Firewall on the server host allows inbound traffic on the configured port.
- **Authentication Errors**:
  - Confirm the `X-API-KEY` matches exactly between client and server configuration files.
- **Commands Timeouts**:
  - If the agent is under heavy load, it may take longer than 30 seconds to gather system info.
  - Check client-side logs for "WebSocket error" or "Connection closed" which may indicate network instability.
