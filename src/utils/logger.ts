/**
 * Enhanced logging utility that ensures errors are visible in Metro bundler
 * All logs appear in your PC terminal where npm start is running
 */

const isDev = __DEV__;

export const logger = {
  /**
   * Log info messages (appears in Metro terminal)
   */
  info: (...args: any[]) => {
    if (isDev) {
      console.log('ℹ️ [INFO]', ...args);
    }
  },

  /**
   * Log warnings (appears in Metro terminal in YELLOW)
   */
  warn: (...args: any[]) => {
    if (isDev) {
      console.warn('⚠️ [WARN]', ...args);
    }
  },

  /**
   * Log errors (appears in Metro terminal in RED)
   */
  error: (...args: any[]) => {
    console.error('❌ [ERROR]', ...args);
    
    // Also log stack trace if available
    if (args[0] instanceof Error) {
      console.error('Stack:', args[0].stack);
    }
  },

  /**
   * Log debug messages
   */
  debug: (...args: any[]) => {
    if (isDev) {
      console.log('🐛 [DEBUG]', ...args);
    }
  },

  /**
   * Log component lifecycle
   */
  component: (componentName: string, action: string, data?: any) => {
    if (isDev) {
      console.log(`📱 [${componentName}] ${action}`, data || '');
    }
  },
};

export default logger;
