# Docker Usage Guide

The `elasticsearch-mcp-k8s:latest` image includes kubectl for automatic Kubernetes port-forwarding.

## Building the Image

```bash
docker build -f Dockerfile.k8s -t elasticsearch-mcp-k8s:latest .
```

## Running with Kubernetes Port-Forward

### Prerequisites
- Kubernetes config must be mounted into the container
- The container needs access to your kubectl config at `~/.kube/config`

### Basic Usage

```bash
docker run -i --rm \
  -v ~/.kube/config:/root/.kube/config:ro \
  -e K8S_PORT_FORWARD=true \
  -e ES_API_KEY=your_api_key \
  elasticsearch-mcp-k8s:latest \
  stdio
```

### With Custom Configuration

```bash
docker run -i --rm \
  -v ~/.kube/config:/root/.kube/config:ro \
  -e K8S_PORT_FORWARD=true \
  -e K8S_NAMESPACE=production \
  -e K8S_SERVICE=elasticsearch-http \
  -e ES_API_KEY=your_api_key \
  elasticsearch-mcp-k8s:latest \
  stdio
```

### HTTP Mode

```bash
docker run --rm \
  -v ~/.kube/config:/root/.kube/config:ro \
  -p 8080:8080 \
  -e K8S_PORT_FORWARD=true \
  -e ES_API_KEY=your_api_key \
  elasticsearch-mcp-k8s:latest \
  http
```

## Claude Desktop Configuration

### Using the Container

```json
{
  "mcpServers": {
    "elasticsearch": {
      "command": "docker",
      "args": [
        "run", "-i", "--rm",
        "-v", "/home/your-user/.kube/config:/root/.kube/config:ro",
        "-e", "K8S_PORT_FORWARD=true",
        "-e", "ES_API_KEY",
        "elasticsearch-mcp-k8s:latest",
        "stdio"
      ],
      "env": {
        "ES_API_KEY": "your_api_key"
      }
    }
  }
}
```

## Environment Variables

All standard environment variables work in the container:

| Variable | Description | Default |
|----------|-------------|---------|
| `K8S_PORT_FORWARD` | Enable port-forwarding | `false` |
| `K8S_NAMESPACE` | Kubernetes namespace | `infra` |
| `K8S_SERVICE` | Service name | `logs-es-http` |
| `K8S_LOCAL_PORT` | Local port | `9200` |
| `K8S_REMOTE_PORT` | Remote port | `9200` |
| `ES_API_KEY` | Elasticsearch API key | - |
| `ES_USERNAME` | Elasticsearch username | - |
| `ES_PASSWORD` | Elasticsearch password | - |
| `ES_SSL_SKIP_VERIFY` | Skip SSL verification | `false` |

## Network Modes

### Host Network (for port-forward access)

If you need the port-forward to be accessible on the host:

```bash
docker run -i --rm \
  --network host \
  -v ~/.kube/config:/root/.kube/config:ro \
  -e K8S_PORT_FORWARD=true \
  -e ES_API_KEY=your_api_key \
  elasticsearch-mcp-k8s:latest \
  stdio
```

## Troubleshooting

**kubectl: command not found**
- You're using the wrong Dockerfile. Use `Dockerfile.k8s`, not `Dockerfile`

**Unable to connect to the server**
- Verify `~/.kube/config` is mounted correctly
- Check the Kubernetes context: `docker run --rm -v ~/.kube/config:/root/.kube/config:ro elasticsearch-mcp-k8s:latest kubectl config current-context`

**Permission denied on kubeconfig**
- Ensure kubeconfig is readable: `chmod 644 ~/.kube/config`

**Port-forward not working**
- Check container logs: Add `--name es-mcp` and use `docker logs es-mcp`
- Verify RBAC permissions for port-forwarding

## Image Details

- **Base image**: Debian Bookworm Slim
- **Includes**: kubectl (latest stable)
- **Size**: ~151MB
- **Architecture**: linux/amd64 (modify Dockerfile.k8s for multi-arch)
