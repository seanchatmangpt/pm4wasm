/**
 * Billing types and Stripe integration
 */

export enum PlanTier {
  FREE = 'free',
  PRO = 'pro',
  ENTERPRISE = 'enterprise',
}

export interface PlanLimits {
  apiCalls: number;       // -1 for unlimited
  storageMb: number;
  sessionMinutes: number;
  activeUsers: number;
  pricePerMonth?: number;
}

export const PLAN_LIMITS: Record<PlanTier, PlanLimits> = {
  [PlanTier.FREE]: {
    apiCalls: 1000,
    storageMb: 100,
    sessionMinutes: 60,
    activeUsers: 1,
  },
  [PlanTier.PRO]: {
    apiCalls: 100000,
    storageMb: 10000,
    sessionMinutes: 1000,
    activeUsers: 10,
    pricePerMonth: 29,
  },
  [PlanTier.ENTERPRISE]: {
    apiCalls: -1,
    storageMb: -1,
    sessionMinutes: -1,
    activeUsers: -1,
    pricePerMonth: null, // Custom pricing
  },
};

export interface CheckoutSessionInput {
  planId: PlanTier;
  successUrl: string;
  cancelUrl: string;
}

export interface PortalSessionInput {
  returnUrl: string;
}

export interface SubscriptionEventData {
  customerId: string;
  subscriptionId: string;
  status: string;
  priceId: string;
}
