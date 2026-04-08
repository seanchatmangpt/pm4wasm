/**
 * Authentication types and interfaces
 */

export interface JWTPayload {
  sub: string;      // User ID
  email: string;
  name: string;
  tenantId?: string; // Optional: current tenant context
  iat?: number;
  exp?: number;
}

export interface TokenPair {
  accessToken: string;
  refreshToken: string;
  expiresIn: number;
}

export interface AuthResponse {
  accessToken: string;
  refreshToken: string;
  user: {
    id: string;
    email: string;
    name: string;
    avatarUrl?: string;
  };
  tenant?: {
    id: string;
    name: string;
    plan: string;
  };
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

export interface OAuthProfile {
  id: string;
  email: string;
  name: string;
  picture?: string;
  provider: 'google' | 'github';
}

export enum AuthError {
  INVALID_CREDENTIALS = 'INVALID_CREDENTIALS',
  USER_EXISTS = 'USER_EXISTS',
  USER_NOT_FOUND = 'USER_NOT_FOUND',
  TOKEN_EXPIRED = 'TOKEN_EXPIRED',
  TOKEN_INVALID = 'TOKEN_INVALID',
  RATE_LIMITED = 'RATE_LIMITED',
}
