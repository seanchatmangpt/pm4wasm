/**
 * Tenant management routes
 */

import { Router } from 'express';
import { body, param, validationResult } from 'express-validator';
import { tenantService } from './service.js';
import { authenticate, requireTenantMembership } from '../auth/middleware.js';
import { incrementUsage } from '../db/prisma.js';

export const tenantsRouter = Router();

/**
 * GET /tenants
 * List user's tenants
 */
tenantsRouter.get('/', authenticate, async (req, res, next) => {
  try {
    const tenants = await tenantService.listUserTenants(req.user!.id);
    res.json({ tenants });
  } catch (error) {
    next(error);
  }
});

/**
 * POST /tenants
 * Create new tenant
 */
tenantsRouter.post('/',
  authenticate,
  [
    body('name').trim().isLength({ min: 1, max: 100 }),
    body('plan').optional().isIn(['FREE', 'PRO', 'ENTERPRISE']),
  ],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      // Track API call
      if (req.user?.tenantId) {
        await incrementUsage(req.user.tenantId, 'apiCalls');
      }

      const tenant = await tenantService.createTenant(req.user!.id, req.body);
      res.status(201).json(tenant);
    } catch (error) {
      if (error instanceof Error && error.message === 'TENANT_LIMIT_REACHED') {
        return res.status(409).json({ code: 'TENANT_LIMIT_REACHED', message: 'Tenant limit reached' });
      }
      next(error);
    }
  }
);

/**
 * GET /tenants/:tenantId
 * Get tenant details
 */
tenantsRouter.get('/:tenantId',
  authenticate,
  [param('tenantId').isUUID()],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const tenant = await tenantService.getTenant(req.params.tenantId, req.user!.id);
      res.json(tenant);
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Not a member of this tenant' });
      }
      next(error);
    }
  }
);

/**
 * PUT /tenants/:tenantId
 * Update tenant settings
 */
tenantsRouter.put('/:tenantId',
  authenticate,
  [
    param('tenantId').isUUID(),
    body('name').optional().trim().isLength({ min: 1, max: 100 }),
  ],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const tenant = await tenantService.updateTenant(req.params.tenantId, req.user!.id, req.body);
      res.json(tenant);
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Insufficient permissions' });
      }
      next(error);
    }
  }
);

/**
 * DELETE /tenants/:tenantId
 * Delete tenant
 */
tenantsRouter.delete('/:tenantId',
  authenticate,
  [param('tenantId').isUUID()],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      await tenantService.deleteTenant(req.params.tenantId, req.user!.id);
      res.status(204).send();
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Only owners can delete tenants' });
      }
      next(error);
    }
  }
);

/**
 * GET /tenants/:tenantId/members
 * List tenant members
 */
tenantsRouter.get('/:tenantId/members',
  authenticate,
  [param('tenantId').isUUID()],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const members = await tenantService.listMembers(req.params.tenantId, req.user!.id);
      res.json({ members });
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Not a member of this tenant' });
      }
      next(error);
    }
  }
);

/**
 * POST /tenants/:tenantId/members
 * Add a member to the tenant
 */
tenantsRouter.post('/:tenantId/members',
  authenticate,
  [
    param('tenantId').isUUID(),
    body('email').isEmail(),
    body('role').isIn(['OWNER', 'ADMIN', 'EDITOR', 'VIEWER']),
  ],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const member = await tenantService.addMember(req.params.tenantId, req.user!.id, req.body);
      res.status(201).json(member);
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Insufficient permissions' });
      }
      if (error instanceof Error && error.message === 'ALREADY_MEMBER') {
        return res.status(409).json({ code: 'ALREADY_MEMBER', message: 'User is already a member' });
      }
      if (error instanceof Error && error.message === 'MEMBER_LIMIT_REACHED') {
        return res.status(409).json({ code: 'MEMBER_LIMIT_REACHED', message: 'Member limit reached for this plan' });
      }
      next(error);
    }
  }
);

/**
 * DELETE /tenants/:tenantId/members/:memberId
 * Remove a member
 */
tenantsRouter.delete('/:tenantId/members/:memberId',
  authenticate,
  [param('tenantId').isUUID(), param('memberId').isUUID()],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      await tenantService.removeMember(req.params.tenantId, req.user!.id, req.params.memberId);
      res.status(204).send();
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Insufficient permissions' });
      }
      if (error instanceof Error && error.message === 'CANNOT_REMOVE_OWNER') {
        return res.status(400).json({ code: 'CANNOT_REMOVE_OWNER', message: 'Cannot remove the tenant owner' });
      }
      next(error);
    }
  }
);

/**
 * GET /tenants/:tenantId/usage
 * Get tenant usage
 */
tenantsRouter.get('/:tenantId/usage',
  authenticate,
  [param('tenantId').isUUID()],
  async (req, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const usage = await tenantService.getUsage(req.params.tenantId, req.user!.id);
      res.json(usage);
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Not a member of this tenant' });
      }
      next(error);
    }
  }
);
