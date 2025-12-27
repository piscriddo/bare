# Deployment Guide - Polymarket HFT Bot

## Quick Start

### Local Development
```bash
# Run tests
cargo test

# Run benchmarks
cargo bench

# Run examples
cargo run --example live_arbitrage_detector
```

### Docker Deployment

**Build and run:**
```bash
# Build Docker image
docker build -t polymarket-hft-bot .

# Run with environment variables
docker run -d \
  -e BOT__DRY_RUN=true \
  -e BOT__WALLET__PRIVATE_KEY=0x... \
  -e RUST_LOG=info \
  --name hft-bot \
  polymarket-hft-bot
```

**Using Docker Compose:**
```bash
# Create .env file first
cp .env.example .env
# Edit .env with your credentials

# Start bot
docker-compose up -d

# View logs
docker-compose logs -f hft-bot

# Stop bot
docker-compose down
```

### Docker Compose with Monitoring

```bash
# Start bot + Prometheus + Grafana
docker-compose --profile monitoring up -d

# Access Grafana dashboard
open http://localhost:3000
# Default credentials: admin/admin
```

## Environment Variables

### Required
- `BOT__WALLET__PRIVATE_KEY` - Your Ethereum private key
- `BOT__WALLET__ADDRESS` - Your Ethereum address

### Optional
- `BOT__DRY_RUN` - Set to `false` for live trading (default: `true`)
- `RUST_LOG` - Logging level (default: `info`)
- `BOT__POLYMARKET__CLOB_API_URL` - CLOB API endpoint
- `BOT__POLYMARKET__WS_URL` - WebSocket URL

## Production Deployment

### 1. Colocation Setup

**Recommended for minimal latency (<1ms):**

```bash
# Rent server in same datacenter as Polymarket
# AWS us-east-1 or similar

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# Clone repository
git clone https://github.com/your-repo/polymarket-hft-bot.git
cd polymarket-hft-bot

# Configure environment
vim .env

# Deploy
docker-compose up -d
```

### 2. Kubernetes Deployment

**For high availability:**

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: polymarket-hft-bot
spec:
  replicas: 1
  selector:
    matchLabels:
      app: hft-bot
  template:
    metadata:
      labels:
        app: hft-bot
    spec:
      containers:
      - name: hft-bot
        image: polymarket-hft-bot:latest
        env:
        - name: BOT__DRY_RUN
          value: "false"
        - name: BOT__WALLET__PRIVATE_KEY
          valueFrom:
            secretKeyRef:
              name: bot-secrets
              key: private-key
        resources:
          limits:
            cpu: "2"
            memory: "512Mi"
          requests:
            cpu: "1"
            memory: "256Mi"
```

### 3. Security Best Practices

**Secrets Management:**
```bash
# Use environment variables, never commit secrets
# Use Docker secrets or Kubernetes secrets

# Create secret
kubectl create secret generic bot-secrets \
  --from-literal=private-key=0x...

# Use sealed secrets for GitOps
kubeseal --format yaml < secret.yaml > sealed-secret.yaml
```

**Network Security:**
```bash
# Restrict network access
# Only allow outbound to Polymarket APIs
# Block all inbound except monitoring

# Example iptables rules
iptables -A OUTPUT -d clob.polymarket.com -j ACCEPT
iptables -A OUTPUT -d ws-subscriptions-clob.polymarket.com -j ACCEPT
iptables -A OUTPUT -j DROP
```

## Monitoring

### Prometheus Metrics

Access metrics at: `http://localhost:9090`

**Key metrics to monitor:**
- Arbitrage opportunities detected
- Order execution latency
- Circuit breaker trips
- WebSocket connection status
- Daily P&L

### Grafana Dashboards

Access dashboard at: `http://localhost:3000`

**Pre-configured dashboards:**
- Trading Performance
- System Metrics (CPU, Memory, Network)
- Error Rates
- Order Book Statistics

### Alerting

**Example Prometheus alerts:**
```yaml
groups:
  - name: hft_bot
    rules:
    - alert: HighErrorRate
      expr: rate(errors_total[5m]) > 0.05
      for: 5m
      annotations:
        summary: "High error rate detected"

    - alert: CircuitBreakerTripped
      expr: circuit_breaker_state == 1
      annotations:
        summary: "Circuit breaker has been tripped"

    - alert: WebSocketDisconnected
      expr: websocket_connected == 0
      for: 1m
      annotations:
        summary: "WebSocket disconnected for >1min"
```

## Performance Tuning

### CPU Affinity (Linux)

```bash
# Pin to specific CPUs for lower latency
docker run -d \
  --cpuset-cpus="0,1" \
  --cpu-quota=200000 \
  polymarket-hft-bot
```

### Memory Limits

```bash
# Prevent swap usage
docker run -d \
  --memory=512m \
  --memory-swap=512m \
  polymarket-hft-bot
```

### Network Optimization

```bash
# Use host network for lowest latency
docker run -d \
  --network=host \
  polymarket-hft-bot
```

## Troubleshooting

### Common Issues

**1. WebSocket connection fails:**
```bash
# Check network connectivity
curl -v https://ws-subscriptions-clob.polymarket.com

# Check TLS certificates
openssl s_client -connect ws-subscriptions-clob.polymarket.com:443
```

**2. High latency:**
```bash
# Measure ping to Polymarket
ping clob.polymarket.com

# Check for network congestion
iftop

# Monitor CPU usage
htop
```

**3. Out of memory:**
```bash
# Check container memory
docker stats hft-bot

# Increase memory limit
docker update --memory=1g hft-bot
```

### Debug Mode

```bash
# Run with debug logging
docker run -d \
  -e RUST_LOG=debug \
  -e RUST_BACKTRACE=full \
  polymarket-hft-bot

# View logs
docker logs -f hft-bot
```

## Backup & Recovery

### Database Backup

```bash
# Backup trading history (if using persistent storage)
docker exec hft-bot tar czf /tmp/backup.tar.gz /data

# Copy backup
docker cp hft-bot:/tmp/backup.tar.gz ./backup-$(date +%Y%m%d).tar.gz
```

### Disaster Recovery

```bash
# Stop bot
docker-compose down

# Restore from backup
docker cp backup-20250127.tar.gz hft-bot:/tmp/

# Restart
docker-compose up -d
```

## Upgrade Procedure

```bash
# 1. Pull latest code
git pull origin main

# 2. Build new image
docker build -t polymarket-hft-bot:latest .

# 3. Stop old container
docker-compose down

# 4. Start new container
docker-compose up -d

# 5. Verify
docker-compose logs -f hft-bot
```

## License

MIT
