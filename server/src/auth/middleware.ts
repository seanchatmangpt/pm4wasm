/**
 * Authentication middleware for Express
 */

import type { Request, Response, NextFunction } from 'express';
import { verifyAccessToken, extractToken } from './jwt.js';

declare global {
  namespace Express {
    interface Request {
      user?: {
        id: string;
        email: string;
        name: string;
        tenantId?: string;
      };
    }
  }
}

/**
 * Authentication middleware - verifies JWT and sets req.user
 */
export function authenticate(req: Request, res: Response, next: NextFunction): void {
  try {
    const token = extractToken(req.headers.authorization);
    if (!token) {
      res.status(401).json({ code: 'UNAUTHORIZED', message: 'Missing or invalid authorization header' });
      return;
    }

    const payload = verifyAccessToken(token);
    req.user = {
      id: payload.sub,
      email: payload.email,
      name: payload.name,
      tenantId: payload.tenantId,
    };
    next();
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Authentication failed';
    const status = message === 'TOKEN_EXPIRED' ? 401 : 401;
    res.status(status).json({ code: message, message: 'Invalid or expired token' });
  }
}

/**
 * Optional authentication - attaches req.user if token present
 */
export function optionalAuthenticate(req: Request, res: Response, next: NextFunction): void {
  try {
    const token = extractToken(req.headers.authorization);
    if (token) {
      const payload = verifyAccessToken(token);
      req.user = {
        id: payload.sub,
        email: payload.email,
        name: payload.name,
        tenantId: payload.tenantId,
      };
    }
  } catch {
    // Ignore errors - this is optional auth
  }
  next();
}

/**
 * Require tenant context - user must have a tenantId in their token
 */
export function requireTenant(req: Request, res: Response, next: NextFunction): void {
  if (!req.user?.tenantId) {
    res.status(400).json({ code: 'NO_TENANT', message: 'Tenant context required' });
    return;
  }
  next();
}

/**
 * Check tenant membership - ensures user is member of the specified tenant
 */
export function requireTenantMembership(req: Request, res: Response, next: NextFunction): void {
  const tenantId = req.params.tenantId || req.query.tenantId;
  if (!tenantId) {
    res.status(400).json({ code: 'NO_TENANT', message: 'Tenant ID required' });
    return;
  }

  // User must be a member of the requested tenant
  // This is checked against the database in the route handler
  // We just verify the format here
  if (typeof tenantId !== 'string') {
    res.status(400).json({ code: 'INVALID_TENANT', message: 'Invalid tenant ID' });
    return;
  }

  req.tenantId = tenantId;
  next();
}

declare global {
  namespace Express {
    interface Request {
      tenantId?: string;
    }
  }
}
