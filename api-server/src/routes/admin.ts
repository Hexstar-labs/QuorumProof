import express, { Request, Response } from 'express';
import { rbac } from '../middleware/rbac.js';
import {
  getWhitelist,
  addToWhitelist,
  removeFromWhitelist,
  clearWhitelist,
  setProtectedEndpoints,
  getAccessLog,
  clearAccessLog,
  getAccessStats,
  getConfigStatus,
} from '../middleware/ipWhitelist.js';

const router = express.Router();

/**
 * Admin endpoints for IP-based access control management
 * All endpoints require admin:all permission
 */

/**
 * GET /api/admin/ip-whitelist
 * Get current IP whitelist
 */
router.get('/ip-whitelist', rbac.requirePermission('admin:all'), (_req: Request, res: Response) => {
  try {
    const whitelist = getWhitelist();
    res.json({
      whitelist,
      count: whitelist.length,
    });
  } catch (error) {
    console.error('[Admin] Error fetching whitelist:', error);
    res.status(500).json({ error: 'Failed to fetch whitelist' });
  }
});

/**
 * POST /api/admin/ip-whitelist
 * Add an IP to the whitelist
 * Body: { ip: string }
 */
router.post('/ip-whitelist', rbac.requirePermission('admin:all'), (req: Request, res: Response) => {
  try {
    const { ip } = req.body;

    if (!ip || typeof ip !== 'string') {
      return res.status(400).json({ error: 'IP address is required' });
    }

    addToWhitelist(ip);
    res.json({
      success: true,
      message: `IP ${ip} added to whitelist`,
      ip,
    });
  } catch (error) {
    console.error('[Admin] Error adding to whitelist:', error);
    res.status(500).json({ error: 'Failed to add IP to whitelist' });
  }
});

/**
 * DELETE /api/admin/ip-whitelist/:ip
 * Remove an IP from the whitelist
 */
router.delete('/ip-whitelist/:ip', rbac.requirePermission('admin:all'), (req: Request, res: Response) => {
  try {
    const { ip } = req.params;

    if (!ip) {
      return res.status(400).json({ error: 'IP address is required' });
    }

    const wasRemoved = removeFromWhitelist(decodeURIComponent(ip));

    if (!wasRemoved) {
      return res.status(404).json({ error: 'IP not found in whitelist' });
    }

    res.json({
      success: true,
      message: `IP ${ip} removed from whitelist`,
      ip,
    });
  } catch (error) {
    console.error('[Admin] Error removing from whitelist:', error);
    res.status(500).json({ error: 'Failed to remove IP from whitelist' });
  }
});

/**
 * DELETE /api/admin/ip-whitelist
 * Clear entire whitelist
 */
router.delete('/ip-whitelist', rbac.requirePermission('admin:all'), (_req: Request, res: Response) => {
  try {
    clearWhitelist();
    res.json({
      success: true,
      message: 'Whitelist cleared',
    });
  } catch (error) {
    console.error('[Admin] Error clearing whitelist:', error);
    res.status(500).json({ error: 'Failed to clear whitelist' });
  }
});

/**
 * GET /api/admin/ip-acl/config
 * Get IP ACL configuration status
 */
router.get('/ip-acl/config', rbac.requirePermission('admin:all'), (_req: Request, res: Response) => {
  try {
    const status = getConfigStatus();
    res.json(status);
  } catch (error) {
    console.error('[Admin] Error fetching config:', error);
    res.status(500).json({ error: 'Failed to fetch configuration' });
  }
});

/**
 * GET /api/admin/ip-acl/access-log
 * Get access log with optional filtering
 * Query params: clientIP, endpoint, action (allowed|denied), since (ISO date)
 */
router.get('/ip-acl/access-log', rbac.requirePermission('admin:all'), (req: Request, res: Response) => {
  try {
    const { clientIP, endpoint, action, since } = req.query;

    const filters: any = {};
    if (clientIP) filters.clientIP = String(clientIP);
    if (endpoint) filters.endpoint = String(endpoint);
    if (action && (action === 'allowed' || action === 'denied')) {
      filters.action = action;
    }
    if (since) {
      const sinceDate = new Date(String(since));
      if (!isNaN(sinceDate.getTime())) {
        filters.since = sinceDate;
      }
    }

    const logs = getAccessLog(filters);
    res.json({
      count: logs.length,
      logs,
    });
  } catch (error) {
    console.error('[Admin] Error fetching access log:', error);
    res.status(500).json({ error: 'Failed to fetch access log' });
  }
});

/**
 * DELETE /api/admin/ip-acl/access-log
 * Clear access log
 */
router.delete('/ip-acl/access-log', rbac.requirePermission('admin:all'), (_req: Request, res: Response) => {
  try {
    clearAccessLog();
    res.json({
      success: true,
      message: 'Access log cleared',
    });
  } catch (error) {
    console.error('[Admin] Error clearing access log:', error);
    res.status(500).json({ error: 'Failed to clear access log' });
  }
});

/**
 * GET /api/admin/ip-acl/stats
 * Get access statistics and denial summary
 */
router.get('/ip-acl/stats', rbac.requirePermission('admin:all'), (_req: Request, res: Response) => {
  try {
    const stats = getAccessStats();
    res.json(stats);
  } catch (error) {
    console.error('[Admin] Error fetching stats:', error);
    res.status(500).json({ error: 'Failed to fetch statistics' });
  }
});

/**
 * POST /api/admin/ip-acl/protected-endpoints
 * Update the list of protected endpoints
 * Body: { endpoints: string[] }
 */
router.post('/ip-acl/protected-endpoints', rbac.requirePermission('admin:all'), (req: Request, res: Response) => {
  try {
    const { endpoints } = req.body;

    if (!Array.isArray(endpoints)) {
      return res.status(400).json({ error: 'endpoints must be an array of strings' });
    }

    if (endpoints.some((e: any) => typeof e !== 'string')) {
      return res.status(400).json({ error: 'all endpoints must be strings' });
    }

    setProtectedEndpoints(endpoints);
    res.json({
      success: true,
      message: 'Protected endpoints updated',
      endpoints,
    });
  } catch (error) {
    console.error('[Admin] Error updating protected endpoints:', error);
    res.status(500).json({ error: 'Failed to update protected endpoints' });
  }
});

export default router;
