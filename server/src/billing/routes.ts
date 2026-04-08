/**
 * Billing routes
 */

import { Router, Request } from 'express';
import { body, param, validationResult } from 'express-validator';
import { billingService } from './service.js';
import { authenticate } from '../auth/middleware.js';
import { PlanTier } from './types.js';

export const billingRouter = Router();

/**
 * POST /billing/checkout
 * Create Stripe Checkout session
 */
billingRouter.post('/checkout',
  authenticate,
  [
    body('tenantId').isUUID(),
    body('planId').isIn(['pro', 'enterprise']),
    body('successUrl').isURL(),
    body('cancelUrl').isURL().optional(),
  ],
  async (req: Request & { user?: { id: string } }, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const checkoutUrl = await billingService.createCheckoutSession(
        req.body.tenantId,
        req.user!.id,
        {
          planId: req.body.planId === 'pro' ? PlanTier.PRO : PlanTier.ENTERPRISE,
          successUrl: req.body.successUrl,
          cancelUrl: req.body.cancelUrl || process.env.FRONTEND_CANCEL_URL || '/',
        }
      );

      res.json({ checkoutUrl });
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Insufficient permissions' });
      }
      if (error instanceof Error && error.message === 'INVALID_PLAN') {
        return res.status(400).json({ code: 'INVALID_PLAN', message: 'Invalid plan selected' });
      }
      next(error);
    }
  }
);

/**
 * POST /billing/portal
 * Create Stripe Customer Portal session
 */
billingRouter.post('/portal',
  authenticate,
  [
    body('tenantId').isUUID(),
    body('returnUrl').isURL(),
  ],
  async (req: Request & { user?: { id: string } }, res, next) => {
    const errors = validationResult(req);
    if (!errors.isEmpty()) {
      return res.status(400).json({ code: 'VALIDATION_ERROR', errors: errors.array() });
    }

    try {
      const portalUrl = await billingService.createPortalSession(
        req.body.tenantId,
        req.user!.id,
        { returnUrl: req.body.returnUrl }
      );

      res.json({ url: portalUrl });
    } catch (error) {
      if (error instanceof Error && error.message === 'ACCESS_DENIED') {
        return res.status(403).json({ code: 'ACCESS_DENIED', message: 'Insufficient permissions' });
      }
      if (error instanceof Error && error.message === 'NO_STRIPE_CUSTOMER') {
        return res.status(400).json({ code: 'NO_STRIPE_CUSTOMER', message: 'No billing account found' });
      }
      next(error);
    }
  }
);

/**
 * GET /billing/plans
 * List available plans
 */
billingRouter.get('/plans', (req, res) => {
  const plans = [
    {
      id: 'free',
      name: 'Free',
      price: 0,
      limits: {
        apiCalls: 1000,
        storageMb: 100,
        sessionMinutes: 60,
        activeUsers: 1,
      },
    },
    {
      id: 'pro',
      name: 'Pro',
      price: 29,
      limits: {
        apiCalls: 100000,
        storageMb: 10000,
        sessionMinutes: 1000,
        activeUsers: 10,
      },
    },
    {
      id: 'enterprise',
      name: 'Enterprise',
      price: null,
      limits: {
        apiCalls: -1,
        storageMb: -1,
        sessionMinutes: -1,
        activeUsers: -1,
      },
    },
  ];

  res.json({ plans });
});

/**
 * POST /webhooks/stripe
 * Handle Stripe webhooks
 */
export async function handleStripeWebhook(req: Request, res: any, next: any) {
  const sig = req.headers['stripe-signature'] as string;
  if (!sig) {
    return res.status(400).json({ code: 'INVALID_SIGNATURE', message: 'Missing Stripe signature' });
  }

  try {
    const event = billingService['constructor'].constructWebhookEvent(req.body, sig);
    await billingService.handleWebhook(event);
    res.json({ received: true });
  } catch (error) {
    if (error instanceof Error) {
      console.error('Webhook error:', error.message);
      return res.status(400).json({ code: 'WEBHOOK_ERROR', message: error.message });
    }
    next(error);
  }
}
