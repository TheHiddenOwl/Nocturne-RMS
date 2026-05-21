# Nocturne-RMS

Secure remote management system for Windows hosts.

## Project Structure

- `server/`: HTTPS/WSS server implementation using Axum.
- `client/`: Windows-targeted client implementation.
- `protocol/`: Shared JSON message definitions.

## Build Instructions

1. Ensure you have Rust and Cargo installed.
2. Build the workspace:
   ```bash
   cargo build
   ```

## Configuration

### Server Configuration
The server expects a configuration for API keys and TLS paths. Default values are:
- Port: 443
- API Keys: `["test-api-key"]`
- TLS: Auto-generates `certs/cert.pem` and `certs/key.pem` if they don't exist.

### Client Configuration
The client requires:
- Server URL (e.g., `wss://localhost:443/ws`)
- API Key
- Agent ID

## Features

- **TLS**: Automatic self-signed certificate generation.
- **Authentication**: API key validation via HTTP headers.
- **Commands**:
  - `systeminfo`: Returns hostname, CPU, RAM, and process list.
  - `reboot`: Safely reboots the Windows host (30s delay).
- **Resilience**: Client features exponential backoff reconnection.
