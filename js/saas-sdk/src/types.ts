/**
 * SaaS SDK types
 */

export interface SaaSConfig {
  apiUrl: string;
  wsUrl?: string;
  clientId?: string;
}

export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
}

export interface User {
  id: string;
  email: string;
  name: string;
  avatarUrl?: string;
}

export interface Tenant {
  id: string;
  name: string;
  slug: string;
  plan: 'FREE' | 'PRO' | 'ENTERPRISE';
  role: 'OWNER' | 'ADMIN' | 'EDITOR' | 'VIEWER';
  createdAt: Date;
}

export interface Session {
  id: string;
  name: string;
  isPublic: boolean;
  createdBy: {
    id: string;
    name: string;
  };
  createdAt: Date;
  updatedAt: Date;
}

export interface UsageReport {
  periodStart: Date;
  periodEnd: Date;
  apiCalls: number;
  storageMb: number;
  sessionMinutes: number;
  activeUsers: number;
  limits: {
    apiCalls: number;
    storageMb: number;
    sessionMinutes: number;
    activeUsers: number;
  };
}

export interface AuthResponse {
  accessToken: string;
  refreshToken: string;
  user: User;
  tenant?: Tenant;
}

export interface RegisterInput {
  email: string;
  password: string;
  name: string;
}

export interface LoginInput {
  email: string;
  password: string;
}

export interface CreateTenantInput {
  name: string;
  plan?: 'FREE' | 'PRO' | 'ENTERPRISE';
}

export interface CreateSessionInput {
  tenantId: string;
  name: string;
  model: string;
  isPublic?: boolean;
}

export interface SessionUser {
  id: string;
  name: string;
  color: string;
  cursor?: {
    line: number;
    column: number;
  };
  selection?: {
    start: { line: number; column: number };
    end: { line: number; column: number };
  };
}
