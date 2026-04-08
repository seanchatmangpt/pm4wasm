/**
 * Tenant management service
 */

import { prisma, checkTenantMembership, getTenantUsage, incrementUsage } from '../db/prisma.js';
import { AuthError } from '../auth/types.js';

export interface CreateTenantInput {
  name: string;
  plan?: 'FREE' | 'PRO' | 'ENTERPRISE';
}

export interface UpdateTenantInput {
  name?: string;
  settings?: Record<string, unknown>;
}

export interface AddMemberInput {
  email: string;
  role: 'OWNER' | 'ADMIN' | 'EDITOR' | 'VIEWER';
}

export class TenantService {
  /**
   * List all tenants the user is a member of
   */
  async listUserTenants(userId: string) {
    const memberships = await prisma.tenantMember.findMany({
      where: { userId },
      include: { tenant: true },
      orderBy: { createdAt: 'asc' },
    });

    return memberships.map((m) => ({
      id: m.tenant.id,
      name: m.tenant.name,
      slug: m.tenant.slug,
      plan: m.tenant.plan,
      role: m.role,
      createdAt: m.tenant.createdAt,
    }));
  }

  /**
   * Get a tenant by ID (user must be a member)
   */
  async getTenant(tenantId: string, userId: string) {
    const membership = await prisma.tenantMember.findUnique({
      where: {
        tenantId_userId: { tenantId, userId },
      },
      include: { tenant: true },
    });

    if (!membership) {
      throw new Error(AuthError.TOKEN_INVALID); // Reuse auth error for "not found"
    }

    return {
      id: membership.tenant.id,
      name: membership.tenant.name,
      slug: membership.tenant.slug,
      plan: membership.tenant.plan,
      settings: membership.tenant.settings,
      role: membership.role,
      createdAt: membership.tenant.createdAt,
      updatedAt: membership.tenant.updatedAt,
    };
  }

  /**
   * Create a new tenant
   */
  async createTenant(userId: string, input: CreateTenantInput) {
    // Check user's tenant count limit
    const count = await prisma.tenantMember.count({ where: { userId } });
    if (count >= 5) {
      throw new Error('TENANT_LIMIT_REACHED');
    }

    const slug = this.generateSlug(input.name);

    const tenant = await prisma.tenant.create({
      data: {
        name: input.name,
        slug,
        plan: input.plan || 'FREE',
        members: {
          create: {
            userId,
            role: 'OWNER',
          },
        },
      },
    });

    return {
      id: tenant.id,
      name: tenant.name,
      slug: tenant.slug,
      plan: tenant.plan,
      createdAt: tenant.createdAt,
    };
  }

  /**
   * Update tenant settings
   */
  async updateTenant(tenantId: string, userId: string, input: UpdateTenantInput) {
    const hasAccess = await checkTenantMembership(userId, tenantId, ['OWNER', 'ADMIN']);
    if (!hasAccess) {
      throw new Error('ACCESS_DENIED');
    }

    const tenant = await prisma.tenant.update({
      where: { id: tenantId },
      data: {
        ...(input.name && { name: input.name }),
        ...(input.settings && { settings: input.settings as any }),
      },
    });

    return {
      id: tenant.id,
      name: tenant.name,
      slug: tenant.slug,
      plan: tenant.plan,
      settings: tenant.settings,
      updatedAt: tenant.updatedAt,
    };
  }

  /**
   * Delete a tenant (owner only)
   */
  async deleteTenant(tenantId: string, userId: string) {
    const membership = await prisma.tenantMember.findUnique({
      where: {
        tenantId_userId: { tenantId, userId },
      },
    });

    if (!membership || membership.role !== 'OWNER') {
      throw new Error('ACCESS_DENIED');
    }

    await prisma.tenant.delete({ where: { id: tenantId } });
  }

  /**
   * List tenant members
   */
  async listMembers(tenantId: string, userId: string) {
    const hasAccess = await checkTenantMembership(userId, tenantId);
    if (!hasAccess) {
      throw new Error('ACCESS_DENIED');
    }

    const members = await prisma.tenantMember.findMany({
      where: { tenantId },
      include: { user: true },
      orderBy: { createdAt: 'asc' },
    });

    return members.map((m) => ({
      id: m.id,
      user: {
        id: m.user.id,
        email: m.user.email,
        name: m.user.name,
        avatarUrl: m.user.avatarUrl,
      },
      role: m.role,
      joinedAt: m.createdAt,
    }));
  }

  /**
   * Add a member to the tenant
   */
  async addMember(tenantId: string, userId: string, input: AddMemberInput) {
    const hasAccess = await checkTenantMembership(userId, tenantId, ['OWNER', 'ADMIN']);
    if (!hasAccess) {
      throw new Error('ACCESS_DENIED');
    }

    // Find or create user
    let user = await prisma.user.findUnique({
      where: { email: input.email },
    });

    if (!user) {
      // Create a pending user (no password - must use OAuth or set password)
      user = await prisma.user.create({
        data: {
          email: input.email,
          name: input.email.split('@')[0],
        },
      });
    }

    // Check if already a member
    const existing = await prisma.tenantMember.findUnique({
      where: {
        tenantId_userId: { tenantId, userId: user.id },
      },
    });

    if (existing) {
      throw new Error('ALREADY_MEMBER');
    }

    // Check member limit
    const memberCount = await prisma.tenantMember.count({ where: { tenantId } });
    const tenant = await prisma.tenant.findUnique({ where: { id: tenantId } });
    const limit = tenant?.plan === 'ENTERPRISE' ? -1 : tenant?.plan === 'PRO' ? 10 : 3;
    if (limit !== -1 && memberCount >= limit) {
      throw new Error('MEMBER_LIMIT_REACHED');
    }

    const member = await prisma.tenantMember.create({
      data: {
        tenantId,
        userId: user.id,
        role: input.role,
      },
      include: { user: true },
    });

    return {
      id: member.id,
      user: {
        id: member.user.id,
        email: member.user.email,
        name: member.user.name,
        avatarUrl: member.user.avatarUrl,
      },
      role: member.role,
      joinedAt: member.createdAt,
    };
  }

  /**
   * Remove a member from the tenant
   */
  async removeMember(tenantId: string, userId: string, memberId: string) {
    const requesterMembership = await prisma.tenantMember.findUnique({
      where: {
        tenantId_userId: { tenantId, userId },
      },
    });

    if (!requesterMembership || requesterMembership.role === 'VIEWER') {
      throw new Error('ACCESS_DENIED');
    }

    // Can't remove the owner
    const targetMember = await prisma.tenantMember.findUnique({
      where: { id: memberId },
    });

    if (targetMember?.role === 'OWNER') {
      throw new Error('CANNOT_REMOVE_OWNER');
    }

    await prisma.tenantMember.delete({ where: { id: memberId } });
  }

  /**
   * Get tenant usage
   */
  async getUsage(tenantId: string, userId: string) {
    const hasAccess = await checkTenantMembership(userId, tenantId);
    if (!hasAccess) {
      throw new Error('ACCESS_DENIED');
    }

    return getTenantUsage(tenantId);
  }

  /**
   * Generate a URL-safe slug
   */
  private generateSlug(name: string): string {
    const base = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    const suffix = Math.random().toString(36).substring(2, 8);
    return `${base}-${suffix}`;
  }
}

export const tenantService = new TenantService();
