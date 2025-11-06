# Kubernetes Integration Guide

This fork of the Elasticsearch MCP server includes built-in Kubernetes port-forwarding support, making it easy to connect to Elasticsearch clusters running in Kubernetes without manual port-forward management.

## Quick Start

1. **Set environment variables:**
```bash
export K8S_PORT_FORWARD=true
export ES_API_KEY=your_api_key
```

2. **Run the server:**
```bash
cargo run -- stdio
```

The server will automatically:
- Connect to the `logs-es-http` service in the `infra` namespace
- Forward port 9200 locally
- Monitor and restart the connection if it fails
- Set `ES_URL` to `http://localhost:9200`

## Configuration Reference

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `K8S_PORT_FORWARD` | `false` | Enable automatic port-forwarding |
| `K8S_NAMESPACE` | `infra` | Kubernetes namespace |
| `K8S_SERVICE` | `logs-es-http` | Service name |
| `K8S_LOCAL_PORT` | `9200` | Local port |
| `K8S_REMOTE_PORT` | `9200` | Remote port on service |

## Features

- **Automatic Port Selection**: Intelligently finds an available local port
  - Tries preferred port (9200) first
  - Falls back to nearby ports if occupied
  - Logs which port is being used
- **Automatic Connection Management**: No need to manually manage `kubectl port-forward`
- **Auto-Restart on Failure**: Connection automatically restarts if severed
- **Exponential Backoff**: Prevents rapid restart loops (1s to 30s)
- **Logging**: All connection events logged for debugging
- **Zero Configuration**: Works with default Kubernetes setup

## Example: Custom Namespace

```bash
export K8S_PORT_FORWARD=true
export K8S_NAMESPACE=production
export K8S_SERVICE=elasticsearch-master
export ES_API_KEY=your_api_key

cargo run -- stdio
```

## Example: Claude Desktop Config

```json
{
  "mcpServers": {
    "elasticsearch": {
      "command": "/path/to/elasticsearch-core-mcp-server",
      "args": ["stdio"],
      "env": {
        "K8S_PORT_FORWARD": "true",
        "ES_API_KEY": "your_api_key"
      }
    }
  }
}
```

## Requirements

- `kubectl` installed and in PATH
- Active Kubernetes context
- RBAC permissions for port-forwarding

## Troubleshooting

**Connection keeps restarting:**
- Check kubectl has access to the namespace
- Verify the service name exists: `kubectl get svc -n infra`
- Check logs for specific errors

**Port already in use:**
- The server automatically finds an available port
- Check logs to see which port was selected
- If needed, set `K8S_LOCAL_PORT` to a specific available port

**Authentication failures:**
- Verify `ES_API_KEY` or `ES_USERNAME`/`ES_PASSWORD` are correct
- Check Elasticsearch is accessible from the service

**Want to use a specific port:**
- Set `K8S_LOCAL_PORT` to your desired port
- If that port is unavailable, the system will automatically find a nearby port
- Check the logs to confirm which port is being used

## Implementation

See `K8S_PORT_FORWARD.md` for detailed implementation information and `src/k8s_port_forward.rs` for the source code.
