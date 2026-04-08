/**
 * OAuth 2.0 integration for Google SSO
 */

import { Router } from 'express';
import passport from 'passport';
import { OAuth2Strategy } from 'passport-google-oauth20';
import { authService } from './service.js';

const router = Router();

// Configure Google OAuth strategy
passport.use('google', new OAuth2Strategy({
  clientID: process.env.GOOGLE_CLIENT_ID || '',
  clientSecret: process.env.GOOGLE_CLIENT_SECRET || '',
  callbackURL: process.env.GOOGLE_CALLBACK_URL || '/v1/auth/oauth/google/callback',
}, async (_accessToken, _refreshToken, profile, done) => {
  try {
    const user = await authService.oauthLogin({
      id: profile.id,
      email: profile.emails![0].value,
      name: profile.displayName,
      picture: profile.photos?.[0]?.value,
      provider: 'google',
    });
    done(null, user);
  } catch (error) {
    done(error);
  }
}));

// Serialize/deserialize user for session
passport.serializeUser((user: any, done) => done(null, user));
passport.deserializeUser((user: any, done) => done(null, user));

/**
 * GET /auth/oauth/google
 * Initiate Google OAuth flow
 */
router.get('/google',
  (req, res, next) => {
    const redirectUri = req.query.redirect_uri as string;
    if (redirectUri) {
      // Store redirect URI in session for callback
      req.session = req.session || {} as any;
      (req.session as any).oauthRedirect = redirectUri;
    }
    next();
  },
  passport.authenticate('google', { scope: ['profile', 'email'] })
);

/**
 * GET /auth/oauth/google/callback
 * Google OAuth callback
 */
router.get('/google/callback',
  passport.authenticate('google', { session: false }),
  (req, res) => {
    const authResponse = req.user as any;
    const redirect = (req.session as any)?.oauthRedirect || process.env.FRONTEND_URL || '/';

    // Redirect to frontend with tokens in hash (client-side will extract)
    const params = new URLSearchParams({
      access_token: authResponse.accessToken,
      refresh_token: authResponse.refreshToken,
    });

    res.redirect(`${redirect}#/auth/callback?${params.toString()}`);
  }
);

export { router as oauthRouter };
