# Kubernetes Port-Forward Support

This fork adds automatic Kubernetes port-forwarding functionality to simplify connecting to Elasticsearch clusters running in Kubernetes.

## Overview

When enabled, the MCP server will automatically:
- Start a `kubectl port-forward` to your Elasticsearch service
- Monitor the connection and automatically restart it if it fails
- Use exponential backoff for retries (1s to 30s maximum)
- Set the `ES_URL` automatically if not already configured

## Configuration

Enable port-forwarding using environment variables:

### Required
- `K8S_PORT_FORWARD=true` - Enable automatic port-forwarding

### Optional (with defaults)
- `K8S_NAMESPACE=infra` - Kubernetes namespace (default: infra)
- `K8S_SERVICE=logs-es-http` - Service name (default: logs-es-http)
- `K8S_LOCAL_PORT=9200` - Local port for forwarding (default: 9200)
- `K8S_REMOTE_PORT=9200` - Remote port on the service (default: 9200)

## Example Usage

### Basic Usage
```bash
export K8S_PORT_FORWARD=true
export ES_API_KEY=your_api_key
cargo run -- stdio
```

### Custom Configuration
```bash
export K8S_PORT_FORWARD=true
export K8S_NAMESPACE=production
export K8S_SERVICE=elasticsearch-http
export K8S_LOCAL_PORT=9201
export K8S_REMOTE_PORT=9200
export ES_API_KEY=your_api_key
cargo run -- stdio
```

### With Claude Desktop

Add to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "elasticsearch-mcp-server": {
      "command": "/path/to/elasticsearch-core-mcp-server",
      "args": ["stdio"],
      "env": {
        "K8S_PORT_FORWARD": "true",
        "K8S_NAMESPACE": "infra",
        "K8S_SERVICE": "logs-es-http",
        "ES_API_KEY": "<elasticsearch-API-key>"
      }
    }
  }
}
```

## How It Works

1. When `K8S_PORT_FORWARD=true`, the server spawns a background task
2. **Port availability check**: Automatically finds an available local port
   - First tries the preferred port (default: 9200)
   - If unavailable, tries nearby ports (9201-9210, 9199-9190)
   - Falls back to high ports (19200-19299) if needed
   - Last resort: lets the OS assign a random available port
3. The task runs `kubectl port-forward -n <namespace> svc/<service> <local>:<remote>`
4. The process output is monitored via stdout/stderr
5. If the connection fails or the process exits, it automatically restarts
6. Exponential backoff prevents rapid restart loops (1s → 2s → 4s → ... → 30s max)
7. If `ES_URL` is not set, it's automatically set to `http://localhost:<local_port>`

## Requirements

- `kubectl` must be installed and available in PATH
- Active Kubernetes context with access to the target namespace
- Appropriate RBAC permissions to port-forward to the service

## Logging

Port-forward events are logged at INFO level:
- Port availability checks and automatic port selection
- Connection start/restart attempts
- kubectl stdout/stderr output
- Connection failures and retry delays

Example log output:
```
Port 9200 not available, using port 9201 instead
Starting port-forward: kubectl port-forward -n infra svc/logs-es-http 9201:9200
Setting ES_URL to http://localhost:9201 for port-forwarding
```

## Implementation Details

See `src/k8s_port_forward.rs` for the implementation:
- `PortForwardConfig`: Configuration structure
- `find_available_port()`: Smart port selection algorithm
- `is_port_available()`: Port availability checker
- `start_port_forward()`: Launches the background monitoring task
- `run_port_forward()`: Manages a single kubectl port-forward instance
- Automatic cleanup via `kill_on_drop` on the child process

### Port Selection Strategy

1. **Preferred port**: Try the configured port (default 9200)
2. **Nearby ports**: Try ports within ±10 of preferred (e.g., 9201-9210, 9199-9190)
3. **High ports**: Try ports 19200-19299
4. **OS assignment**: Let the OS assign a random available port
5. **Fallback**: Use preferred port anyway (will fail but errors are logged)
