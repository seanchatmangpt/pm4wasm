/**
 * PM4Wasm SaaS – RBAC (Role-Based Access Control)
 * Copyright (C) 2026 Process Intelligence Solutions GmbH
 *
 * Fine-grained access control with roles, permissions, and resource scoping.
 */

import { PrismaClient } from '../db/prisma';

/**
 * Resource types that can be protected
 */
export enum Resource {
  TENANT = 'tenant',
  SESSION = 'session',
  USER = 'user',
  BILLING = 'billing',
  ANALYTICS = 'analytics',
  API_KEY = 'api_key',
  WEBHOOK = 'webhook',
  INTEGRATION = 'integration',
  AUDIT_LOG = 'audit_log',
}

/**
 * Actions that can be performed on resources
 */
export enum Action {
  CREATE = 'create',
  READ = 'read',
  UPDATE = 'update',
  DELETE = 'delete',
  LIST = 'list',
  EXECUTE = 'execute',
  EXPORT = 'export',
  IMPORT = 'import',
  APPROVE = 'approve',
  REJECT = 'reject',
}

/**
 * Permission represents a single granular permission
 */
export interface Permission {
  resource: Resource;
  action: Action;
  scope?: string; // Optional scope for fine-grained control (e.g., "department:hr")
}

/**
 * Role defines a collection of permissions
 */
export interface Role {
  id: string;
  name: string;
  description: string;
  permissions: Permission[];
  isSystem: boolean; // System roles cannot be deleted
  tenantId?: string; // Tenant-specific roles (null = global)
}

/**
 * Predefined system roles
 */
export const SYSTEM_ROLES: Record<string, Omit<Role, 'id'>> = {
  OWNER: {
    name: 'Owner',
    description: 'Full access to all resources and billing',
    permissions: [
      // All permissions on all resources
      ...Object.values(Resource).flatMap(resource =>
        Object.values(Action).map(action => ({ resource, action }))
      ),
    ],
    isSystem: true,
  },
  ADMIN: {
    name: 'Admin',
    description: 'Full access except billing and owner management',
    permissions: [
      // Tenant management
      { resource: Resource.TENANT, action: Action.READ },
      { resource: Resource.TENANT, action: Action.UPDATE },
      // Session management
      { resource: Resource.SESSION, action: Action.CREATE },
      { resource: Resource.SESSION, action: Action.READ },
      { resource: Resource.SESSION, action: Action.UPDATE },
      { resource: Resource.SESSION, action: Action.DELETE },
      { resource: Resource.SESSION, action: Action.LIST },
      // User management
      { resource: Resource.USER, action: Action.CREATE },
      { resource: Resource.USER, action: Action.READ },
      { resource: Resource.USER, action: Action.UPDATE },
      { resource: Resource.USER, action: Action.LIST },
      // Analytics
      { resource: Resource.ANALYTICS, action: Action.READ },
      { resource: Resource.ANALYTICS, action: Action.EXPORT },
      // API keys
      { resource: Resource.API_KEY, action: Action.CREATE },
      { resource: Resource.API_KEY, action: Action.READ },
      { resource: Resource.API_KEY, action: Action.DELETE },
      { resource: Resource.API_KEY, action: Action.LIST },
      // Webhooks
      { resource: Resource.WEBHOOK, action: Action.CREATE },
      { resource: Resource.WEBHOOK, action: Action.READ },
      { resource: Resource.WEBHOOK, action: Action.UPDATE },
      { resource: Resource.WEBHOOK, action: Action.DELETE },
      { resource: Resource.WEBHOOK, action: Action.LIST },
      // Integrations
      { resource: Resource.INTEGRATION, action: Action.CREATE },
      { resource: Resource.INTEGRATION, action: Action.READ },
      { resource: Resource.INTEGRATION, action: Action.UPDATE },
      { resource: Resource.INTEGRATION, action: Action.DELETE },
      { resource: Resource.INTEGRATION, action: Action.LIST },
      // Audit logs (read-only)
      { resource: Resource.AUDIT_LOG, action: Action.READ },
      { resource: Resource.AUDIT_LOG, action: Action.LIST },
      { resource: Resource.AUDIT_LOG, action: Action.EXPORT },
    ],
    isSystem: true,
  },
  ANALYST: {
    name: 'Analyst',
    description: 'Read-only access to analytics and session data',
    permissions: [
      { resource: Resource.SESSION, action: Action.READ },
      { resource: Resource.SESSION, action: Action.LIST },
      { resource: Resource.ANALYTICS, action: Action.READ },
      { resource: Resource.ANALYTICS, action: Action.EXPORT },
      { resource: Resource.AUDIT_LOG, action: Action.READ },
      { resource: Resource.AUDIT_LOG, action: Action.LIST },
    ],
    isSystem: true,
  },
  MEMBER: {
    name: 'Member',
    description: 'Can create and view sessions',
    permissions: [
      { resource: Resource.SESSION, action: Action.CREATE },
      { resource: Resource.SESSION, action: Action.READ },
      { resource: Resource.SESSION, action: Action.UPDATE },
      { resource: Resource.SESSION, action: Action.LIST },
      { resource: Resource.ANALYTICS, action: Action.READ },
    ],
    isSystem: true,
  },
  VIEWER: {
    name: 'Viewer',
    description: 'Read-only access to shared sessions',
    permissions: [
      { resource: Resource.SESSION, action: Action.READ },
      { resource: Resource.SESSION, action: Action.LIST },
    ],
    isSystem: true,
  },
};

/**
 * RBACService handles role-based access control
 */
export class RBACService {
  private prisma: PrismaClient;
  private roleCache: Map<string, Role> = new Map();

  constructor(prisma: PrismaClient) {
    this.prisma = prisma;
    this.initializeSystemRoles();
  }

  /**
   * Initialize system roles in database
   */
  private async initializeSystemRoles(): Promise<void> {
    for (const [key, role] of Object.entries(SYSTEM_ROLES)) {
      const existing = await this.prisma.role.upsert({
        where: { name: role.name },
        update: {},
        create: {
          name: role.name,
          description: role.description,
          permissions: role.permissions,
          isSystem: true,
        },
      });
      this.roleCache.set(existing.name, existing as Role);
    }
  }

  /**
   * Check if a user has permission to perform an action
   */
  async hasPermission(
    userId: string,
    tenantId: string,
    resource: Resource,
    action: Action,
    scope?: string
  ): Promise<boolean> {
    // Get user's role in tenant
    const membership = await this.prisma.tenantMember.findUnique({
      where: {
        userId_tenantId: {
          userId,
          tenantId,
        },
      },
      include: {
        role: true,
      },
    });

    if (!membership) {
      return false;
    }

    const role = membership.role as Role;

    // Check if role has the required permission
    return role.permissions.some(p => {
      const matchResource = p.resource === resource;
      const matchAction = p.action === action;
      const matchScope = !scope || !p.scope || p.scope === scope;
      return matchResource && matchAction && matchScope;
    });
  }

  /**
   * Get all permissions for a user in a tenant
   */
  async getUserPermissions(
    userId: string,
    tenantId: string
  ): Promise<Permission[]> {
    const membership = await this.prisma.tenantMember.findUnique({
      where: {
        userId_tenantId: {
          userId,
          tenantId,
        },
      },
      include: {
        role: true,
      },
    });

    if (!membership) {
      return [];
    }

    const role = membership.role as Role;
    return role.permissions;
  }

  /**
   * Create a custom role for a tenant
   */
  async createRole(
    tenantId: string,
    name: string,
    description: string,
    permissions: Permission[]
  ): Promise<Role> {
    const role = await this.prisma.role.create({
      data: {
        name,
        description,
        permissions,
        tenantId,
        isSystem: false,
      },
    });

    this.roleCache.set(role.name, role as Role);
    return role as Role;
  }

  /**
   * Update a role
   */
  async updateRole(
    roleId: string,
    updates: Partial<Pick<Role, 'name' | 'description' | 'permissions'>>
  ): Promise<Role> {
    const existing = await this.prisma.role.findUnique({
      where: { id: roleId },
    });

    if (!existing) {
      throw new Error('Role not found');
    }

    if (existing.isSystem) {
      throw new Error('Cannot modify system roles');
    }

    const role = await this.prisma.role.update({
      where: { id: roleId },
      data: updates,
    });

    this.roleCache.set(role.name, role as Role);
    return role as Role;
  }

  /**
   * Delete a role
   */
  async deleteRole(roleId: string): Promise<void> {
    const existing = await this.prisma.role.findUnique({
      where: { id: roleId },
    });

    if (!existing) {
      throw new Error('Role not found');
    }

    if (existing.isSystem) {
      throw new Error('Cannot delete system roles');
    }

    await this.prisma.role.delete({
      where: { id: roleId },
    });
  }

  /**
   * Assign role to user in tenant
   */
  async assignRole(
    userId: string,
    tenantId: string,
    roleId: string
  ): Promise<void> {
    await this.prisma.tenantMember.update({
      where: {
        userId_tenantId: {
          userId,
          tenantId,
        },
      },
      data: { roleId },
    });
  }

  /**
   * Get all roles for a tenant (system + custom)
   */
  async getTenantRoles(tenantId: string): Promise<Role[]> {
    const systemRoles = Object.values(SYSTEM_ROLES).map(r => ({
      ...r,
      id: `system:${r.name}`,
    })) as Role[];

    const customRoles = await this.prisma.role.findMany({
      where: { tenantId },
    }) as Role[];

    return [...systemRoles, ...customRoles];
  }

  /**
   * Authorize request - throws if unauthorized
   */
  async authorize(
    userId: string,
    tenantId: string,
    resource: Resource,
    action: Action,
    scope?: string
  ): Promise<void> {
    const hasPermission = await this.hasPermission(
      userId,
      tenantId,
      resource,
      action,
      scope
    );

    if (!hasPermission) {
      throw new Error(
        `Unauthorized: ${action} on ${resource}${scope ? ` (${scope})` : ''}`
      );
    }
  }

  /**
   * Middleware factory for Express routes
   */
  requirePermission(resource: Resource, action: Action, scope?: string) {
    return async (req: any, res: any, next: any) => {
      const userId = req.user?.id;
      const tenantId = req.tenantId;

      if (!userId || !tenantId) {
        return res.status(401).json({ error: 'Unauthorized' });
      }

      const hasPermission = await this.hasPermission(
        userId,
        tenantId,
        resource,
        action,
        scope
      );

      if (!hasPermission) {
        return res.status(403).json({
          error: 'Forbidden',
          message: `Missing permission: ${action} on ${resource}`,
        });
      }

      next();
    };
  }
}
