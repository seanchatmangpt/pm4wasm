/**
 * Collaboration service - manages real-time sessions
 */

import { prisma, checkTenantMembership } from '../db/prisma.js';
import type { CreateSessionInput, SessionUser, ModelUpdate } from './types.js';

export class CollabService {
  /**
   * Create a new collaborative session
   */
  async createSession(tenantId: string, userId: string, input: CreateSessionInput) {
    const session = await prisma.session.create({
      data: {
        tenantId,
        name: input.name,
        model: input.model,
        isPublic: input.isPublic ?? false,
        createdBy: userId,
      },
    });

    // Record creation in history
    await prisma.sessionHistory.create({
      data: {
        sessionId: session.id,
        userId,
        action: 'join',
        data: { initial: true },
      },
    });

    return session;
  }

  /**
   * Get a session by ID
   */
  async getSession(sessionId: string, userId?: string) {
    const session = await prisma.session.findUnique({
      where: { id: sessionId },
    });

    if (!session) {
      throw new Error('SESSION_NOT_FOUND');
    }

    // Check access if private
    if (!session.isPublic && userId) {
      const hasAccess = await checkTenantMembership(userId, session.tenantId);
      if (!hasAccess) {
        throw new Error('ACCESS_DENIED');
      }
    }

    return session;
  }

  /**
   * Delete a session
   */
  async deleteSession(sessionId: string, userId: string) {
    const session = await prisma.session.findUnique({
      where: { id: sessionId },
    });

    if (!session) {
      throw new Error('SESSION_NOT_FOUND');
    }

    if (session.createdBy !== userId) {
      // Check if user is admin
      const hasAccess = await checkTenantMembership(userId, session.tenantId, ['OWNER', 'ADMIN']);
      if (!hasAccess) {
        throw new Error('ACCESS_DENIED');
      }
    }

    await prisma.session.delete({ where: { id: sessionId } });
  }

  /**
   * List sessions for a tenant
   */
  async listSessions(tenantId: string, userId: string) {
    const hasAccess = await checkTenantMembership(userId, tenantId);
    if (!hasAccess) {
      throw new Error('ACCESS_DENIED');
    }

    const sessions = await prisma.session.findMany({
      where: { tenantId },
      include: { creator: true },
      orderBy: { updatedAt: 'desc' },
    });

    return sessions.map((s) => ({
      id: s.id,
      name: s.name,
      isPublic: s.isPublic,
      createdBy: {
        id: s.creator.id,
        name: s.creator.name,
      },
      createdAt: s.createdAt,
      updatedAt: s.updatedAt,
    }));
  }

  /**
   * Record a model update in session history
   */
  async recordUpdate(sessionId: string, userId: string, update: ModelUpdate) {
    await prisma.sessionHistory.create({
      data: {
        sessionId,
        userId,
        action: 'update',
        data: update,
      },
    });

    // Update session timestamp
    await prisma.session.update({
      where: { id: sessionId },
      data: { updatedAt: new Date() },
    });
  }

  /**
   * Record cursor movement
   */
  async recordCursor(sessionId: string, userId: string, cursor: { line: number; column: number }) {
    await prisma.sessionHistory.create({
      data: {
        sessionId,
        userId,
        action: 'cursor',
        data: { cursor },
      },
    });
  }

  /**
   * Get session history for replay
   */
  async getSessionHistory(sessionId: string) {
    const history = await prisma.sessionHistory.findMany({
      where: { sessionId },
      orderBy: { timestamp: 'asc' },
    });

    return history.map((h) => ({
      userId: h.userId,
      action: h.action,
      data: h.data,
      timestamp: h.timestamp,
    }));
  }

  /**
   * Generate a WebSocket ticket for session join
   */
  generateJoinTicket(sessionId: string, userId: string): string {
    const timestamp = Date.now();
    const data = `${sessionId}:${userId}:${timestamp}`;
    return Buffer.from(data).toString('base64');
  }

  /**
   * Validate a join ticket
   */
  validateJoinTicket(ticket: string): { sessionId: string; userId: string; timestamp: number } | null {
    try {
      const data = Buffer.from(ticket, 'base64').toString('utf-8');
      const [sessionId, userId, timestamp] = data.split(':');

      // Check if ticket is expired (5 minutes)
      const ticketTime = parseInt(timestamp, 10);
      if (Date.now() - ticketTime > 5 * 60 * 1000) {
        return null;
      }

      return { sessionId, userId, timestamp: ticketTime };
    } catch {
      return null;
    }
  }
}

export const collabService = new CollabService();
