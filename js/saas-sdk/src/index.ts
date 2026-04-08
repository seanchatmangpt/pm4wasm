/**
 * pm4wasm SaaS SDK
 *
 * Browser-native SDK for the pm4wasm SaaS platform.
 * Handles authentication, tenant management, collaboration, and billing.
 *
 * @example
 * ```ts
 * import { Pm4wasmSaaS } from '@pm4wasm/saas-sdk';
 *
 * const sdk = new Pm4wasmSaaS({
 *   apiUrl: 'https://api.pm4py.org/v1',
 * });
 *
 * // Login
 * await sdk.auth.login({ email: 'user@example.com', password: 'password' });
 *
 * // List tenants
 * const tenants = await sdk.tenants.list();
 *
 * // Create a collaborative session
 * const session = await sdk.collab.create({
 *   tenantId: tenants[0].id,
 *   name: 'My Process Model',
 *   model: 'PO=(nodes={A, B}, order={A-->B})',
 * });
 *
 * // Join the session with real-time collaboration
 * await sdk.collab.join(session.id, {
 *   'user:joined': (user) => console.log('User joined:', user),
 *   'model:updated': (update) => console.log('Model updated:', update),
 * });
 * ```
 */

import { AuthClient } from './auth.js';
import { TenantClient } from './tenants.js';
import { CollabClient } from './collab.js';
import { BillingClient } from './billing.js';
import type { SaaSConfig, User, AuthResponse, RegisterInput, LoginInput } from './types.js';

export class Pm4wasmSaaS {
  public readonly auth: AuthClient;
  public readonly tenants: TenantClient;
  public readonly collab: CollabClient;
  public readonly billing: BillingClient;

  private config: SaaSConfig;

  constructor(config: SaaSConfig) {
    this.config = config;

    // Initialize clients
    this.auth = new AuthClient(config.apiUrl);
    this.tenants = new TenantClient(config.apiUrl, () => this.auth.getAccessToken());
    this.collab = new CollabClient(config.wsUrl || config.apiUrl, () => this.auth.getAccessToken());
    this.billing = new BillingClient(config.apiUrl, () => this.auth.getAccessToken());

    // Set up auth state change handler
    this.auth.onAuthChange = (user) => {
      // Trigger custom event for auth state changes
      if (typeof window !== 'undefined') {
        window.dispatchEvent(new CustomEvent('pm4wasm:auth', { detail: { user } }));
      }
    };
  }

  /**
   * Initialize SDK from stored tokens
   */
  async init(): Promise<User | null> {
    if (this.auth.isAuthenticated()) {
      try {
        const user = await this.auth.me();
        this.auth.onAuthChange = (u) => {
          if (typeof window !== 'undefined') {
            window.dispatchEvent(new CustomEvent('pm4wasm:auth', { detail: { user: u } }));
          }
        };
        return user;
      } catch {
        // Token might be expired, clear it
        await this.auth.logout();
        return null;
      }
    }
    return null;
  }

  /**
   * Register a new user
   */
  async register(input: RegisterInput): Promise<AuthResponse> {
    return this.auth.register(input);
  }

  /**
   * Login
   */
  async login(input: LoginInput): Promise<AuthResponse> {
    return this.auth.login(input);
  }

  /**
   * Logout
   */
  async logout(): Promise<void> {
    await this.auth.logout();
  }

  /**
   * Get current user (returns null if not authenticated)
   */
  async getCurrentUser(): Promise<User | null> {
    if (!this.auth.isAuthenticated()) {
      return null;
    }
    return this.auth.me();
  }

  /**
   * Check if authenticated
   */
  isAuthenticated(): boolean {
    return this.auth.isAuthenticated();
  }

  /**
   * Get Google OAuth URL
   */
  getGoogleOAuthUrl(redirectUri?: string): string {
    return this.auth.getGoogleOAuthUrl(
      redirectUri || `${typeof window !== 'undefined' ? window.location.origin : ''}/auth/callback`
    );
  }

  /**
   * Handle OAuth callback from redirect
   */
  handleOAuthCallback(): AuthResponse | null {
    return this.auth.handleOAuthCallback();
  }
}

// Export types
export * from './types.js';
export { AuthClient } from './auth.js';
export { TenantClient } from './tenants.js';
export { CollabClient } from './collab.js';
export { BillingClient } from './billing.js';

// Default export
export default Pm4wasmSaaS;
