import { Router, Request, Response } from 'express';
import { CredentialAuditTrail } from '../services/credentialAuditTrail.js';

/**
 * Credential Audit Routes
 * Issue #1573: Add Audit Trail for Credential Amendments
 *
 * Endpoints:
 * GET /api/credentials/:id/audit - Get full change history
 * GET /api/credentials/:id/audit/field/:field - Get field-specific history
 * GET /api/credentials/:id/audit/user/:user - Get changes by user
 * GET /api/credentials/:id/audit/report - Generate audit report
 * GET /api/credentials/:id/audit/anomalies - Detect suspicious activity
 */

export function createAuditRouter() {
  const router = Router();
  const auditTrail = new CredentialAuditTrail();

  // GET /api/credentials/:id/audit
  // Get full change history for a credential
  router.get('/:id/audit', async (req: Request, res: Response) => {
    const { id } = req.params;

    try {
      const credentialId = BigInt(id);
      const changeHistory = await auditTrail.getChangeHistory(credentialId);

      res.json({
        credential_id: id,
        summary: {
          total_changes: changeHistory.changes.length,
          integrity_verified: changeHistory.integrity_verified,
          last_verified_at: new Date(changeHistory.last_verified_at).toISOString(),
        },
        changes: changeHistory.changes.map((change) => ({
          change_id: change.change_id,
          field_name: change.field_name,
          old_value: change.old_value,
          new_value: change.new_value,
          changed_at: new Date(change.changed_at).toISOString(),
          changed_by: change.changed_by,
          change_reason: change.change_reason,
        })),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to retrieve change history',
        message,
      });
    }
  });

  // GET /api/credentials/:id/audit/field/:field
  // Get history for a specific field
  router.get('/:id/audit/field/:field', async (req: Request, res: Response) => {
    const { id, field } = req.params;

    try {
      const credentialId = BigInt(id);
      const fieldHistory = await auditTrail.getFieldHistory(credentialId, field);

      res.json({
        credential_id: id,
        field_name: field,
        total_changes: fieldHistory.length,
        changes: fieldHistory.map((change) => ({
          change_id: change.change_id,
          old_value: change.old_value,
          new_value: change.new_value,
          changed_at: new Date(change.changed_at).toISOString(),
          changed_by: change.changed_by,
          change_reason: change.change_reason,
        })),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to retrieve field history',
        message,
      });
    }
  });

  // GET /api/credentials/:id/audit/user/:user
  // Get changes made by a specific user
  router.get('/:id/audit/user/:user', async (req: Request, res: Response) => {
    const { id, user } = req.params;

    try {
      const credentialId = BigInt(id);
      const userChanges = await auditTrail.getChangesByUser(credentialId, decodeURIComponent(user));

      res.json({
        credential_id: id,
        user_address: user,
        total_changes: userChanges.length,
        changes: userChanges.map((change) => ({
          change_id: change.change_id,
          field_name: change.field_name,
          old_value: change.old_value,
          new_value: change.new_value,
          changed_at: new Date(change.changed_at).toISOString(),
          change_reason: change.change_reason,
        })),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to retrieve user changes',
        message,
      });
    }
  });

  // GET /api/credentials/:id/audit/report
  // Generate comprehensive audit report
  router.get('/:id/audit/report', async (req: Request, res: Response) => {
    const { id } = req.params;

    try {
      const credentialId = BigInt(id);
      const report = await auditTrail.generateAuditReport(credentialId);

      res.json({
        credential_id: report.credential_id,
        audit_summary: {
          total_changes: report.total_changes,
          unique_changers: report.unique_changers,
          date_range: {
            from: report.date_range.from,
            to: report.date_range.to,
          },
          integrity_status: report.integrity_status,
          anomalies_count: report.anomalies.length,
        },
        anomalies: report.anomalies.map((a) => ({
          type: a.type,
          description: a.description,
        })),
        change_summary: report.changes.map((change) => ({
          change_id: change.change_id,
          field_name: change.field_name,
          changed_at: new Date(change.changed_at).toISOString(),
          changed_by: change.changed_by,
        })),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to generate audit report',
        message,
      });
    }
  });

  // GET /api/credentials/:id/audit/anomalies
  // Detect and return suspicious activity
  router.get('/:id/audit/anomalies', async (req: Request, res: Response) => {
    const { id } = req.params;

    try {
      const credentialId = BigInt(id);
      const changeHistory = await auditTrail.getChangeHistory(credentialId);
      const anomalies = auditTrail.detectAnomalies(changeHistory.changes);

      res.json({
        credential_id: id,
        total_anomalies_detected: anomalies.length,
        has_anomalies: anomalies.length > 0,
        anomalies: anomalies.map((a) => ({
          type: a.type,
          description: a.description,
          severity: a.type === 'value_reversal' ? 'medium' : 'low',
        })),
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to detect anomalies',
        message,
      });
    }
  });

  // GET /api/credentials/:id/audit/integrity
  // Check integrity status and verify hash chain
  router.get('/:id/audit/integrity', async (req: Request, res: Response) => {
    const { id } = req.params;

    try {
      const credentialId = BigInt(id);
      const changeHistory = await auditTrail.getChangeHistory(credentialId);

      res.json({
        credential_id: id,
        integrity_verified: changeHistory.integrity_verified,
        integrity_status: changeHistory.integrity_verified ? 'verified' : 'TAMPERED',
        last_verified_at: new Date(changeHistory.last_verified_at).toISOString(),
        total_changes_verified: changeHistory.changes.length,
        warning: !changeHistory.integrity_verified
          ? 'ALERT: Hash mismatch detected. The change history may have been tampered with.'
          : null,
      });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      res.status(500).json({
        error: 'Failed to check integrity',
        message,
      });
    }
  });

  return router;
}

export default createAuditRouter();
