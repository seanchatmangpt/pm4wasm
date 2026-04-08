/**
 * Tenant management client
 */

import type { Tenant, CreateTenantInput, UsageReport, Member } from './types.js';

export class TenantClient {
  private baseUrl: string;
  private getAccessToken: () => Promise<string>;

  constructor(baseUrl: string, getAccessToken: () => Promise<string>) {
    this.baseUrl = baseUrl;
    this.getAccessToken = getAccessToken;
  }

  /**
   * List user's tenants
   */
  async list(): Promise<Tenant[]> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/tenants`, {
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('LIST_TENANTS_FAILED');
    }

    const data = await response.json();
    return data.tents;
  }

  /**
   * Get a specific tenant
   */
  async get(tenantId: string): Promise<Tenant> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/tenants/${tenantId}`, {
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('GET_TENANT_FAILED');
    }

    return response.json();
  }

  /**
   * Create a new tenant
   */
  async create(input: CreateTenantInput): Promise<Tenant> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/tenants`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(input),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.code || 'CREATE_TENANT_FAILED');
    }

    return response.json();
  }

  /**
   * Update tenant
   */
  async update(tenantId: string, input: { name?: string; settings?: Record<string, unknown> }): Promise<Tenant> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/tenants/${tenantId}`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify(input),
    });

    if (!response.ok) {
      throw new Error('UPDATE_TENANT_FAILED');
    }

    return response.json();
  }

  /**
   * Delete tenant
   */
  async delete(tenantId: string): Promise<void> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/tenants/${tenantId}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('DELETE_TENANT_FAILED');
    }
  }

  /**
   * List tenant members
   */
  async listMembers(tenantId: string): Promise<Member[]> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/tenants/${tenantId}/members`, {
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('LIST_MEMBERS_FAILED');
    }

    const data = await response.json();
    return data.members;
  }

  /**
   * Add a member to the tenant
   */
  async addMember(tenantId: string, email: string, role: 'OWNER' | 'ADMIN' | 'EDITOR' | 'VIEWER'): Promise<Member> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/tenants/${tenantId}/members`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify({ email, role }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.code || 'ADD_MEMBER_FAILED');
    }

    return response.json();
  }

  /**
   * Remove a member from the tenant
   */
  async removeMember(tenantId: string, memberId: string): Promise<void> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/tenants/${tenantId}/members/${memberId}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('REMOVE_MEMBER_FAILED');
    }
  }

  /**
   * Get tenant usage
   */
  async getUsage(tenantId: string): Promise<UsageReport> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/tenants/${tenantId}/usage`, {
      headers: { 'Authorization': `Bearer ${token}` },
    });

    if (!response.ok) {
      throw new Error('GET_USAGE_FAILED');
    }

    return response.json();
  }
}
