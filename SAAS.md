# pm4wasm SaaS Platform

Browser-native process mining SaaS powered by WebAssembly. All process mining computation happens in the browser; the server handles authentication, tenant management, billing, and real-time collaboration.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Browser (Client)                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   WASM Core  │  │   SaaS SDK   │  │   UI Layer   │         │
│  │  (pm4wasm)   │  │  (auth/api)  │  │  (SvelteKit)  │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      SaaS Backend (Node.js)                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐    │
│  │   Auth   │  │  Tenants │  │ Billing  │  │ Collaboration │    │
│  │  (JWT)   │  │ (Prisma) │  │ (Stripe) │  │  (Socket.IO)  │    │
│  └──────────┘  └──────────┘  └──────────┘  └──────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Data Layer (PostgreSQL)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │    Users     │  │   Tenants    │  │   Sessions   │         │
│  │  + OAuth     │  │ + Members    │  │ + History    │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

## Key Value Proposition

**Browser-Native = Privacy First**
- Event logs never leave the client
- Process models discovered locally via WASM
- Conformance checking in the browser
- Enterprise wedge against Python/AGPL blockers

## Quick Start

### Local Development

```bash
# Start all services (PostgreSQL, Redis, API, Prometheus, Grafana)
cd pm4wasm
docker-compose up -d

# Run database migrations
docker-compose exec api npx prisma migrate dev

# View logs
docker-compose logs -f api

# Stop services
docker-compose down
```

### Using the SDK

```typescript
import { Pm4wasmSaaS } from '@pm4wasm/saas-sdk';

const sdk = new Pm4wasmSaaS({
  apiUrl: 'http://localhost:3000/v1',
});

// Login
await sdk.auth.login({ email: 'user@example.com', password: 'password' });

// Create a collaborative session
const session = await sdk.collab.create({
  tenantId: 'tenant-id',
  name: 'My Process Model',
  model: 'PO=(nodes={A, B, C}, order={A-->B, B-->C})',
});

// Join with real-time collaboration
await sdk.collab.join(session.id, {
  'user:joined': (user) => console.log('User joined:', user),
  'model:updated': (update) => console.log('Model updated:', update),
});
```

## API Documentation

See [openapi.yaml](./openapi.yaml) for complete API specification.

### Authentication Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/auth/register` | POST | Register new user |
| `/v1/auth/login` | POST | Login with email/password |
| `/v1/auth/oauth/google` | GET | Initiate Google OAuth flow |
| `/v1/auth/refresh` | POST | Refresh access token |
| `/v1/auth/logout` | POST | Logout and invalidate tokens |

### Tenant Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/tenants` | GET | List user's tenants |
| `/v1/tenants` | POST | Create new tenant |
| `/v1/tenants/:id` | GET | Get tenant details |
| `/v1/tenants/:id/members` | GET | List tenant members |
| `/v1/tenants/:id/members` | POST | Add member to tenant |
| `/v1/tenants/:id/usage` | GET | Get usage metrics |

### Collaboration Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/sessions` | GET | List collaborative sessions |
| `/v1/sessions` | POST | Create new session |
| `/v1/sessions/:id/join` | POST | Join session (returns WebSocket ticket) |
| `/v1/sessions/:id/history` | GET | Get session history for replay |

### Billing Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/billing/plans` | GET | List available plans |
| `/v1/billing/checkout` | POST | Create Stripe Checkout session |
| `/v1/billing/portal` | POST | Create Customer Portal session |

## Plans and Pricing

| Plan | Price | API Calls | Storage | Sessions | Users |
|------|-------|-----------|---------|----------|-------|
| Free | $0 | 1,000/month | 100 MB | 60 min/month | 1 |
| Pro | $29/month | 100,000/month | 10 GB | 1,000 min/month | 10 |
| Enterprise | Custom | Unlimited | Unlimited | Unlimited | Unlimited |

## Deployment

### Prerequisites

- Node.js 20+
- PostgreSQL 16+
- Redis 7+ (optional, for session storage)
- Stripe account (for billing)
- Google Cloud project (for OAuth)

### Production Deployment

1. **Set environment variables:**
   ```bash
   cp server/.env.example server/.env
   # Edit .env with production values
   ```

2. **Build Docker image:**
   ```bash
   docker build -t pm4wasm-saas:latest -f server/Dockerfile server
   ```

3. **Run database migrations:**
   ```bash
   npx prisma migrate deploy
   ```

4. **Start the server:**
   ```bash
   docker-compose up -d
   ```

### Monitoring

- **Prometheus:** http://localhost:9090
- **Grafana:** http://localhost:3001 (admin/admin)
- **Health Check:** http://localhost:3000/health
- **Metrics:** http://localhost:3000/metrics

## Security Considerations

- All API endpoints protected with JWT authentication
- Multi-tenant data isolation via tenant_id filtering
- Row-level security in PostgreSQL
- Rate limiting on all public endpoints
- Stripe webhook signature verification
- CORS configuration for allowed origins

## License

AGPL-3.0 — See [LICENSE](../LICENSE) for details.

## Support

- Documentation: https://docs.pm4py.org
- Issues: https://github.com/pm4py/pm4py-core/issues
- Community: https://discord.gg/pm4py
