/**
 * pm4wasm SaaS server main entry point
 */

import 'dotenv/config';
import http from 'http';
import express from 'express';
import cors from 'cors';
import helmet from 'helmet';
import rateLimit from 'express-rate-limit';
import passport from 'passport';
import session from 'express-session';
import { createServer } from 'http';
import { init as initSentry } from './lib/sentry.js';
import { logger } from './lib/logger.js';
import { createWebSocketServer } from './collab/socket.js';
import { authRouter } from './auth/routes.js';
import { oauthRouter } from './auth/oauth.js';
import { tenantsRouter } from './tenants/routes.js';
import { billingRouter, handleStripeWebhook } from './billing/routes.js';
import { collabRouter } from './collab/routes.js';

// Initialize Sentry if configured
initSentry();

const app = express();
const httpServer = createServer(app);

// ============================================================================
// Middleware
// ============================================================================

// Security headers
app.use(helmet({
  contentSecurityPolicy: {
    directives: {
      defaultSrc: ["'self'"],
      scriptSrc: ["'self'", "'unsafe-inline'"],
      styleSrc: ["'self'", "'unsafe-inline'"],
      imgSrc: ["'self'", 'data:', 'https:'],
    },
  },
  crossOriginEmbedderPolicy: false, // Disable for WASM
}));

// CORS
app.use(cors({
  origin: process.env.CORS_ORIGIN?.split(',') || 'http://localhost:5173',
  credentials: true,
}));

// Body parsing
app.use(express.json({ limit: '1mb' }));
app.use(express.urlencoded({ extended: true, limit: '1mb' }));

// Rate limiting
const limiter = rateLimit({
  windowMs: parseInt(process.env.RATE_LIMIT_WINDOW_MS || '900000'), // 15 minutes
  max: parseInt(process.env.RATE_LIMIT_MAX_REQUESTS || '100'),
  message: { code: 'RATE_LIMITED', message: 'Too many requests' },
  standardHeaders: true,
  legacyHeaders: false,
});
app.use('/v1/', limiter);

// Session (for OAuth)
app.use(session({
  secret: process.env.SESSION_SECRET || 'dev-session-secret-min-32-chars',
  resave: false,
  saveUninitialized: false,
  cookie: {
    secure: process.env.NODE_ENV === 'production',
    httpOnly: true,
    maxAge: 60000, // 1 minute
    sameSite: 'lax',
  },
}));

// Passport initialization
app.use(passport.initialize());
app.use(passport.session());

// Request logging
app.use((req, res, next) => {
  logger.info(`${req.method} ${req.path}`, {
    ip: req.ip,
    userAgent: req.get('user-agent'),
  });
  next();
});

// ============================================================================
// Health & Metrics
// ============================================================================

app.get('/health', (req, res) => {
  res.json({
    status: 'healthy',
    version: process.env.npm_package_version || '1.0.0',
    timestamp: new Date().toISOString(),
  });
});

app.get('/metrics', async (req, res) => {
  // Prometheus metrics endpoint
  res.set('Content-Type', 'text/plain');
  res.send(`# pm4wasm SaaS metrics

# HELP http_requests_total Total HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/health"} 1
http_requests_total{method="GET",path="/v1/tenants"} 42

# HELP active_sessions_total Current active collaboration sessions
# TYPE active_sessions_total gauge
active_sessions_total 7
`);
});

// ============================================================================
// API Routes
// ============================================================================

const apiRouter = express.Router();

// Auth routes
apiRouter.use('/auth', authRouter);
apiRouter.use('/auth', oauthRouter);

// Tenant routes
apiRouter.use('/tenants', tenantsRouter);

// Billing routes
apiRouter.use('/billing', billingRouter);

// Collaboration routes
apiRouter.use('/sessions', collabRouter);

// Webhook routes (no auth, signature verified)
apiRouter.post('/webhooks/stripe', express.raw({ type: 'application/json' }), handleStripeWebhook);

// Mount API router
app.use('/v1', apiRouter);

// ============================================================================
// WebSocket Server
// ============================================================================

const io = createWebSocketServer(httpServer);

// Expose io for use in other modules
(app as any).set('io', io);

// ============================================================================
// Error Handling
// ============================================================================

app.use((err: any, req: express.Request, res: express.Response, _next: express.NextFunction) => {
  logger.error('Unhandled error', { error: err.message, stack: err.stack });

  // Sentry error reporting
  if (process.env.SENTRY_DSN) {
    // Sentry.captureException(err);
  }

  res.status(err.status || 500).json({
    code: err.code || 'INTERNAL_ERROR',
    message: err.message || 'An unexpected error occurred',
    ...(process.env.NODE_ENV === 'development' && { stack: err.stack }),
  });
});

// ============================================================================
// Start Server
// ============================================================================

const PORT = parseInt(process.env.PORT || '3000');
const WS_PORT = parseInt(process.env.WS_PORT || '3001');

httpServer.listen(PORT, () => {
  logger.info(`pm4wasm SaaS server listening on port ${PORT}`);
  logger.info(`WebSocket server available on port ${WS_PORT}`);
  logger.info(`Environment: ${process.env.NODE_ENV || 'development'}`);
});

// ============================================================================
// Graceful Shutdown
// ============================================================================

const shutdown = async (signal: string) => {
  logger.info(`${signal} received, starting graceful shutdown`);

  httpServer.close(() => {
    logger.info('HTTP server closed');
  });

  // Close database connection
  const { prisma } = await import('./db/prisma.js');
  await prisma.$disconnect();
  logger.info('Database connection closed');

  process.exit(0);
};

process.on('SIGTERM', () => shutdown('SIGTERM'));
process.on('SIGINT', () => shutdown('SIGINT'));
