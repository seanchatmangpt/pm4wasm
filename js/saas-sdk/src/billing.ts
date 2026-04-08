/**
 * Billing client
 */

export class BillingClient {
  private baseUrl: string;
  private getAccessToken: () => Promise<string>;

  constructor(baseUrl: string, getAccessToken: () => Promise<string>) {
    this.baseUrl = baseUrl;
    this.getAccessToken = getAccessToken;
  }

  /**
   * Get available plans
   */
  async getPlans(): Promise<Plan[]> {
    const response = await fetch(`${this.baseUrl}/billing/plans`);
    if (!response.ok) {
      throw new Error('GET_PLANS_FAILED');
    }

    const data = await response.json();
    return data.plans;
  }

  /**
   * Create checkout session for plan upgrade
   */
  async createCheckoutSession(tenantId: string, planId: 'pro' | 'enterprise', options?: {
    successUrl?: string;
    cancelUrl?: string;
  }): Promise<string> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/billing/checkout`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify({
        tenantId,
        planId,
        successUrl: options?.successUrl || `${window.location.origin}/billing/success`,
        cancelUrl: options?.cancelUrl || `${window.location.origin}/billing/cancel`,
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.code || 'CHECKOUT_FAILED');
    }

    const data = await response.json();
    return data.checkoutUrl;
  }

  /**
   * Create customer portal session
   */
  async createPortalSession(tenantId: string, returnUrl?: string): Promise<string> {
    const token = await this.getAccessToken();
    const response = await fetch(`${this.baseUrl}/billing/portal`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify({
        tenantId,
        returnUrl: returnUrl || `${window.location.origin}/settings/billing`,
      }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.code || 'PORTAL_FAILED');
    }

    const data = await response.json();
    return data.url;
  }
}

export interface Plan {
  id: string;
  name: string;
  price: number | null;
  limits: {
    apiCalls: number;
    storageMb: number;
    sessionMinutes: number;
    activeUsers: number;
  };
}
