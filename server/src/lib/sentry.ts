/**
 * Sentry error tracking (optional)
 */

export function init() {
  if (!process.env.SENTRY_DSN) {
    return;
  }

  // Lazy import Sentry to avoid issues when not configured
  import('@sentry/node').then(Sentry => {
    Sentry.init({
      dsn: process.env.SENTRY_DSN,
      environment: process.env.NODE_ENV || 'development',
      tracesSampleRate: 0.1,
      beforeSend(event) {
        // Filter out sensitive data
        if (event.request) {
          delete event.request.cookies;
          delete event.request.headers;
        }
        return event;
      },
    });
    console.log('Sentry initialized');
  }).catch(err => {
    console.error('Failed to initialize Sentry:', err);
  });
}
