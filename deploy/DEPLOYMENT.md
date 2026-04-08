# PM4Wasm SaaS – Production Deployment Guide

## Prerequisites

1. **Kubernetes Cluster** (v1.25+)
   - AWS EKS, GKE, or Azure AKS
   - Minimum: 3 nodes, each with 4 vCPU, 16GB RAM
   - Recommended: 6 nodes for high availability

2. **Tools Installed**
   - kubectl v1.25+
   - helm v3.0+
   - kustomize v4.0+ (optional)

3. **External Services**
   - Domain name with DNS configured
   - SSL/TLS certificates (or use cert-manager)
   - Stripe account (for billing)
   - Google OAuth app (for SSO)
   - Sentry account (optional, for error tracking)

## Quick Start

### 1. Create Namespace and Secrets

```bash
# Create namespace
kubectl create namespace pm4wasm

# Create secrets from environment file
cat > .env.prod << EOF
POSTGRES_PASSWORD=your_secure_password_here
JWT_SECRET=your_256_bit_random_secret_here
JWT_REFRESH_SECRET=another_256_bit_random_secret_here
GOOGLE_OAUTH_CLIENT_ID=your_client_id.apps.googleusercontent.com
GOOGLE_OAUTH_CLIENT_SECRET=your_client_secret_here
STRIPE_SECRET_KEY=sk_live_your_key_here
STRIPE_WEBHOOK_SECRET=whsec_your_webhook_secret_here
ENCRYPTION_KEY=your_32_byte_hex_encryption_key
GRAFANA_ADMIN_USER=admin
GRAFANA_ADMIN_PASSWORD=your_secure_grafana_password
EOF

kubectl create secret generic pm4wasm-secrets \
  --from-env-file=.env.prod \
  -n pm4wasm
```

### 2. Deploy with Kustomize

```bash
# Deploy all resources
cd pm4wasm/deploy/k8s
kubectl apply -k .

# Verify deployment
kubectl get pods -n pm4wasm
kubectl get services -n pm4wasm
```

### 3. Deploy with Helm

```bash
# Add Helm chart (if published)
helm repo add pm4py https://charts.pm4py.org
helm repo update

# Install
helm install pm4wasm pm4py/pm4wasm \
  --namespace pm4wasm \
  --create-namespace \
  --values values-production.yaml
```

### 4. Configure DNS

Add A records pointing to your load balancer:

```
api.pm4py.org        → LB_IP
saas.pm4py.org       → LB_IP
grafana.pm4py.org    → LB_IP
```

### 5. Run Database Migrations

```bash
# Port-forward to API
kubectl port-forward -n pm4wasm deployment/pm4wasm-api 3000:3000

# Run migrations
cd pm4wasm/server
npx prisma migrate deploy
```

## Scaling

### Horizontal Scaling

```bash
# Scale API deployment
kubectl scale deployment/pm4wasm-api --replicas=10 -n pm4wasm

# Or edit HPA
kubectl edit hpa/pm4wasm-api-hpa -n pm4wasm
```

### Vertical Scaling

Edit deployment and adjust resource requests/limits:

```bash
kubectl edit deployment/pm4wasm-api -n pm4wasm
```

## Monitoring

### Access Grafana

```bash
# Port-forward
kubectl port-forward -n pm4wasm service/grafana 3001:3000

# Open http://localhost:3001
# Login with admin credentials from secrets
```

### Access Prometheus

```bash
kubectl port-forward -n pm4wasm service/prometheus 9090:9090

# Open http://localhost:9090
```

## Backup and Restore

### Database Backup

```bash
# Backup to S3
kubectl exec -n pm4wasm statefulset/postgres -- pg_dump \
  -U pm4wasm pm4wasm | gzip > backup.sql.gz

# Or automated backup with cron
kubectl apply -f deploy/k8s/backup-cronjob.yaml
```

### Restore from Backup

```bash
# Restore from backup
gunzip -c backup.sql.gz | kubectl exec -i -n pm4wasm statefulset/postgres -- \
  psql -U pm4wasm pm4wasm
```

## Troubleshooting

### Check Pod Status

```bash
kubectl get pods -n pm4wasm
kubectl describe pod <pod-name> -n pm4wasm
kubectl logs <pod-name> -n pm4wasm --tail=100 -f
```

### Common Issues

**Pod pending**: Check node resources and image pull secrets
**CrashLoopBackOff**: Check logs for configuration errors
**502 errors**: Check service endpoints and pod readiness

## Upgrading

### Rolling Update

```bash
# Update image
kubectl set image deployment/pm4wasm-api \
  api=ghcr.io/pm4py/pm4wasm-api:v1.2.3 \
  -n pm4wasm

# Watch rollout
kubectl rollout status deployment/pm4wasm-api -n pm4wasm

# Rollback if needed
kubectl rollout undo deployment/pm4wasm-api -n pm4wasm
```

## Security Checklist

- [ ] Secrets stored in Kubernetes Secrets (not ConfigMaps)
- [ ] RBAC enabled and least-privilege applied
- [ ] Network policies configured
- [ ] Pod security policies enabled
- [ ] TLS enabled for all endpoints
- [ ] Database encrypted at rest
- [ ] API rate limiting configured
- [ ] Audit logging enabled
- [ ] Regular security scans configured
- [ ] Incident response plan documented
