/**
 * JWT token generation and validation
 */

import jwt from 'jsonwebtoken';
import type { JWTPayload, TokenPair } from './types.js';

const ACCESS_SECRET = process.env.JWT_ACCESS_SECRET || 'dev-access-secret-min-32-chars';
const REFRESH_SECRET = process.env.JWT_REFRESH_SECRET || 'dev-refresh-secret-min-32-chars';
const ACCESS_EXPIRY = process.env.JWT_ACCESS_EXPIRY || '15m';
const REFRESH_EXPIRY = process.env.JWT_REFRESH_EXPIRY || '7d';

/**
 * Generate an access token (short-lived)
 */
export function generateAccessToken(payload: JWTPayload): string {
  return jwt.sign(payload, ACCESS_SECRET, {
    expiresIn: ACCESS_EXPIRY,
    issuer: 'pm4wasm',
    audience: 'pm4wasm-api',
  });
}

/**
 * Generate a refresh token (long-lived)
 */
export function generateRefreshToken(payload: JWTPayload): string {
  return jwt.sign(payload, REFRESH_SECRET, {
    expiresIn: REFRESH_EXPIRY,
    issuer: 'pm4wasm',
    audience: 'pm4wasm-api',
  });
}

/**
 * Generate a token pair
 */
export function generateTokenPair(payload: JWTPayload): TokenPair {
  const accessToken = generateAccessToken(payload);
  const refreshToken = generateRefreshToken(payload);

  // Calculate expiry in seconds for client
  const decoded = jwt.decode(accessToken) as { exp: number };
  const expiresIn = decoded.exp ? decoded.exp - Math.floor(Date.now() / 1000) : 900;

  return { accessToken, refreshToken, expiresIn };
}

/**
 * Verify and decode an access token
 */
export function verifyAccessToken(token: string): JWTPayload {
  try {
    return jwt.verify(token, ACCESS_SECRET, {
      issuer: 'pm4wasm',
      audience: 'pm4wasm-api',
    }) as JWTPayload;
  } catch (error) {
    if (error instanceof jwt.TokenExpiredError) {
      throw new Error('TOKEN_EXPIRED');
    }
    if (error instanceof jwt.JsonWebTokenError) {
      throw new Error('TOKEN_INVALID');
    }
    throw error;
  }
}

/**
 * Verify and decode a refresh token
 */
export function verifyRefreshToken(token: string): JWTPayload {
  try {
    return jwt.verify(token, REFRESH_SECRET, {
      issuer: 'pm4wasm',
      audience: 'pm4wasm-api',
    }) as JWTPayload;
  } catch (error) {
    if (error instanceof jwt.TokenExpiredError) {
      throw new Error('TOKEN_EXPIRED');
    }
    if (error instanceof jwt.JsonWebTokenError) {
      throw new Error('TOKEN_INVALID');
    }
    throw error;
  }
}

/**
 * Extract token from Authorization header
 */
export function extractToken(authHeader: string | undefined): string | null {
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return null;
  }
  return authHeader.substring(7);
}
