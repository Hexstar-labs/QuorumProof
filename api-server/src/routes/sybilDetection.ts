/**
 * Sybil Attack Detection Routes (Issue #1579)
 *
 * Endpoints for Sybil detection, scoring, and alert management
 */

import { Router, Request, Response } from 'express';
import { SybilDetectionService } from '../services/sybilDetection.js';
import { problemJson } from '../middleware/problemDetails.js';

const router = Router();
const sybilService = new SybilDetectionService();

/**
 * POST /api/sybil/record-event
 * Records a credential creation event for Sybil analysis
 */
router.post('/record-event', (req: Request, res: Response) => {
  try {
    const { address } = req.body as Record<string, unknown>;

    if (!address || typeof address !== 'string') {
      res.status(400).json(problemJson(400, 'invalid-parameter', 'address is required'));
      return;
    }

    sybilService.recordCredentialCreation(address);

    const sybilScore = sybilService.getSybilScore(address);
    const alerts = sybilService.getAlertsForAddress(address);

    res.status(200).json({
      ok: true,
      data: {
        address,
        sybil_score: sybilScore,
        active_alerts: alerts,
      },
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/sybil/score/:address
 * Gets the Sybil score for an address
 */
router.get('/score/:address', (req: Request, res: Response) => {
  try {
    const address = (req.params.address || '') as string;

    const sybilScore = sybilService.getSybilScore(address);

    if (!sybilScore) {
      res.status(404).json(problemJson(404, 'not-found', `No Sybil score found for address: ${address}`));
      return;
    }

    res.json({ ok: true, data: sybilScore });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/sybil/activity/:address
 * Gets the activity history for an address
 */
router.get('/activity/:address', (req: Request, res: Response) => {
  try {
    const address = (req.params.address || '') as string;

    const activity = sybilService.getAccountActivity(address);

    if (!activity) {
      res.status(404).json(problemJson(404, 'not-found', `No activity found for address: ${address}`));
      return;
    }

    res.json({ ok: true, data: activity });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/sybil/alerts/:address
 * Gets active alerts for an address
 */
router.get('/alerts/:address', (req: Request, res: Response) => {
  try {
    const address = (req.params.address || '') as string;

    const alerts = sybilService.getAlertsForAddress(address);

    res.json({
      ok: true,
      data: alerts,
      count: alerts.length,
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * POST /api/sybil/dismiss-alert/:alert_id
 * Dismisses an anomaly alert
 */
router.post('/dismiss-alert/:alert_id', (req: Request, res: Response) => {
  try {
    const alert_id = (req.params.alert_id || '') as string;

    const dismissed = sybilService.dismissAlert(alert_id);

    if (!dismissed) {
      res.status(404).json(problemJson(404, 'not-found', `Alert ${alert_id} not found`));
      return;
    }

    res.json({ ok: true, message: 'Alert dismissed successfully' });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/sybil/high-risk-accounts
 * Gets all high-risk accounts based on Sybil scoring
 */
router.get('/high-risk-accounts', (req: Request, res: Response) => {
  try {
    const highRiskAccounts = sybilService.getHighRiskAccounts();

    res.json({
      ok: true,
      data: highRiskAccounts,
      count: highRiskAccounts.length,
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

/**
 * GET /api/sybil/all-active-alerts
 * Gets all active alerts in the system
 */
router.get('/all-active-alerts', (req: Request, res: Response) => {
  try {
    const alerts = sybilService.getAllActiveAlerts();

    res.json({
      ok: true,
      data: alerts,
      count: alerts.length,
    });
  } catch (error: unknown) {
    const msg = error instanceof Error ? error.message : String(error);
    res.status(400).json(problemJson(400, 'error', msg));
  }
});

export default router;
