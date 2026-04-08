/**
 * Authentication routes
 */

import { Router } from 'express';
import { body, validationResult } from 'express-validator';
import { authService } from './service.js';
import { authenticate } from './middleware.js';

export const authRouter = Router();

/**
 * POST /auth/register
 * Register a new user account
 */
authRouter.post('/register',
  [
    body('email').isEmail().normalizeEmail(),
    body('password').isLength({ min: 8 }),
    body('name').trim().isLength({ min: 1, max: 100 }),
  ],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const result = await authService.register(req.body);
      res.status(201).json(result);
    } catch (error) {
      if (error instanceof Error && error.message === 'USER_EXISTS') {
        return res.status(409).json({ code: 'USER_EXISTS', message: 'Email already registered' });
      }
      next(error);
    }
  }
);

/**
 * POST /auth/login
 * Login with email/password
 */
authRouter.post('/login',
  [
    body('email').isEmail().normalizeEmail(),
    body('password').notEmpty(),
  ],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const result = await authService.login(req.body);
      res.json(result);
    } catch (error) {
      if (error instanceof Error && error.message === 'INVALID_CREDENTIALS') {
        return res.status(401).json({ code: 'INVALID_CREDENTIALS', message: 'Invalid email or password' });
      }
      next(error);
    }
  }
);

/**
 * POST /auth/refresh
 * Refresh access token using refresh token
 */
authRouter.post('/refresh',
  [
    body('refreshToken').notEmpty().isString(),
  ],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const result = await authService.refresh(req.body.refreshToken);
      res.json(result);
    } catch (error) {
      if (error instanceof Error && error.message === 'TOKEN_EXPIRED') {
        return res.status(401).json({ code: 'TOKEN_EXPIRED', message: 'Refresh token expired' });
      }
      if (error instanceof Error && error.message === 'TOKEN_INVALID') {
        return res.status(401).json({ code: 'TOKEN_INVALID', message: 'Invalid refresh token' });
      }
      next(error);
    }
  }
);

/**
 * POST /auth/logout
 * Logout and invalidate refresh token
 */
authRouter.post('/logout', authenticate, async (req, res, next) => {
  try {
    const { refreshToken } = req.body;
    if (refreshToken) {
      await authService.logout(refreshToken);
    }
    res.status(204).send();
  } catch (error) {
    next(error);
  }
);

/**
 * GET /auth/me
 * Get current user info
 */
authRouter.get('/me', authenticate, async (req, res) => {
  res.json({
    id: req.user!.id,
    email: req.user!.email,
    name: req.user!.name,
    tenantId: req.user!.tenantId,
  });
});
