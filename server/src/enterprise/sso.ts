/**
 * PM4Wasm SaaS – SSO (Single Sign-On)
 * Copyright (C) 2026 Process Intelligence Solutions GmbH
 *
 * Enterprise SSO integration: SAML, OIDC, SCIM provisioning.
 */

import { PrismaClient } from '../db/prisma';
import { logger } from '../lib/logger';

/**
 * Supported SSO protocols
 */
export enum SSOProtocol {
  SAML = 'saml',
  OIDC = 'oidc',
  SCIM = 'scim',
}

/**
 * SSO provider types
 */
export enum SSOProvider {
  OKTA = 'okta',
  AZURE_AD = 'azure_ad',
  AZURE_B2C = 'azure_b2c',
  GOOGLE_WORKSPACE = 'google_workspace',
  ONELOGIN = 'onelogin',
  PING_IDENTITY = 'ping_identity',
  AUTH0 = 'auth0',
  KEYCLOAK = 'keycloak',
  CUSTOM = 'custom',
}

/**
 * SSO configuration
 */
export interface SSOConfig {
  provider: SSOProvider;
  protocol: SSOProtocol;
  enabled: boolean;
  config: {
    // SAML
    samlEntryPoint?: string;
    samlCert?: string;
    samlIssuer?: string;

    // OIDC
    oidcDiscoveryUrl?: string;
    oidcClientId?: string;
    oidcClientSecret?: string;
    oidcScope?: string[];

    // SCIM
    scimBaseUrl?: string;
    scimBearerToken?: string;

    // Common
    domain?: string; // For email domain matching
    attributeMapping?: AttributeMapping;
  };
}

/**
 * Attribute mapping for SSO
 */
export interface AttributeMapping {
  email: string;
  firstName: string;
  lastName: string;
  department?: string;
  title?: string;
  groups?: string;
  role?: string;
}

/**
 * Sync direction for provisioning
 */
export enum SSODirection {
  INBOUND = 'inbound', // Identity Provider → pm4wasm
  OUTBOUND = 'outbound', // pm4wasm → Identity Provider
  BIDIRECTIONAL = 'bidirectional',
}

/**
 * SCIM user resource
 */
export interface SCIMUser {
  id: string;
  userName: string;
  name: {
    givenName: string;
    familyName: string;
  };
  emails: Array<{
    value: string;
    primary: boolean;
    type: string;
  }>;
  active: boolean;
  department?: string;
  title?: string;
  groups?: string[];
}

/**
 * SSO sync status
 */
export interface SSOSyncStatus {
  provider: SSOProvider;
  lastSyncAt?: Date;
  lastSyncStatus: 'success' | 'error' | 'in_progress';
  usersProvisioned: number;
  usersDeprovisioned: number;
  errors: string[];
}

/**
 * SSOService manages SSO integrations
 */
export class SSOService {
  private prisma: PrismaClient;
  private configs: Map<string, SSOConfig> = new Map();

  constructor(prisma: PrismaClient) {
    this.prisma = prisma;
    this.loadConfigs();
  }

  private async loadConfigs(): Promise<void> {
    const storedConfigs = await this.prisma.sSOConfig.findMany({
      where: { enabled: true },
    });

    for (const config of storedConfigs) {
      this.configs.set(config.tenantId, config as any);
    }
  }

  /**
   * Configure SSO for a tenant
   */
  async configureSSO(
    tenantId: string,
    config: SSOConfig
  ): Promise<SSOConfig> {
    // Validate config
    this.validateConfig(config);

    // Store in database
    await this.prisma.sSOConfig.upsert({
      where: { tenantId },
      update: config as any,
      create: {
        tenantId,
        ...config,
      } as any,
    });

    this.configs.set(tenantId, config);
    logger.info(`SSO configured for tenant ${tenantId}`, {
      provider: config.provider,
      protocol: config.protocol,
    });

    return config;
  }

  /**
   * Get SSO config for tenant
   */
  async getSSOConfig(tenantId: string): Promise<SSOConfig | null> {
    const config = this.configs.get(tenantId);
    if (config) {
      return config;
    }

    const stored = await this.prisma.sSOConfig.findUnique({
      where: { tenantId },
    });

    if (stored) {
      const config = stored as any;
      this.configs.set(tenantId, config);
      return config;
    }

    return null;
  }

  /**
   * Disable SSO for tenant
   */
  async disableSSO(tenantId: string): Promise<void> {
    await this.prisma.sSOConfig.update({
      where: { tenantId },
      data: { enabled: false },
    });

    this.configs.delete(tenantId);
    logger.info(`SSO disabled for tenant ${tenantId}`);
  }

  /**
   * Handle SAML SSO login
   */
  async handleSAMLLogin(
    tenantId: string,
    samlResponse: string,
    relayState?: string
  ): Promise<{ userId: string; email: string; name: string }> {
    const config = await this.getSSOConfig(tenantId);
    if (!config || config.protocol !== SSOProtocol.SAML) {
      throw new Error('SAML not configured for tenant');
    }

    // Validate SAML response (would use passport-saml in production)
    // For now, simulate extraction
    const userData = this.extractSAMLAttributes(samlResponse, config);

    // Find or create user
    const user = await this.findOrCreateUser(tenantId, userData);

    // Log the SSO login
    logger.info(`SAML login successful`, {
      tenantId,
      userId: user.id,
      email: user.email,
    });

    return {
      userId: user.id,
      email: user.email,
      name: user.name,
    };
  }

  /**
   * Handle OIDC SSO login
   */
  async handleOIDCLogin(
    tenantId: string,
    code: string,
    redirectUri: string
  ): Promise<{ userId: string; email: string; name: string }> {
    const config = await this.getSSOConfig(tenantId);
    if (!config || config.protocol !== SSOProtocol.OIDC) {
      throw new Error('OIDC not configured for tenant');
    }

    // Exchange code for tokens (would use openid-client in production)
    const userData = await this.exchangeOIDCCode(code, redirectUri, config);

    // Find or create user
    const user = await this.findOrCreateUser(tenantId, userData);

    logger.info(`OIDC login successful`, {
      tenantId,
      userId: user.id,
      email: user.email,
    });

    return {
      userId: user.id,
      email: user.email,
      name: user.name,
    };
  }

  /**
   * SCIM: Get all users (provisioning endpoint)
   */
  async scimGetUsers(
    tenantId: string,
    startIndex: number = 1,
    count: number = 100
  ): Promise<{
    schemas: string[];
    totalResults: number;
    startIndex: number;
    itemsPerPage: number;
    Resources: SCIMUser[];
  }> {
    const config = await this.getSSOConfig(tenantId);
    if (!config) {
      throw new Error('SCIM not configured for tenant');
    }

    const members = await this.prisma.tenantMember.findMany({
      where: { tenantId },
      include: { user: true },
      skip: startIndex - 1,
      take: count,
    });

    const total = await this.prisma.tenantMember.count({
      where: { tenantId },
    });

    return {
      schemas: ['urn:ietf:params:scim:api:messages:2.0:ListResponse'],
      totalResults: total,
      startIndex,
      itemsPerPage: members.length,
      Resources: members.map(m => this.formatSCIMUser(m)),
    };
  }

  /**
   * SCIM: Create user (provisioning endpoint)
   */
  async scimCreateUser(
    tenantId: string,
    user: SCIMUser
  ): Promise<SCIMUser> {
    // Check if user exists by email
    const existing = await this.prisma.user.findUnique({
      where: { email: user.userName },
    });

    if (existing) {
      // Add to tenant if not already member
      const existingMember = await this.prisma.tenantMember.findUnique({
        where: {
          userId_tenantId: {
            userId: existing.id,
            tenantId,
          },
        },
      });

      if (!existingMember) {
        await this.prisma.tenantMember.create({
          data: {
            userId: existing.id,
            tenantId,
            roleId: await this.getDefaultRoleId(tenantId),
          },
        });
      }

      return this.formatSCIMUser({
        user: existing,
        userId: existing.id,
      });
    }

    // Create new user
    const newUser = await this.prisma.user.create({
      data: {
        email: user.userName,
        name: `${user.name.givenName} ${user.name.familyName}`,
        passwordHash: '', // SSO users don't have passwords
      },
    });

    // Add to tenant
    await this.prisma.tenantMember.create({
      data: {
        userId: newUser.id,
        tenantId,
        roleId: await this.getDefaultRoleId(tenantId),
      },
    });

    logger.info(`SCIM user created`, {
      tenantId,
      userId: newUser.id,
      email: newUser.email,
    });

    return this.formatSCIMUser({
      user: newUser,
      userId: newUser.id,
    });
  }

  /**
   * SCIM: Update user (provisioning endpoint)
   */
  async scimUpdateUser(
    tenantId: string,
    userId: string,
    attributes: Partial<SCIMUser>
  ): Promise<SCIMUser> {
    const user = await this.prisma.user.findUnique({
      where: { id: userId },
    });

    if (!user) {
      throw new Error('User not found');
    }

    const updated = await this.prisma.user.update({
      where: { id: userId },
      data: {
        ...(attributes.name?.givenName || attributes.name?.familyName
          ? {
              name: `${attributes.name?.givenName || ''} ${
                attributes.name?.familyName || ''
              }`,
            }
          : {}),
      },
    });

    logger.info(`SCIM user updated`, {
      tenantId,
      userId,
      attributes,
    });

    return this.formatSCIMUser({
      user: updated,
      userId,
    });
  }

  /**
   * SCIM: Delete user (deprovisioning endpoint)
   */
  async scimDeleteUser(tenantId: string, userId: string): Promise<void> {
    await this.prisma.tenantMember.delete({
      where: {
        userId_tenantId: {
          userId,
          tenantId,
        },
      },
    });

    logger.info(`SCIM user deprovisioned`, {
      tenantId,
      userId,
    });
  }

  /**
   * Sync users from identity provider
   */
  async syncUsers(
    tenantId: string,
    direction: SSODirection = SSODirection.INBOUND
  ): Promise<SSOSyncStatus> {
    const config = await this.getSSOConfig(tenantId);
    if (!config) {
      throw new Error('SSO not configured for tenant');
    }

    const status: SSOSyncStatus = {
      provider: config.provider,
      lastSyncStatus: 'in_progress',
      usersProvisioned: 0,
      usersDeprovisioned: 0,
      errors: [],
    };

    try {
      if (direction === SSODirection.INBOUND || direction === SSODirection.BIDIRECTIONAL) {
        // Fetch users from IdP via SCIM
        const remoteUsers = await this.fetchSCIMUsers(config);
        status.usersProvisioned = remoteUsers.length;

        for (const user of remoteUsers) {
          await this.scimCreateUser(tenantId, user);
        }
      }

      status.lastSyncStatus = 'success';
      status.lastSyncAt = new Date();

      logger.info(`SSO sync completed`, {
        tenantId,
        provider: config.provider,
        ...status,
      });

      return status;
    } catch (error) {
      status.lastSyncStatus = 'error';
      status.errors.push(error.message);
      logger.error(`SSO sync failed`, {
        tenantId,
        error: error.message,
      });
      return status;
    }
  }

  /**
   * Get sync status
   */
  async getSyncStatus(tenantId: string): Promise<SSOSyncStatus | null> {
    const config = await this.getSSOConfig(tenantId);
    if (!config) {
      return null;
    }

    // In production, fetch from database
    return {
      provider: config.provider,
      lastSyncStatus: 'success',
      usersProvisioned: 0,
      usersDeprovisioned: 0,
      errors: [],
    };
  }

  /**
   * Validate SSO configuration
   */
  private validateConfig(config: SSOConfig): void {
    if (!config.provider || !Object.values(SSOProvider).includes(config.provider)) {
      throw new Error('Invalid SSO provider');
    }

    if (!config.protocol || !Object.values(SSOProtocol).includes(config.protocol)) {
      throw new Error('Invalid SSO protocol');
    }

    if (config.protocol === SSOProtocol.SAML) {
      if (!config.config.samlEntryPoint || !config.config.samlCert) {
        throw new Error('SAML requires entryPoint and certificate');
      }
    }

    if (config.protocol === SSOProtocol.OIDC) {
      if (
        !config.config.oidcDiscoveryUrl ||
        !config.config.oidcClientId ||
        !config.config.oidcClientSecret
      ) {
        throw new Error('OIDC requires discoveryUrl, clientId, and clientSecret');
      }
    }
  }

  private extractSAMLAttributes(
    samlResponse: string,
    config: SSOConfig
  ): { email: string; firstName: string; lastName: string } {
    // In production, use passport-saml to extract attributes
    // This is a placeholder
    return {
      email: 'user@example.com',
      firstName: 'Test',
      lastName: 'User',
    };
  }

  private async exchangeOIDCCode(
    code: string,
    redirectUri: string,
    config: SSOConfig
  ): Promise<{ email: string; firstName: string; lastName: string }> {
    // In production, use openid-client to exchange code
    // This is a placeholder
    return {
      email: 'user@example.com',
      firstName: 'Test',
      lastName: 'User',
    };
  }

  private async findOrCreateUser(
    tenantId: string,
    userData: { email: string; firstName: string; lastName: string }
  ): Promise<{ id: string; email: string; name: string }> {
    let user = await this.prisma.user.findUnique({
      where: { email: userData.email },
    });

    if (!user) {
      user = await this.prisma.user.create({
        data: {
          email: userData.email,
          name: `${userData.firstName} ${userData.lastName}`,
          passwordHash: '',
        },
      });
    }

    // Add to tenant if not already member
    const existingMember = await this.prisma.tenantMember.findUnique({
      where: {
        userId_tenantId: {
          userId: user.id,
          tenantId,
        },
      },
    });

    if (!existingMember) {
      await this.prisma.tenantMember.create({
        data: {
          userId: user.id,
          tenantId,
          roleId: await this.getDefaultRoleId(tenantId),
        },
      });
    }

    return {
      id: user.id,
      email: user.email,
      name: user.name,
    };
  }

  private async getDefaultRoleId(tenantId: string): Promise<string> {
    // Find or create the default MEMBER role
    const role = await this.prisma.role.findFirst({
      where: {
        tenantId,
        name: 'Member',
      },
    });

    if (role) {
      return role.id;
    }

    const newRole = await this.prisma.role.create({
      data: {
        tenantId,
        name: 'Member',
        description: 'Default SSO member role',
        permissions: [],
        isSystem: false,
      },
    });

    return newRole.id;
  }

  private formatSCIMUser(member: any): SCIMUser {
    const nameParts = member.user?.name?.split(' ') || ['', ''];
    return {
      id: member.userId,
      userName: member.user?.email || '',
      name: {
        givenName: nameParts[0] || '',
        familyName: nameParts.slice(1).join(' ') || '',
      },
      emails: [
        {
          value: member.user?.email || '',
          primary: true,
          type: 'work',
        },
      ],
      active: true,
    };
  }

  private async fetchSCIMUsers(config: SSOConfig): Promise<SCIMUser[]> {
    // In production, fetch from IdP's SCIM endpoint
    // This is a placeholder
    return [];
  }
}
