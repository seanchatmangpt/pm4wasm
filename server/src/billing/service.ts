/**
 * Stripe billing service
 */

import Stripe from 'stripe';
import { prisma } from '../db/prisma.js';
import { PlanTier, PLAN_LIMITS, type CheckoutSessionInput, type PortalSessionInput } from './types.js';

const stripe = new Stripe(process.env.STRIPE_SECRET_KEY || '', {
  apiVersion: '2024-11-20.acacia',
  typescript: true,
});

const PRICE_IDS = {
  PRO: process.env.STRIPE_PRICE_ID_PRO || '',
  ENTERPRISE: process.env.STRIPE_PRICE_ID_ENTERPRISE || '',
};

export class BillingService {
  /**
   * Create a Stripe Checkout session for plan upgrade
   */
  async createCheckoutSession(tenantId: string, userId: string, input: CheckoutSessionInput): Promise<string> {
    // Verify user has access to tenant
    const membership = await prisma.tenantMember.findUnique({
      where: {
        tenantId_userId: { tenantId, userId },
      },
      include: { tenant: true },
    });

    if (!membership || membership.role === 'VIEWER') {
      throw new Error('ACCESS_DENIED');
    }

    const tenant = membership.tenant;

    // Create or get Stripe customer
    let customerId = tenant.stripeCustomerId;
    if (!customerId) {
      const customer = await stripe.customers.create({
        email: membership.role === 'OWNER' ? undefined : undefined, // Don't pre-fill for non-owners
        metadata: { tenantId },
      });
      customerId = customer.id;

      await prisma.tenant.update({
        where: { id: tenantId },
        data: { stripeCustomerId: customerId },
      });
    }

    // Map plan to price ID
    const priceId = input.planId === PlanTier.PRO ? PRICE_IDS.PRO : PRICE_IDS.ENTERPRISE;
    if (!priceId) {
      throw new Error('INVALID_PLAN');
    }

    // Create checkout session
    const session = await stripe.checkout.sessions.create({
      customer: customerId,
      mode: 'subscription',
      payment_method_types: ['card'],
      line_items: [
        {
          price: priceId,
          quantity: 1,
        },
      ],
      success_url: input.successUrl,
      cancel_url: input.cancelUrl,
      metadata: { tenantId, planId: input.planId },
    });

    return session.url!;
  }

  /**
   * Create a Stripe Customer Portal session
   */
  async createPortalSession(tenantId: string, userId: string, input: PortalSessionInput): Promise<string> {
    // Verify user has access to tenant
    const membership = await prisma.tenantMember.findUnique({
      where: {
        tenantId_userId: { tenantId, userId },
      },
      include: { tenant: true },
    });

    if (!membership || membership.role === 'VIEWER') {
      throw new Error('ACCESS_DENIED');
    }

    const customerId = membership.tenant.stripeCustomerId;
    if (!customerId) {
      throw new Error('NO_STRIPE_CUSTOMER');
    }

    const session = await stripe.billingPortal.sessions.create({
      customer: customerId,
      return_url: input.returnUrl,
    });

    return session.url;
  }

  /**
   * Handle Stripe webhook events
   */
  async handleWebhook(event: Stripe.Event): Promise<void> {
    switch (event.type) {
      case 'checkout.session.completed': {
        const session = event.data.object as Stripe.Checkout.Session;
        await this.handleCheckoutCompleted(session);
        break;
      }
      case 'customer.subscription.created':
      case 'customer.subscription.updated': {
        const subscription = event.data.object as Stripe.Subscription;
        await this.handleSubscriptionUpdated(subscription);
        break;
      }
      case 'customer.subscription.deleted': {
        const subscription = event.data.object as Stripe.Subscription;
        await this.handleSubscriptionDeleted(subscription);
        break;
      }
      case 'invoice.payment_succeeded': {
        const invoice = event.data.object as Stripe.Invoice;
        await this.handlePaymentSucceeded(invoice);
        break;
      }
      case 'invoice.payment_failed': {
        const invoice = event.data.object as Stripe.Invoice;
        await this.handlePaymentFailed(invoice);
        break;
      }
    }
  }

  /**
   * Handle checkout.session.completed event
   */
  private async handleCheckoutCompleted(session: Stripe.Checkout.Session): Promise<void> {
    const tenantId = session.metadata?.tenantId;
    if (!tenantId) return;

    const customerId = session.customer as string;

    // Update tenant with Stripe customer ID
    await prisma.tenant.update({
      where: { id: tenantId },
      data: { stripeCustomerId: customerId },
    });
  }

  /**
   * Handle subscription created/updated events
   */
  private async handleSubscriptionUpdated(subscription: Stripe.Subscription): Promise<void> {
    const customerId = subscription.customer as string;

    // Find tenant by customer ID
    const tenant = await prisma.tenant.findFirst({
      where: { stripeCustomerId: customerId },
    });

    if (!tenant) return;

    // Determine plan from subscription
    const priceId = subscription.items.data[0]?.price.id;
    let plan = 'FREE';
    if (priceId === PRICE_IDS.PRO) {
      plan = 'PRO';
    } else if (priceId === PRICE_IDS.ENTERPRISE) {
      plan = 'ENTERPRISE';
    }

    // Update tenant plan and subscription status
    await prisma.tenant.update({
      where: { id: tenant.id },
      data: {
        plan: plan as 'FREE' | 'PRO' | 'ENTERPRISE',
        stripeSubscriptionId: subscription.id,
        subscriptionStatus: subscription.status,
      },
    });
  }

  /**
   * Handle subscription deleted event
   */
  private async handleSubscriptionDeleted(subscription: Stripe.Subscription): Promise<void> {
    const customerId = subscription.customer as string;
    const tenant = await prisma.tenant.findFirst({
      where: { stripeCustomerId: customerId },
    });

    if (!tenant) return;

    // Downgrade to free plan
    await prisma.tenant.update({
      where: { id: tenant.id },
      data: {
        plan: 'FREE',
        stripeSubscriptionId: null,
        subscriptionStatus: null,
      },
    });
  }

  /**
   * Handle successful payment
   */
  private async handlePaymentSucceeded(invoice: Stripe.Invoice): Promise<void> {
    // Could trigger notifications here
    console.log(`Payment succeeded for customer ${invoice.customer}`);
  }

  /**
   * Handle failed payment
   */
  private async handlePaymentFailed(invoice: Stripe.Invoice): Promise<void> {
    const customerId = invoice.customer as string;
    const tenant = await prisma.tenant.findFirst({
      where: { stripeCustomerId: customerId },
    });

    if (tenant) {
      // Mark subscription as past due
      await prisma.tenant.update({
        where: { id: tenant.id },
        data: { subscriptionStatus: 'past_due' },
      });
    }
  }

  /**
   * Verify webhook signature
   */
  static constructWebhookEvent(payload: string, signature: string): Stripe.Event {
    return stripe.webhooks.constructEvent(
      payload,
      signature,
      process.env.STRIPE_WEBHOOK_SECRET || ''
    );
  }

  /**
   * Get plan limits for a tenant
   */
  async getPlanLimits(tenantId: string) {
    const tenant = await prisma.tenant.findUnique({
      where: { id: tenantId },
      select: { plan: true },
    });

    if (!tenant) {
      throw new Error('TENANT_NOT_FOUND');
    }

    return PLAN_LIMITS[tenant.plan as PlanTier];
  }
}

export const billingService = new BillingService();
