/**
 * PM4Wasm SaaS – Audit Logs
 * Copyright (C) 2026 Process Intelligence Solutions GmbH
 *
 * Comprehensive audit trail for compliance (SOC2, HIPAA, GDPR).
 */

import { PrismaClient } from '../db/prisma';
import { logger } from '../lib/logger';

/**
 * Audit event types for categorization
 */
export enum AuditEventType {
  // Authentication
  AUTH_LOGIN = 'auth.login',
  AUTH_LOGOUT = 'auth.logout',
  AUTH_FAILED = 'auth.failed',
  AUTH_PASSWORD_RESET = 'auth.password_reset',
  AUTH_MFA_ENABLED = 'auth.mfa_enabled',
  AUTH_MFA_DISABLED = 'auth.mfa_disabled',

  // User management
  USER_CREATED = 'user.created',
  USER_UPDATED = 'user.updated',
  USER_DELETED = 'user.deleted',
  USER_INVITED = 'user.invited',

  // Tenant management
  TENANT_CREATED = 'tenant.created',
  TENANT_UPDATED = 'tenant.updated',
  TENANT_DELETED = 'tenant.deleted',

  // Role/Permission changes
  ROLE_ASSIGNED = 'role.assigned',
  ROLE_REVOKED = 'role.revoked',
  ROLE_CREATED = 'role.created',
  ROLE_UPDATED = 'role.updated',
  ROLE_DELETED = 'role.deleted',

  // Session management
  SESSION_CREATED = 'session.created',
  SESSION_UPDATED = 'session.updated',
  SESSION_DELETED = 'session.deleted',
  SESSION_SHARED = 'session.shared',

  // Data access
  DATA_EXPORTED = 'data.exported',
  DATA_IMPORTED = 'data.imported',
  DATA_ACCESSED = 'data.accessed',

  // Billing
  BILLING_PLAN_CHANGED = 'billing.plan_changed',
  BILLING_PAYMENT_METHOD_ADDED = 'billing.payment_method_added',
  BILLING_INVOICE_GENERATED = 'billing.invoice_generated',

  // API keys
  API_KEY_CREATED = 'api_key.created',
  API_KEY_DELETED = 'api_key.deleted',
  API_KEY_ROTATED = 'api_key.rotated',

  // Webhooks
  WEBHOOK_CREATED = 'webhook.created',
  WEBHOOK_UPDATED = 'webhook.updated',
  WEBHOOK_DELETED = 'webhook.deleted',
  WEBHOOK_TRIGGERED = 'webhook.triggered',

  // Integrations
  INTEGRATION_CONNECTED = 'integration.connected',
  INTEGRATION_DISCONNECTED = 'integration.disconnected',
  INTEGRATION_SYNCED = 'integration.synced',

  // Security
  SECURITY_SUSPICIOUS_ACTIVITY = 'security.suspicious_activity',
  SECURITY_RATE_LIMIT_EXCEEDED = 'security.rate_limit_exceeded',
  SECURITY_UNAUTHORIZED_ACCESS = 'security.unauthorized_access',

  // Compliance
  COMPLIANCE_AUDIT_EXPORTED = 'compliance.audit_exported',
  COMPLIANCE_RETENTION_APPLIED = 'compliance.retention_applied',
}

/**
 * Audit event metadata
 */
export interface AuditEventMetadata {
  ip?: string;
  userAgent?: string;
  requestId?: string;
  sessionId?: string;
  resourceType?: string;
  resourceId?: string;
  oldValue?: any;
  newValue?: any;
  reason?: string;
}

/**
 * Complete audit event
 */
export interface AuditEvent {
  id: string;
  eventType: AuditEventType;
  userId?: string;
  tenantId?: string;
  actor?: {
    id: string;
    email: string;
    name?: string;
  };
  resource?: {
    type: string;
    id: string;
    name?: string;
  };
  action: string;
  metadata: AuditEventMetadata;
  timestamp: Date;
  severity: 'info' | 'warning' | 'error' | 'critical';
}

/**
 * Data retention policies (in days)
 */
export const RETENTION_POLICIES = {
  [AuditEventType.AUTH_LOGIN]: 365,
  [AuditEventType.AUTH_LOGOUT]: 365,
  [AuditEventType.AUTH_FAILED]: 90,
  [AuditEventType.USER_CREATED]: 2555, // 7 years for compliance
  [AuditEventType.USER_DELETED]: 2555,
  [AuditEventType.ROLE_ASSIGNED]: 2555,
  [AuditEventType.DATA_EXPORTED]: 2555,
  [AuditEventType.COMPLIANCE_AUDIT_EXPORTED]: 2555,
  [AuditEventType.SECURITY_SUSPICIOUS_ACTIVITY]: 2555,
  [AuditEventType.BILLING_INVOICE_GENERATED]: 2555, // 7 years for tax
};

/**
 * Default retention for unlisted events
 */
const DEFAULT_RETENTION_DAYS = 90;

/**
 * AuditService manages audit log storage and retrieval
 */
export class AuditService {
  private prisma: PrismaClient;
  private retentionDays: Map<AuditEventType, number> = new Map();

  constructor(prisma: PrismaClient) {
    this.prisma = prisma;
    this.initializeRetention();
  }

  private initializeRetention(): void {
    for (const [eventType, days] of Object.entries(RETENTION_POLICIES)) {
      this.retentionDays.set(eventType as AuditEventType, days);
    }
  }

  /**
   * Log an audit event
   */
  async log(event: Omit<AuditEvent, 'id' | 'timestamp'>): Promise<AuditEvent> {
    const auditEvent: AuditEvent = {
      ...event,
      id: this.generateId(),
      timestamp: new Date(),
    };

    // Store in database
    await this.prisma.auditLog.create({
      data: {
        id: auditEvent.id,
        eventType: auditEvent.eventType,
        userId: auditEvent.userId,
        tenantId: auditEvent.tenantId,
        actorId: auditEvent.actor?.id,
        actorEmail: auditEvent.actor?.email,
        actorName: auditEvent.actor?.name,
        resourceType: auditEvent.resource?.type,
        resourceId: auditEvent.resource?.id,
        resourceName: auditEvent.resource?.name,
        action: auditEvent.action,
        metadata: auditEvent.metadata,
        severity: auditEvent.severity,
        timestamp: auditEvent.timestamp,
        expiresAt: this.calculateExpiry(auditEvent.eventType),
      },
    });

    // Log to Winston for immediate visibility
    logger.info('Audit event', {
      eventType: auditEvent.eventType,
      userId: auditEvent.userId,
      tenantId: auditEvent.tenantId,
      action: auditEvent.action,
      resource: auditEvent.resource,
    });

    return auditEvent;
  }

  /**
   * Query audit logs with filters
   */
  async query(filters: {
    tenantId?: string;
    userId?: string;
    eventType?: AuditEventType | AuditEventType[];
    resourceType?: string;
    resourceId?: string;
    startDate?: Date;
    endDate?: Date;
    severity?: AuditEvent['severity'];
    limit?: number;
    offset?: number;
  }): Promise<{ events: AuditEvent[]; total: number }> {
    const where: any = {};

    if (filters.tenantId) where.tenantId = filters.tenantId;
    if (filters.userId) where.userId = filters.userId;
    if (filters.resourceType) where.resourceType = filters.resourceType;
    if (filters.resourceId) where.resourceId = filters.resourceId;
    if (filters.severity) where.severity = filters.severity;

    if (filters.eventType) {
      const types = Array.isArray(filters.eventType)
        ? filters.eventType
        : [filters.eventType];
      where.eventType = { in: types };
    }

    if (filters.startDate || filters.endDate) {
      where.timestamp = {};
      if (filters.startDate) where.timestamp.gte = filters.startDate;
      if (filters.endDate) where.timestamp.lte = filters.endDate;
    }

    const [total, events] = await Promise.all([
      this.prisma.auditLog.count({ where }),
      this.prisma.auditLog.findMany({
        where,
        orderBy: { timestamp: 'desc' },
        take: filters.limit || 100,
        skip: filters.offset || 0,
      }),
    ]);

    return {
      events: events.map(e => this.formatEvent(e)),
      total,
    };
  }

  /**
   * Export audit logs for compliance
   */
  async exportForCompliance(
    tenantId: string,
    startDate: Date,
    endDate: Date,
    format: 'json' | 'csv' = 'json'
  ): Promise<string> {
    const { events } = await this.query({
      tenantId,
      startDate,
      endDate,
      limit: 100000,
    });

    // Log the export itself
    await this.log({
      eventType: AuditEventType.COMPLIANCE_AUDIT_EXPORTED,
      tenantId,
      action: 'export',
      resource: {
        type: 'audit_log',
        id: `export_${Date.now()}`,
      },
      metadata: {
        startDate: startDate.toISOString(),
        endDate: endDate.toISOString(),
        eventCount: events.length,
        format,
      },
      severity: 'info',
    });

    if (format === 'json') {
      return JSON.stringify(events, null, 2);
    }

    // CSV format
    const headers = [
      'timestamp',
      'eventType',
      'actor',
      'resource',
      'action',
      'metadata',
    ];
    const rows = events.map(e => [
      e.timestamp.toISOString(),
      e.eventType,
      e.actor?.email || '',
      `${e.resource?.type}:${e.resource?.id}` || '',
      e.action,
      JSON.stringify(e.metadata),
    ]);
    return [headers.join(','), ...rows.map(r => r.join(','))].join('\n');
  }

  /**
   * Apply retention policy - delete expired events
   */
  async applyRetentionPolicy(): Promise<number> {
    const now = new Date();
    let deleted = 0;

    for (const [eventType, retentionDays] of this.retentionDays) {
      const cutoff = new Date(now.getTime() - retentionDays * 24 * 60 * 60 * 1000);

      const result = await this.prisma.auditLog.deleteMany({
        where: {
          eventType,
          timestamp: { lt: cutoff },
        },
      });

      deleted += result.count;
    }

    // Also delete events with no specific retention policy
    const defaultCutoff = new Date(
      now.getTime() - DEFAULT_RETENTION_DAYS * 24 * 60 * 60 * 1000
    );
    const listedTypes = Array.from(this.retentionDays.keys());
    const defaultResult = await this.prisma.auditLog.deleteMany({
      where: {
        eventType: { notIn: listedTypes },
        timestamp: { lt: defaultCutoff },
      },
    });
    deleted += defaultResult.count;

    logger.info(`Applied retention policy: deleted ${deleted} expired events`);

    return deleted;
  }

  /**
   * Get audit summary for dashboard
   */
  async getSummary(tenantId: string, days: number = 30): Promise<{
    totalEvents: number;
    byEventType: Record<string, number>;
    bySeverity: Record<string, number>;
    topUsers: Array<{ userId: string; email: string; count: number }>;
    suspiciousActivities: number;
  }> {
    const startDate = new Date(Date.now() - days * 24 * 60 * 60 * 1000);

    const events = await this.prisma.auditLog.findMany({
      where: {
        tenantId,
        timestamp: { gte: startDate },
      },
    });

    const byEventType: Record<string, number> = {};
    const bySeverity: Record<string, number> = {};
    const userCounts: Record<string, { email: string; count: number }> = {};
    let suspiciousActivities = 0;

    for (const event of events) {
      // Count by type
      byEventType[event.eventType] = (byEventType[event.eventType] || 0) + 1;

      // Count by severity
      bySeverity[event.severity] = (bySeverity[event.severity] || 0) + 1;

      // Count by user
      if (event.userId) {
        if (!userCounts[event.userId]) {
          userCounts[event.userId] = { email: event.actorEmail || '', count: 0 };
        }
        userCounts[event.userId].count++;
      }

      // Count suspicious activities
      if (
        event.eventType === AuditEventType.SECURITY_SUSPICIOUS_ACTIVITY ||
        event.eventType === AuditEventType.SECURITY_UNAUTHORIZED_ACCESS ||
        event.eventType === AuditEventType.AUTH_FAILED
      ) {
        suspiciousActivities++;
      }
    }

    const topUsers = Object.entries(userCounts)
      .map(([userId, { email, count }]) => ({ userId, email, count }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 10);

    return {
      totalEvents: events.length,
      byEventType,
      bySeverity,
      topUsers,
      suspiciousActivities,
    };
  }

  /**
   * Helper: Log authentication event
   */
  async logAuth(
    eventType: AuditEventType,
    userId: string,
    tenantId?: string,
    metadata?: AuditEventMetadata
  ): Promise<AuditEvent> {
    return this.log({
      eventType,
      userId,
      tenantId,
      action: eventType.split('.')[1] || 'auth',
      metadata: metadata || {},
      severity: 'info',
    });
  }

  /**
   * Helper: log resource change event
   */
  async logResourceChange(
    eventType: AuditEventType,
    userId: string,
    tenantId: string,
    resource: { type: string; id: string; name?: string },
    changes?: { oldValue?: any; newValue?: any; reason?: string }
  ): Promise<AuditEvent> {
    return this.log({
      eventType,
      userId,
      tenantId,
      resource,
      action: eventType.split('.')[1] || 'update',
      metadata: {
        oldValue: changes?.oldValue,
        newValue: changes?.newValue,
        reason: changes?.reason,
      },
      severity: 'info',
    });
  }

  /**
   * Helper: log security event
   */
  async logSecurity(
    eventType: AuditEventType,
    tenantId: string,
    metadata: AuditEventMetadata & {
      reason: string;
      severity?: AuditEvent['severity'];
    }
  ): Promise<AuditEvent> {
    return this.log({
      eventType,
      tenantId,
      action: 'security_alert',
      metadata,
      severity: metadata.severity || 'warning',
    });
  }

  private calculateExpiry(eventType: AuditEventType): Date {
    const retentionDays = this.retentionDays.get(eventType) || DEFAULT_RETENTION_DAYS;
    return new Date(Date.now() + retentionDays * 24 * 60 * 60 * 1000);
  }

  private generateId(): string {
    return `audit_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  private formatEvent(event: any): AuditEvent {
    return {
      id: event.id,
      eventType: event.eventType,
      userId: event.userId,
      tenantId: event.tenantId,
      actor: event.actorId
        ? {
            id: event.actorId,
            email: event.actorEmail,
            name: event.actorName,
          }
        : undefined,
      resource: event.resourceType
        ? {
            type: event.resourceType,
            id: event.resourceId,
            name: event.resourceName,
          }
        : undefined,
      action: event.action,
      metadata: event.metadata,
      timestamp: event.timestamp,
      severity: event.severity,
    };
  }
}
