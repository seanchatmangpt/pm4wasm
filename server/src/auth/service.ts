/**
 * Authentication service - handles registration, login, token refresh
 */

import bcrypt from 'bcrypt';
import { prisma } from '../db/prisma.js';
import type { RegisterInput, LoginInput, AuthResponse, OAuthProfile } from './types.js';
import { generateTokenPair, verifyRefreshToken } from './jwt.js';
import { AuthError } from './types.js';

const SALT_ROUNDS = 12;

export class AuthService {
  /**
   * Register a new user (creates a default tenant)
   */
  async register(input: RegisterInput): Promise<AuthResponse> {
    // Check if user exists
    const existing = await prisma.user.findUnique({
      where: { email: input.email },
    });
    if (existing) {
      throw new Error(AuthError.USER_EXISTS);
    }

    // Hash password
    const passwordHash = await bcrypt.hash(input.password, SALT_ROUNDS);

    // Create user with default tenant
    const user = await prisma.user.create({
      data: {
        email: input.email,
        name: input.name,
        password: passwordHash,
        memberships: {
          create: {
            role: 'OWNER',
            tenant: {
              create: {
                name: `${input.name}'s Workspace`,
                slug: this.generateSlug(input.email),
                plan: 'FREE',
              },
            },
          },
        },
      },
      include: {
        memberships: {
          where: { role: 'OWNER' },
          include: { tenant: true },
          take: 1,
        },
      },
    });

    const tenant = user.memberships[0]?.tenant;

    // Generate tokens
    const payload = {
      sub: user.id,
      email: user.email,
      name: user.name,
      tenantId: tenant?.id,
    };

    const tokens = generateTokenPair(payload);

    // Store refresh token
    await this.storeRefreshToken(user.id, tokens.refreshToken);

    return {
      ...tokens,
      user: {
        id: user.id,
        email: user.email,
        name: user.name,
        avatarUrl: user.avatarUrl ?? undefined,
      },
      tenant: tenant ? {
        id: tenant.id,
        name: tenant.name,
        plan: tenant.plan,
      } : undefined,
    };
  }

  /**
   * Login with email/password
   */
  async login(input: LoginInput): Promise<AuthResponse> {
    const user = await prisma.user.findUnique({
      where: { email: input.email },
      include: {
        memberships: {
          where: { role: 'OWNER' },
          include: { tenant: true },
          take: 1,
        },
      },
    });

    if (!user || !user.password) {
      throw new Error(AuthError.INVALID_CREDENTIALS);
    }

    const valid = await bcrypt.compare(input.password, user.password);
    if (!valid) {
      throw new Error(AuthError.INVALID_CREDENTIALS);
    }

    const tenant = user.memberships[0]?.tenant;

    const payload = {
      sub: user.id,
      email: user.email,
      name: user.name,
      tenantId: tenant?.id,
    };

    const tokens = generateTokenPair(payload);
    await this.storeRefreshToken(user.id, tokens.refreshToken);

    return {
      ...tokens,
      user: {
        id: user.id,
        email: user.email,
        name: user.name,
        avatarUrl: user.avatarUrl ?? undefined,
      },
      tenant: tenant ? {
        id: tenant.id,
        name: tenant.name,
        plan: tenant.plan,
      } : undefined,
    };
  }

  /**
   * Refresh access token using refresh token
   */
  async refresh(refreshToken: string): Promise<AuthResponse> {
    const payload = verifyRefreshToken(refreshToken);

    // Check if refresh token exists and is not revoked
    const stored = await prisma.refreshToken.findUnique({
      where: { token: refreshToken },
      include: { user: true },
    });

    if (!stored || stored.revokedAt || stored.expiresAt < new Date()) {
      throw new Error(AuthError.TOKEN_INVALID);
    }

    const user = stored.user;
    const memberships = await prisma.tenantMember.findMany({
      where: { userId: user.id, role: 'OWNER' },
      include: { tenant: true },
      take: 1,
    });

    const tenant = memberships[0]?.tenant;

    const newPayload = {
      sub: user.id,
      email: user.email,
      name: user.name,
      tenantId: tenant?.id,
    };

    const tokens = generateTokenPair(newPayload);

    // Revoke old token and store new one
    await prisma.refreshToken.update({
      where: { id: stored.id },
      data: { revokedAt: new Date() },
    });
    await this.storeRefreshToken(user.id, tokens.refreshToken);

    return {
      ...tokens,
      user: {
        id: user.id,
        email: user.email,
        name: user.name,
        avatarUrl: user.avatarUrl ?? undefined,
      },
      tenant: tenant ? {
        id: tenant.id,
        name: tenant.name,
        plan: tenant.plan,
      } : undefined,
    };
  }

  /**
   * Logout - revoke refresh token
   */
  async logout(refreshToken: string): Promise<void> {
    await prisma.refreshToken.updateMany({
      where: { token: refreshToken },
      data: { revokedAt: new Date() },
    });
  }

  /**
   * Handle OAuth login/signup
   */
  async oauthLogin(profile: OAuthProfile): Promise<AuthResponse> {
    // Check for existing OAuth provider
    let oauthProvider = await prisma.oAuthProvider.findUnique({
      where: {
        provider_providerId: {
          provider: profile.provider,
          providerId: profile.id,
        },
      },
      include: { user: { include: { memberships: { where: { role: 'OWNER' }, include: { tenant: true }, take: 1 } } } },
    });

    if (oauthProvider) {
      // Existing user - login
      const user = oauthProvider.user;
      const tenant = user.memberships[0]?.tenant;

      const payload = {
        sub: user.id,
        email: user.email,
        name: user.name,
        tenantId: tenant?.id,
      };

      const tokens = generateTokenPair(payload);
      await this.storeRefreshToken(user.id, tokens.refreshToken);

      return {
        ...tokens,
        user: {
          id: user.id,
          email: user.email,
          name: user.name,
          avatarUrl: user.avatarUrl ?? undefined,
        },
        tenant: tenant ? {
          id: tenant.id,
          name: tenant.name,
          plan: tenant.plan,
        } : undefined,
      };
    }

    // New user - register with OAuth
    const user = await prisma.user.create({
      data: {
        email: profile.email,
        name: profile.name,
        avatarUrl: profile.picture,
        oauthProviders: {
          create: {
            provider: profile.provider,
            providerId: profile.id,
          },
        },
        memberships: {
          create: {
            role: 'OWNER',
            tenant: {
              create: {
                name: `${profile.name}'s Workspace`,
                slug: this.generateSlug(profile.email),
                plan: 'FREE',
              },
            },
          },
        },
      },
      include: {
        memberships: {
          where: { role: 'OWNER' },
          include: { tenant: true },
          take: 1,
        },
      },
    });

    const tenant = user.memberships[0]?.tenant;

    const payload = {
      sub: user.id,
      email: user.email,
      name: user.name,
      tenantId: tenant?.id,
    };

    const tokens = generateTokenPair(payload);
    await this.storeRefreshToken(user.id, tokens.refreshToken);

    return {
      ...tokens,
      user: {
        id: user.id,
        email: user.email,
        name: user.name,
        avatarUrl: user.avatarUrl ?? undefined,
      },
      tenant: tenant ? {
        id: tenant.id,
        name: tenant.name,
        plan: tenant.plan,
      } : undefined,
    };
  }

  /**
   * Store a refresh token in the database
   */
  private async storeRefreshToken(userId: string, token: string): Promise<void> {
    const expiresAt = new Date();
    expiresAt.setDate(expiresAt.getDate() + 7); // 7 days

    await prisma.refreshToken.create({
      data: {
        token,
        userId,
        expiresAt,
      },
    });
  }

  /**
   * Generate a URL-safe slug from email
   */
  private generateSlug(email: string): string {
    const local = email.split('@')[0];
    const timestamp = Date.now().toString(36);
    return `${local}-${timestamp}`.toLowerCase().replace(/[^a-z0-9-]/g, '-');
  }
}

export const authService = new AuthService();
