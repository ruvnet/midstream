# AIMDS Deployment Guide

## Production Deployment

### Prerequisites

- Node.js 18+ (LTS recommended)
- npm 9+
- Docker (optional)
- Kubernetes (optional)

## Environment Setup

### 1. Environment Variables

Create `.env` file:

```bash
# Server Configuration
NODE_ENV=production
PORT=3000
HOST=0.0.0.0

# AgentDB Configuration
AGENTDB_PATH=./data/agentdb
EMBEDDING_DIM=384
HNSW_M=16
HNSW_EF_CONSTRUCTION=200
HNSW_EF_SEARCH=100

# QUIC Synchronization (optional)
QUIC_ENABLED=false
QUIC_PORT=4433
QUIC_PEERS=

# lean-agentic Configuration
LEAN_HASH_CONS=true
LEAN_DEPENDENT_TYPES=true
LEAN_THEOREM_PROVING=true
LEAN_CACHE_SIZE=10000
LEAN_PROOF_TIMEOUT=5000

# Security
RATE_LIMIT_WINDOW_MS=60000
RATE_LIMIT_MAX=100
ENABLE_CORS=true
ENABLE_COMPRESSION=true

# Monitoring
LOG_LEVEL=info
METRICS_ENABLED=true
```

### 2. Build and Start

```bash
# Install dependencies
npm install --production

# Build TypeScript
npm run build

# Start server
npm start
```

## Docker Deployment

### Build Docker Image

```bash
docker build -t aimds:latest .
```

### Run Container

```bash
docker run -d \
  --name aimds \
  -p 3000:3000 \
  -v $(pwd)/data:/app/data \
  -e NODE_ENV=production \
  aimds:latest
```

### Docker Compose

```bash
docker-compose up -d
```

## Kubernetes Deployment

### Apply Manifests

```bash
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
```

### Verify Deployment

```bash
kubectl get pods -n aimds
kubectl get svc -n aimds
```

## Health Checks

### Liveness Probe

```bash
GET /health
```

Expected response:
```json
{
  "status": "healthy",
  "timestamp": 1234567890,
  "components": {
    "gateway": { "status": "up" },
    "agentdb": { "status": "up" },
    "verifier": { "status": "up" }
  }
}
```

### Readiness Probe

Same as liveness probe, but checks if all components are ready to serve traffic.

## Monitoring

### Prometheus Metrics

Scrape endpoint: `http://localhost:3000/metrics`

Key metrics:
- `aimds_requests_total`
- `aimds_latency_ms`
- `aimds_threats_detected`
- `agentdb_vector_search_ms`
- `verifier_proof_time_ms`

### Grafana Dashboard

Import dashboard from `k8s/grafana-dashboard.json`

## Scaling

### Horizontal Scaling

```bash
kubectl scale deployment aimds --replicas=3 -n aimds
```

### Load Balancing

Use Kubernetes Service with LoadBalancer type or Nginx Ingress.

## Security Considerations

1. **Rate Limiting**: Configure appropriate limits based on traffic
2. **CORS**: Restrict origins in production
3. **TLS/SSL**: Use reverse proxy (Nginx/Traefik) for HTTPS
4. **Secrets Management**: Use Kubernetes Secrets or Vault
5. **Network Policies**: Restrict pod-to-pod communication

## Performance Tuning

### Node.js

```bash
NODE_OPTIONS="--max-old-space-size=4096 --max-http-header-size=16384"
```

### AgentDB

- Adjust HNSW parameters based on dataset size
- Enable QUIC sync for distributed deployments
- Configure memory limits and TTL

### Caching

- Increase proof certificate cache size for better hit rates
- Use Redis for distributed caching (future enhancement)

## Troubleshooting

### High Latency

1. Check AgentDB HNSW index configuration
2. Monitor vector search times
3. Review proof certificate cache hit rate

### Memory Issues

1. Check AgentDB memory usage
2. Adjust cache sizes
3. Review TTL settings

### Connection Errors

1. Verify QUIC port is open (if enabled)
2. Check network policies
3. Review firewall rules

## Backup and Recovery

### Database Backup

```bash
tar -czf agentdb-backup.tar.gz data/agentdb/
```

### Restore

```bash
tar -xzf agentdb-backup.tar.gz -C ./
```

## Upgrade Strategy

1. Deploy new version alongside old (blue-green)
2. Run smoke tests
3. Gradually shift traffic
4. Monitor metrics
5. Rollback if issues detected

## Support

For production issues, check:
- Logs: `kubectl logs -f deployment/aimds -n aimds`
- Metrics: Prometheus/Grafana dashboards
- Health: `/health` endpoint
