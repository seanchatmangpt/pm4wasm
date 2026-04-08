/**
 * Prisma client singleton with tenant scoping
 */

import { PrismaClient } from '@prisma/client';

const globalForPrisma = globalThis as unknown as {
  prisma: PrismaClient | undefined;
};

export const prisma = globalForPrisma.prisma ?? new PrismaClient({
  log: process.env.NODE_ENV === 'development' ? ['query', 'error', 'warn'] : ['error'],
});

if (process.env.NODE_ENV !== 'production') {
  globalForPrisma.prisma = prisma;
}

/**
 * Create a Prisma client scoped to a specific tenant
 * This ensures all queries are automatically filtered by tenant_id
 */
export function createTenantScopedClient(tenantId: string) {
  return prisma.$extends({
    query: {
      $allOperations({ operation, model, args, query }) {
        // Inject tenant filter into queries that support it
        if (model === 'Session' || model === 'UsageRecord') {
          args = args || {};
          args.where = { ...args.where, tenantId } as any;
        }
        return query(args);
      },
    },
  });
}

/**
 * Check if a user is a member of a tenant and has required role
 */
export async function checkTenantMembership(
  userId: string,
  tenantId: string,
  roles?: string[]
): Promise<boolean> {
  const membership = await prisma.tenantMember.findUnique({
    where: {
      tenantId_userId: { tenantId, userId },
    },
  });

  if (!membership) {
    return false;
  }

  if (roles && !roles.includes(membership.role)) {
    return false;
  }

  return true;
}

/**
 * Get tenant usage for current billing period
 */
export async function getTenantUsage(tenantId: string) {
  const now = new Date();
  const periodStart = new Date(now.getFullYear(), now.getMonth(), 1);
  const periodEnd = new Date(now.getFullYear(), now.getMonth() + 1, 0, 23, 59, 59);

  const usage = await prisma.usageRecord.findUnique({
    where: {
      tenantId_periodStart: { tenantId, periodStart },
    },
  });

  const tenant = await prisma.tenant.findUnique({
    where: { id: tenantId },
    select: { plan: true },
  });

  // Define limits per plan
  const limits = {
    FREE: { apiCalls: 1000, storageMb: 100, sessionMinutes: 60, activeUsers: 1 },
    PRO: { apiCalls: 100000, storageMb: 10000, sessionMinutes: 1000, activeUsers: 10 },
    ENTERPRISE: { apiCalls: -1, storageMb: -1, sessionMinutes: -1, activeUsers: -1 },
  };

  return {
    periodStart,
    periodEnd,
    apiCalls: usage?.apiCalls ?? 0,
    storageMb: usage?.storageMb ?? 0,
    sessionMinutes: usage?.sessionMinutes ?? 0,
    activeUsers: usage?.activeUsers ?? 0,
    limits: limits[tenant?.plan ?? 'FREE'],
  };
}

/**
 * Increment usage counter for a tenant
 */
export async function incrementUsage(
  tenantId: string,
  metric: 'apiCalls' | 'sessionMinutes' | 'activeUsers',
  amount = 1
): Promise<void> {
  const now = new Date();
  const periodStart = new Date(now.getFullYear(), now.getMonth(), 1);

  await prisma.usageRecord.upsert({
    where: {
      tenantId_periodStart: { tenantId, periodStart },
    },
    create: {
      tenantId,
      periodStart,
      periodEnd: new Date(now.getFullYear(), now.getMonth() + 1, 0, 23, 59, 59),
      [metric]: amount,
    },
    update: {
      [metric]: { increment: amount },
    },
  });
}
