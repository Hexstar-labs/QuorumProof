import { Request, Response, NextFunction } from 'express';

/**
 * IP-based access control middleware for sensitive endpoints
 * Implements IP whitelist with proxy detection support
 */

interface IPWhitelistConfig {
  enabledEndpoints: string[];
  whitelistedIPs: Set<string>;
  enableProxyDetection: boolean;
  proxyHeadersToCheck: string[];
  enableAuditLogging: boolean;
  logDenials: boolean;
}

interface AccessLogEntry {
  timestamp: Date;
  clientIP: string;
  forwardedIP?: string;
  endpoint: string;
  action: 'allowed' | 'denied';
  reason?: string;
}

let config: IPWhitelistConfig = {
  enabledEndpoints: [
    '/api/admin/users',
    '/api/admin/settings',
    '/api/admin/system',
    '/api/admin/credentials/revoke-batch',
    '/api/admin/credentials/export',
    '/api/admin/audit',
    '/api/admin/roles',
    '/api/admin/permissions',
    '/api/admin/recovery',
  ],
  whitelistedIPs: new Set(),
  enableProxyDetection: true,
  proxyHeadersToCheck: [
    'x-forwarded-for',
    'cf-connecting-ip', // Cloudflare
    'x-client-ip',
    'x-real-ip',
  ],
  enableAuditLogging: true,
  logDenials: true,
};

// In-memory access log (in production, this would go to a database/logging service)
const accessLog: AccessLogEntry[] = [];

/**
 * Extract the client's real IP address, checking for proxy headers
 */
function getClientIP(req: Request): { clientIP: string; forwardedIP?: string } {
  let clientIP = req.ip || req.socket.remoteAddress || 'unknown';
  let forwardedIP: string | undefined;

  if (config.enableProxyDetection) {
    for (const header of config.proxyHeadersToCheck) {
      const headerValue = req.get(header);
      if (headerValue) {
        // X-Forwarded-For can contain multiple IPs, take the first
        const ips = headerValue.split(',').map((ip) => ip.trim());
        if (ips.length > 0) {
          forwardedIP = ips[0];
          clientIP = ips[0];
          break;
        }
      }
    }
  }

  // Normalize IPv6 loopback
  if (clientIP === '::1') clientIP = '127.0.0.1';
  if (clientIP === '::ffff:127.0.0.1') clientIP = '127.0.0.1';

  return { clientIP, forwardedIP };
}

/**
 * Check if an IP is in the whitelist (supports CIDR notation)
 */
function isIPWhitelisted(ip: string): boolean {
  if (config.whitelistedIPs.has(ip)) {
    return true;
  }

  // Check CIDR ranges if they're stored as "x.x.x.x/24" format
  for (const whitelistedEntry of config.whitelistedIPs) {
    if (whitelistedEntry.includes('/')) {
      if (isIPInCIDR(ip, whitelistedEntry)) {
        return true;
      }
    }
  }

  return false;
}

/**
 * Simple CIDR range check
 */
function isIPInCIDR(ip: string, cidr: string): boolean {
  try {
    const [range, mask] = cidr.split('/');
    if (!mask) return false;

    const maskBits = parseInt(mask, 10);
    const ipParts = ip.split('.').map(Number);
    const rangeParts = range.split('.').map(Number);

    if (ipParts.length !== 4 || rangeParts.length !== 4) {
      return false;
    }

    const ipNum = (ipParts[0] << 24) + (ipParts[1] << 16) + (ipParts[2] << 8) + ipParts[3];
    const rangeNum = (rangeParts[0] << 24) + (rangeParts[1] << 16) + (rangeParts[2] << 8) + rangeParts[3];
    const maskNum = (-1 << (32 - maskBits)) >>> 0;

    return (ipNum & maskNum) === (rangeNum & maskNum);
  } catch {
    return false;
  }
}

/**
 * Check if an endpoint should be protected
 */
function isProtectedEndpoint(path: string): boolean {
  return config.enabledEndpoints.some(endpoint => path.startsWith(endpoint));
}

/**
 * Log access attempt
 */
function logAccess(entry: AccessLogEntry): void {
  if (!config.enableAuditLogging) return;
  if (entry.action === 'denied' && !config.logDenials) return;

  accessLog.push(entry);

  // Log to console in production this would go to a proper logging service
  const logLevel = entry.action === 'denied' ? 'WARN' : 'INFO';
  console.log(
    `[IP_ACL] ${logLevel}: ${entry.action.toUpperCase()} ${entry.endpoint} from ${entry.clientIP}${entry.forwardedIP ? ` (forwarded: ${entry.forwardedIP})` : ''} - ${entry.reason || ''}`
  );
}

/**
 * Main middleware function
 */
export function createIPWhitelistMiddleware() {
  return (req: Request, res: Response, next: NextFunction) => {
    const path = req.path;

    if (!isProtectedEndpoint(path)) {
      return next();
    }

    const { clientIP, forwardedIP } = getClientIP(req);

    if (!isIPWhitelisted(clientIP)) {
      logAccess({
        timestamp: new Date(),
        clientIP,
        forwardedIP,
        endpoint: path,
        action: 'denied',
        reason: 'IP not in whitelist',
      });

      return res.status(403).json({
        error: 'Forbidden',
        message: 'Your IP address is not authorized to access this endpoint',
        code: 'IP_NOT_WHITELISTED',
      });
    }

    logAccess({
      timestamp: new Date(),
      clientIP,
      forwardedIP,
      endpoint: path,
      action: 'allowed',
    });

    next();
  };
}

/**
 * Get the current whitelist
 */
export function getWhitelist(): string[] {
  return Array.from(config.whitelistedIPs);
}

/**
 * Add an IP to the whitelist
 */
export function addToWhitelist(ip: string): void {
  if (!ip) throw new Error('IP cannot be empty');
  config.whitelistedIPs.add(ip);
  console.log(`[IP_ACL] Added ${ip} to whitelist`);
}

/**
 * Remove an IP from the whitelist
 */
export function removeFromWhitelist(ip: string): boolean {
  const wasPresent = config.whitelistedIPs.has(ip);
  if (wasPresent) {
    config.whitelistedIPs.delete(ip);
    console.log(`[IP_ACL] Removed ${ip} from whitelist`);
  }
  return wasPresent;
}

/**
 * Clear the entire whitelist
 */
export function clearWhitelist(): void {
  config.whitelistedIPs.clear();
  console.log('[IP_ACL] Cleared whitelist');
}

/**
 * Update the protected endpoints list
 */
export function setProtectedEndpoints(endpoints: string[]): void {
  config.enabledEndpoints = endpoints;
  console.log(`[IP_ACL] Updated protected endpoints: ${endpoints.join(', ')}`);
}

/**
 * Get access log entries with optional filtering
 */
export function getAccessLog(filters?: {
  clientIP?: string;
  endpoint?: string;
  action?: 'allowed' | 'denied';
  since?: Date;
}): AccessLogEntry[] {
  return accessLog.filter(entry => {
    if (filters?.clientIP && entry.clientIP !== filters.clientIP) return false;
    if (filters?.endpoint && !entry.endpoint.startsWith(filters.endpoint)) return false;
    if (filters?.action && entry.action !== filters.action) return false;
    if (filters?.since && entry.timestamp < filters.since) return false;
    return true;
  });
}

/**
 * Clear access log
 */
export function clearAccessLog(): void {
  accessLog.length = 0;
  console.log('[IP_ACL] Access log cleared');
}

/**
 * Get access statistics
 */
export function getAccessStats() {
  const totalAccess = accessLog.length;
  const allowedAccess = accessLog.filter(e => e.action === 'allowed').length;
  const deniedAccess = accessLog.filter(e => e.action === 'denied').length;

  const uniqueIPs = new Set(accessLog.map(e => e.clientIP));
  const deniedIPs = new Set(
    accessLog.filter(e => e.action === 'denied').map(e => e.clientIP)
  );

  return {
    totalAccess,
    allowedAccess,
    deniedAccess,
    denialRate: totalAccess > 0 ? (deniedAccess / totalAccess * 100).toFixed(2) + '%' : '0%',
    uniqueIPs: uniqueIPs.size,
    deniedIPs: Array.from(deniedIPs),
  };
}

/**
 * Set IP whitelist configuration from environment or config
 */
export function initializeWhitelist(ips: string[], protectedEndpoints?: string[]): void {
  config.whitelistedIPs.clear();
  ips.forEach(ip => config.whitelistedIPs.add(ip));

  if (protectedEndpoints) {
    config.enabledEndpoints = protectedEndpoints;
  }

  console.log(`[IP_ACL] Initialized with ${ips.length} whitelisted IPs`);
}

/**
 * Load whitelist from environment variables
 * Format: IP_WHITELIST="192.168.1.0/24,10.0.0.1,127.0.0.1"
 */
export function loadWhitelistFromEnv(): void {
  const envWhitelist = process.env.IP_WHITELIST || '';
  if (!envWhitelist) {
    console.log('[IP_ACL] No IP_WHITELIST environment variable found');
    return;
  }

  const ips = envWhitelist.split(',').map(ip => ip.trim()).filter(ip => ip);
  initializeWhitelist(ips);
}

/**
 * Get configuration status
 */
export function getConfigStatus() {
  return {
    enabledEndpoints: config.enabledEndpoints,
    whitelistSize: config.whitelistedIPs.size,
    proxyDetectionEnabled: config.enableProxyDetection,
    auditLoggingEnabled: config.enableAuditLogging,
    denialLoggingEnabled: config.logDenials,
  };
}
