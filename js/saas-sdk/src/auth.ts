/**
 * Authentication client
 */

import type {
  AuthTokens,
  AuthResponse,
  RegisterInput,
  LoginInput,
  User,
} from './types.js';

export class AuthClient {
  private baseUrl: string;
  private tokens: AuthTokens | null = null;
  private tokenExpiry: number | null = null;
  private refreshTimer: NodeJS.Timeout | null = null;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
    this.loadTokensFromStorage();
  }

  /**
   * Register a new user
   */
  async register(input: RegisterInput): Promise<AuthResponse> {
    const response = await fetch(`${this.baseUrl}/auth/register`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(input),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.code || 'REGISTRATION_FAILED');
    }

    const data: AuthResponse = await response.json();
    this.setTokens(data);
    return data;
  }

  /**
   * Login with email/password
   */
  async login(input: LoginInput): Promise<AuthResponse> {
    const response = await fetch(`${this.baseUrl}/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(input),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.code || 'LOGIN_FAILED');
    }

    const data: AuthResponse = await response.json();
    this.setTokens(data);
    return data;
  }

  /**
   * Refresh access token
   */
  async refresh(): Promise<AuthResponse> {
    if (!this.tokens?.refreshToken) {
      throw new Error('NO_REFRESH_TOKEN');
    }

    const response = await fetch(`${this.baseUrl}/auth/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refreshToken: this.tokens.refreshToken }),
    });

    if (!response.ok) {
      this.clearTokens();
      throw new Error('REFRESH_FAILED');
    }

    const data: AuthResponse = await response.json();
    this.setTokens(data);
    return data;
  }

  /**
   * Logout
   */
  async logout(): Promise<void> {
    if (!this.tokens?.refreshToken) {
      this.clearTokens();
      return;
    }

    try {
      await fetch(`${this.baseUrl}/auth/logout`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${this.tokens.accessToken}`,
        },
        body: JSON.stringify({ refreshToken: this.tokens.refreshToken }),
      });
    } finally {
      this.clearTokens();
    }
  }

  /**
   * Get current user info
   */
  async me(): Promise<User> {
    const response = await this.authenticatedFetch(`${this.baseUrl}/auth/me`);
    if (!response.ok) {
      throw new Error('GET_USER_FAILED');
    }
    return response.json();
  }

  /**
   * Get OAuth URL for Google
   */
  getGoogleOAuthUrl(redirectUri: string): string {
    return `${this.baseUrl}/auth/oauth/google?redirect_uri=${encodeURIComponent(redirectUri)}`;
  }

  /**
   * Handle OAuth callback (extract tokens from URL hash)
   */
  handleOAuthCallback(): AuthResponse | null {
    if (typeof window === 'undefined') return null;

    const hash = window.location.hash.substring(1);
    const params = new URLSearchParams(hash);

    const accessToken = params.get('access_token');
    const refreshToken = params.get('refresh_token');

    if (accessToken && refreshToken) {
      const data: AuthResponse = {
        accessToken,
        refreshToken,
        expiresIn: 900, // 15 minutes default
        user: {} as any, // Will be populated by /auth/me call
      };
      this.setTokens(data);
      // Clear hash from URL
      window.location.hash = '';
      return data;
    }

    return null;
  }

  /**
   * Get access token (auto-refresh if needed)
   */
  async getAccessToken(): Promise<string> {
    if (!this.tokens || !this.tokenExpiry || Date.now() >= this.tokenExpiry) {
      await this.refresh();
    }
    return this.tokens!.accessToken;
  }

  /**
   * Check if user is authenticated
   */
  isAuthenticated(): boolean {
    return this.tokens !== null && (this.tokenExpiry === null || Date.now() < this.tokenExpiry);
  }

  /**
   * Make authenticated fetch request
   */
  async authenticatedFetch(url: RequestInfo | URL, options?: RequestInit): Promise<Response> {
    const token = await this.getAccessToken();
    return fetch(url, {
      ...options,
      headers: {
        ...options?.headers,
        'Authorization': `Bearer ${token}`,
      },
    });
  }

  /**
   * Set tokens and start auto-refresh timer
   */
  private setTokens(data: AuthResponse): void {
    this.tokens = {
      accessToken: data.accessToken,
      refreshToken: data.refreshToken,
      expiresIn: data.expiresIn,
    };
    this.tokenExpiry = Date.now() + (data.expiresIn * 1000) - 60000; // Refresh 1 minute early

    // Save to localStorage
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('pm4wasm_auth', JSON.stringify({
        ...this.tokens,
        expiry: this.tokenExpiry,
      }));
    }

    // Start auto-refresh timer
    this.startRefreshTimer();
  }

  /**
   * Clear tokens
   */
  private clearTokens(): void {
    this.tokens = null;
    this.tokenExpiry = null;

    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
      this.refreshTimer = null;
    }

    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem('pm4wasm_auth');
    }
  }

  /**
   * Load tokens from localStorage
   */
  private loadTokensFromStorage(): void {
    if (typeof localStorage === 'undefined') return;

    const stored = localStorage.getItem('pm4wasm_auth');
    if (stored) {
      try {
        const data = JSON.parse(stored);
        this.tokens = {
          accessToken: data.accessToken,
          refreshToken: data.refreshToken,
          expiresIn: data.expiresIn,
        };
        this.tokenExpiry = data.expiry;

        if (this.tokenExpiry && Date.now() < this.tokenExpiry) {
          this.startRefreshTimer();
        } else {
          this.clearTokens();
        }
      } catch {
        this.clearTokens();
      }
    }
  }

  /**
   * Start auto-refresh timer
   */
  private startRefreshTimer(): void {
    if (this.refreshTimer) {
      clearTimeout(this.refreshTimer);
    }

    if (!this.tokenExpiry) return;

    const refreshIn = Math.max(0, this.tokenExpiry - Date.now());
    this.refreshTimer = setTimeout(() => {
      this.refresh().catch(() => {
        this.clearTokens();
        // Trigger auth state change callback if set
        if (this.onAuthChange) {
          this.onAuthChange(null);
        }
      });
    }, refreshIn);
  }

  /**
   * Auth state change callback
   */
  onAuthChange: ((user: User | null) => void) | null = null;
}
