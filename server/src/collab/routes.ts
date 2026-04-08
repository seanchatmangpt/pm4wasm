/**
 * Collaboration routes
 */

import { Router } from 'express';
import { body, param, validationResult } from 'express-validator';
import { collabService } from './service.js';
import { authenticate } from '../auth/middleware.js';

export const collabRouter = Router();

/**
 * GET /sessions
 * List collaborative sessions
 */
collabRouter.get('/',
  authenticate,
  async (req, res, next) => {
    try {
      const tenantId = req.query.tenantId as string;
      if (!tenantId) {
        return res.status(400).json({ code: 'MISSING_TENANT', message: 'Tenant ID required' });
      }

      const sessions = await collabService.listSessions(tenantId, req.user!.id);
      res.json({ sessions });
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Not a member of this tenant' });
      }
      next(error);
    }
  }
);

/**
 * POST /sessions
 * Create a new collaborative session
 */
collabRouter.post('/',
  authenticate,
  [
    body('tenantId').isUUID(),
    body('name').trim().isLength({ min: 1, max: 200 }),
    body('model').isString(),
    body('isPublic').optional().isBoolean(),
  ],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const session = await collabService.createSession(
        req.body.tenantId,
        req.user!.id,
        {
          name: req.body.name,
          model: req.body.model,
          isPublic: req.body.isPublic,
        }
      );

      res.status(201).json({
        id: session.id,
        name: session.name,
        isPublic: session.isPublic,
        createdAt: session.createdAt,
      });
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Not a member of this tenant' });
      }
      next(error);
    }
  }
);

/**
 * GET /sessions/:sessionId
 * Get session details
 */
collabRouter.get('/:sessionId',
  authenticate,
  [param('sessionId').isUUID()],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const session = await collabService.getSession(req.params.sessionId, req.user?.id);
      res.json({
        id: session.id,
        name: session.name,
        model: session.model,
        isPublic: session.isPublic,
        createdAt: session.createdAt,
        updatedAt: session.updatedAt,
      });
    } catch (error) {
      if (error instanceof Error && error.message === 'SESSION_NOT_FOUND') {
        return res.status(404).json({ code: 'SESSION_NOT_FOUND', message: 'Session not found' });
      }
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Access denied' });
      }
      next(error);
    }
  }
);

/**
 * DELETE /sessions/:sessionId
 * Delete a session
 */
collabRouter.delete('/:sessionId',
  authenticate,
  [param('sessionId').isUUID()],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      await collabService.deleteSession(req.params.sessionId, req.user!.id);
      res.status(204).send();
    } catch (error) {
      if (error instanceof Error && error.message === 'SESSION_NOT_FOUND') {
        return res.status(404).json({ code: 'SESSION_NOT_FOUND', message: 'Session not found' });
      }
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Only session creator can delete' });
      }
      next(error);
    }
  }
);

/**
 * POST /sessions/:sessionId/join
 * Join a collaborative session (returns WebSocket ticket)
 */
collabRouter.post('/:sessionId/join',
  authenticate,
  [param('sessionId').isUUID()],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      // Verify access first
      await collabService.getSession(req.params.sessionId, req.user!.id);

      // Generate join ticket
      const ticket = collabService.generateJoinTicket(req.params.sessionId, req.user!.id);

      res.json({
        ticket,
        wsUrl: `ws://localhost:${process.env.WS_PORT || 3001}/socket.io/`,
        signalingUrl: `/socket.io/`,
      });
    } catch (error) {
      if (error instanceof Error && error.message === 'SESSION_NOT_FOUND') {
        return res.status(404).json({ code: 'SESSION_NOT_FOUND', message: 'Session not found' });
      }
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Access denied' });
      }
      next(error);
    }
  }
);

/**
 * GET /sessions/:sessionId/history
 * Get session history for replay
 */
collabRouter.get('/:sessionId/history',
  authenticate,
  [param('sessionId').isUUID()],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      // Verify access
      await collabService.getSession(req.params.sessionId, req.user!.id);

      const history = await collabService.getSessionHistory(req.params.sessionId);
      res.json({ history });
    } catch (error) {
      if (error instanceof Error && error.message === 'SESSION_NOT_FOUND') {
        return res.status(404).json({ code: 'SESSION_NOT_FOUND', message: 'Session not found' });
      }
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Access denied' });
      }
      next(error);
    }
  }
);
