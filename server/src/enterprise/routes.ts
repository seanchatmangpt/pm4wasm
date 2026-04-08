/**
 * PM4Wasm SaaS – Enterprise Routes
 * Copyright (C) 2026 Process Intelligence Solutions GmbH
 *
 * REST API endpoints for RBAC, Audit Logs, and SSO.
 */

import { Router } from 'express';
import { RBACService, Resource, Action } from './rbac';
import { AuditService, AuditEventType } from './audit';
import { SSOService, SSOProtocol, SSOProvider, SSODirection } from './sso';

export function createEnterpriseRoutes(
  rbac: RBACService,
  audit: AuditService,
  sso: SSOService
): Router {
  const router = Router();

  // ============== RBAC Routes ==============

  /**
   * GET /enterprise/roles
   * List all roles for tenant
   */
  router.get('/roles', async (req, res) => {
    try {
      const tenantId = req.tenantId;
      const roles = await rbac.getTenantRoles(tenantId);
      res.json({ roles });
    } catch (error) {
      res.status(500).json({ error: error.message });
    }
  });

  /**
   * POST /enterprise/roles
   * Create custom role
   */
  router.post(
    '/roles',
    rbac.requirePermission(Resource.TENANT, Action.UPDATE),
    async (req, res) => {
      try {
        const tenantId = req.tenantId;
        const { name, description, permissions } = req.body;

        const role = await rbac.createRole(tenantId, name, description, permissions);

        await audit.logResourceChange(
          AuditEventType.ROLE_CREATED,
          req.user.id,
          tenantId,
          { type: 'role', id: role.id, name: role.name }
        );

        res.json({ role });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * PUT /enterprise/roles/:roleId
   * Update role
   */
  router.put(
    '/roles/:roleId',
    rbac.requirePermission(Resource.TENANT, Action.UPDATE),
    async (req, res) => {
      try {
        const { roleId } = req.params;
        const { name, description, permissions } = req.body;

        const role = await rbac.updateRole(roleId, {
          name,
          description,
          permissions,
        });

        await audit.logResourceChange(
          AuditEventType.ROLE_UPDATED,
          req.user.id,
          req.tenantId,
          { type: 'role', id: role.id, name: role.name },
          { newValue: { name, description, permissions } }
        );

        res.json({ role });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * DELETE /enterprise/roles/:roleId
   * Delete role
   */
  router.delete(
    '/roles/:roleId',
    rbac.requirePermission(Resource.TENANT, Action.UPDATE),
    async (req, res) => {
      try {
        const { roleId } = req.params;
        await rbac.deleteRole(roleId);

        await audit.logResourceChange(
          AuditEventType.ROLE_DELETED,
          req.user.id,
          req.tenantId,
          { type: 'role', id: roleId }
        );

        res.json({ success: true });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * PUT /enterprise/members/:userId/role
   * Assign role to user
   */
  router.put(
    '/members/:userId/role',
    rbac.requirePermission(Resource.USER, Action.UPDATE),
    async (req, res) => {
      try {
        const { userId } = req.params;
        const { roleId } = req.body;
        const tenantId = req.tenantId;

        await rbac.assignRole(userId, tenantId, roleId);

        await audit.logResourceChange(
          AuditEventType.ROLE_ASSIGNED,
          req.user.id,
          tenantId,
          { type: 'user', id: userId },
          { newValue: { roleId } }
        );

        res.json({ success: true });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * GET /enterprise/permissions
   * Get current user's permissions
   */
  router.get('/permissions', async (req, res) => {
    try {
      const userId = req.user?.id;
      const tenantId = req.tenantId;

      if (!userId || !tenantId) {
        return res.status(401).json({ error: 'Unauthorized' });
      }

      const permissions = await rbac.getUserPermissions(userId, tenantId);
      res.json({ permissions });
    } catch (error) {
      res.status(500).json({ error: error.message });
    }
  });

  /**
   * POST /enterprise/authorize
   * Check if user has specific permission
   */
  router.post('/authorize', async (req, res) => {
    try {
      const userId = req.user?.id;
      const tenantId = req.tenantId;
      const { resource, action, scope } = req.body;

      if (!userId || !tenantId) {
        return res.status(401).json({ error: 'Unauthorized' });
      }

      const authorized = await rbac.hasPermission(
        userId,
        tenantId,
        resource,
        action,
        scope
      );

      res.json({ authorized });
    } catch (error) {
      res.status(500).json({ error: error.message });
    }
  });

  // ============== Audit Log Routes ==============

  /**
   * GET /enterprise/audit-logs
   * Query audit logs
   */
  router.get(
    '/audit-logs',
    rbac.requirePermission(Resource.AUDIT_LOG, Action.READ),
    async (req, res) => {
      try {
        const tenantId = req.tenantId;
        const {
          eventType,
          resourceType,
          resourceId,
          startDate,
          endDate,
          severity,
          limit = 100,
          offset = 0,
        } = req.query;

        const result = await audit.query({
          tenantId,
          eventType: eventType as any,
          resourceType: resourceType as string,
          resourceId: resourceId as string,
          startDate: startDate ? new Date(startDate as string) : undefined,
          endDate: endDate ? new Date(endDate as string) : undefined,
          severity: severity as any,
          limit: Number(limit),
          offset: Number(offset),
        });

        res.json(result);
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * GET /enterprise/audit-logs/export
   * Export audit logs for compliance
   */
  router.get(
    '/audit-logs/export',
    rbac.requirePermission(Resource.AUDIT_LOG, Action.EXPORT),
    async (req, res) => {
      try {
        const tenantId = req.tenantId;
        const { startDate, endDate, format = 'json' } = req.query;

        const data = await audit.exportForCompliance(
          tenantId,
          new Date(startDate as string),
          new Date(endDate as string),
          format as 'json' | 'csv'
        );

        res.setHeader('Content-Type', format === 'json' ? 'application/json' : 'text/csv');
        res.setHeader(
          'Content-Disposition',
          `attachment; filename="audit-${startDate}-${endDate}.${format}"`
        );
        res.send(data);
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * GET /enterprise/audit-logs/summary
   * Get audit summary for dashboard
   */
  router.get(
    '/audit-logs/summary',
    rbac.requirePermission(Resource.AUDIT_LOG, Action.READ),
    async (req, res) => {
      try {
        const tenantId = req.tenantId;
        const { days = 30 } = req.query;

        const summary = await audit.getSummary(tenantId, Number(days));
        res.json(summary);
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  // ============== SSO Routes ==============

  /**
   * GET /enterprise/sso/config
   * Get SSO configuration
   */
  router.get(
    '/sso/config',
    rbac.requirePermission(Resource.TENANT, Action.READ),
    async (req, res) => {
      try {
        const tenantId = req.tenantId;
        const config = await sso.getSSOConfig(tenantId);

        // Don't expose sensitive data
        if (config) {
          const safeConfig = { ...config };
          delete (safeConfig as any).config?.samlCert;
          delete (safeConfig as any).config?.oidcClientSecret;
          delete (safeConfig as any).config?.scimBearerToken;
          res.json({ config: safeConfig });
        } else {
          res.json({ config: null });
        }
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * PUT /enterprise/sso/config
   * Configure SSO
   */
  router.put(
    '/sso/config',
    rbac.requirePermission(Resource.TENANT, Action.UPDATE),
    async (req, res) => {
      try {
        const tenantId = req.tenantId;
        const config = req.body;

        const result = await sso.configureSSO(tenantId, config);

        await audit.logResourceChange(
          AuditEventType.INTEGRATION_CONNECTED,
          req.user.id,
          tenantId,
          { type: 'sso', id: config.provider },
          { newValue: { provider: config.provider, protocol: config.protocol } }
        );

        res.json({ config: result });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * DELETE /enterprise/sso/config
   * Disable SSO
   */
  router.delete(
    '/sso/config',
    rbac.requirePermission(Resource.TENANT, Action.UPDATE),
    async (req, res) => {
      try {
        const tenantId = req.tenantId;
        await sso.disableSSO(tenantId);

        await audit.logResourceChange(
          AuditEventType.INTEGRATION_DISCONNECTED,
          req.user.id,
          tenantId,
          { type: 'sso', id: 'config' }
        );

        res.json({ success: true });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * POST /enterprise/sso/sync
   * Sync users from identity provider
   */
  router.post(
    '/sso/sync',
    rbac.requirePermission(Resource.USER, Action.CREATE),
    async (req, res) => {
      try {
        const tenantId = req.tenantId;
        const { direction } = req.body;

        const status = await sso.syncUsers(
          tenantId,
          direction || SSODirection.INBOUND
        );

        res.json({ status });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  /**
   * GET /enterprise/sso/sync/status
   * Get sync status
   */
  router.get(
    '/sso/sync/status',
    rbac.requirePermission(Resource.TENANT, Action.READ),
    async (req, res) => {
      try {
        const tenantId = req.tenantId;
        const status = await sso.getSyncStatus(tenantId);
        res.json({ status });
      } catch (error) {
        res.status(500).json({ error: error.message });
      }
    }
  );

  // ============== SCIM Endpoints ==============

  /**
   * GET /enterprise/scim/Users
   * SCIM: List users
   */
  router.get('/scim/Users', async (req, res) => {
    try {
      const tenantId = req.tenantId;
      const { startIndex = 1, count = 100 } = req.query;

      const result = await sso.scimGetUsers(
        tenantId,
        Number(startIndex),
        Number(count)
      );

      res.json(result);
    } catch (error) {
      res.status(500).json({ error: error.message });
    }
  });

  /**
   * POST /enterprise/scim/Users
   * SCIM: Create user
   */
  router.post('/scim/Users', async (req, res) => {
    try {
      const tenantId = req.tenantId;
      const user = await sso.scimCreateUser(tenantId, req.body);
      res.status(201).json(user);
    } catch (error) {
      res.status(500).json({ error: error.message });
    }
  });

  /**
   * PUT /enterprise/scim/Users/:id
   * SCIM: Update user
   */
  router.put('/scim/Users/:id', async (req, res) => {
    try {
      const tenantId = req.tenantId;
      const { id } = req.params;
      const user = await sso.scimUpdateUser(tenantId, id, req.body);
      res.json(user);
    } catch (error) {
      res.status(500).json({ error: error.message });
    }
  });

  /**
   * DELETE /enterprise/scim/Users/:id
   * SCIM: Delete user
   */
  router.delete('/scim/Users/:id', async (req, res) => {
    try {
      const tenantId = req.tenantId;
      const { id } = req.params;
      await sso.scimDeleteUser(tenantId, id);
      res.status(204).send();
    } catch (error) {
      res.status(500).json({ error: error.message });
    }
  });

  return router;
}
