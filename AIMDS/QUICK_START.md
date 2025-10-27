# AIMDS Quick Start Guide

Get up and running with AIMDS in 5 minutes!

## 📋 Prerequisites

- Node.js 18+ (LTS)
- npm 9+
- 2GB RAM minimum

## 🚀 Installation

```bash
# Navigate to AIMDS directory
cd /workspaces/midstream/AIMDS

# Install dependencies
npm install

# Build TypeScript
npm run build
```

## ⚙️ Configuration

1. Copy environment template:
```bash
cp .env.example .env
```

2. Edit `.env` (optional - defaults work fine):
```env
PORT=3000
HOST=0.0.0.0
NODE_ENV=development
```

## 🎯 Start the Gateway

```bash
# Development mode (with hot reload)
npm run dev

# Production mode
npm start
```

Gateway will start at: `http://localhost:3000`

## 🧪 Test the API

### Health Check
```bash
curl http://localhost:3000/health
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

### Defense Endpoint
```bash
curl -X POST http://localhost:3000/api/v1/defend \
  -H "Content-Type: application/json" \
  -d '{
    "action": {
      "type": "read",
      "resource": "/api/users",
      "method": "GET"
    },
    "source": {
      "ip": "192.168.1.1"
    }
  }'
```

Expected response:
```json
{
  "requestId": "req_...",
  "allowed": true,
  "confidence": 0.95,
  "threatLevel": "LOW",
  "latency": 8.5,
  "metadata": {
    "pathTaken": "fast"
  }
}
```

### Metrics
```bash
curl http://localhost:3000/metrics
```

## 🔬 Run Tests

```bash
# All tests
npm test

# Unit tests only
npm run test:unit

# Integration tests only
npm run test:integration

# Performance benchmarks
npm run bench
```

## 📊 Monitor

- **Health**: `http://localhost:3000/health`
- **Metrics**: `http://localhost:3000/metrics`
- **Stats**: `http://localhost:3000/api/v1/stats`

## 🐳 Docker Quick Start

```bash
# Build image
docker build -t aimds:latest .

# Run container
docker run -d -p 3000:3000 aimds:latest

# Or use Docker Compose
docker-compose up -d
```

## 📚 Next Steps

1. Read the [Architecture Guide](ARCHITECTURE.md)
2. Check [API Documentation](docs/api/)
3. Review [Deployment Guide](DEPLOYMENT.md)
4. Explore [Examples](examples/)

## 🆘 Troubleshooting

### Port already in use
```bash
# Change port in .env
PORT=3001
```

### Module not found
```bash
# Clean install
rm -rf node_modules package-lock.json
npm install
npm run build
```

### Build errors
```bash
# Check Node version
node --version  # Should be 18+

# Clean build
rm -rf dist
npm run build
```

## 💡 Usage Examples

### TypeScript
```typescript
import { AIMDSGateway } from './src/gateway/server';

const gateway = new AIMDSGateway(
  gatewayConfig,
  agentdbConfig,
  leanAgenticConfig
);

await gateway.initialize();
await gateway.start();
```

See [examples/typescript/](examples/typescript/) for more.

## 📞 Support

- Documentation: `docs/`
- Issues: GitHub Issues
- Guides: `docs/guides/`

---

**Ready to deploy?** See [DEPLOYMENT.md](DEPLOYMENT.md)
