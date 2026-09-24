import crypto from 'crypto';
import { getPool } from '../db.js';

/**
 * Credential Audit Trail
 * Issue #1573: Add Audit Trail for Credential Amendments
 *
 * Tracks all changes to credentials, maintaining a tamper-evident log
 * using cryptographic hashing to detect unauthorized modifications.
 */

export interface CredentialChange {
  change_id: string;
  credential_id: bigint;
  field_name: string;
  old_value: unknown;
  new_value: unknown;
  changed_at: number;
  changed_by: string;
  change_reason: string;
  hash: string; // SHA-256 hash of change for tamper detection
}

export interface ChangeHistory {
  changes: CredentialChange[];
  integrity_verified: boolean;
  last_verified_at: number;
}

export class CredentialAuditTrail {
  /**
   * Log a change to a credential
   */
  async logChange(
    credentialId: bigint,
    fieldName: string,
    oldValue: unknown,
    newValue: unknown,
    changedBy: string,
    changeReason: string = 'Not specified'
  ): Promise<CredentialChange> {
    const pool = getPool();
    const now = Date.now();
    const changeId = crypto.randomBytes(16).toString('hex');

    // Create tamper-evident hash
    const changeData = {
      change_id: changeId,
      credential_id: credentialId.toString(),
      field_name: fieldName,
      old_value: oldValue,
      new_value: newValue,
      changed_at: now,
      changed_by: changedBy,
      change_reason: changeReason,
    };

    const hash = crypto.createHash('sha256').update(JSON.stringify(changeData)).digest('hex');

    const query = `
      INSERT INTO credential_change_history (
        change_id,
        credential_id,
        field_name,
        old_value,
        new_value,
        changed_at,
        changed_by,
        change_reason,
        hash
      ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
      RETURNING *;
    `;

    const result = await pool.query<CredentialChange>(query, [
      changeId,
      credentialId,
      fieldName,
      JSON.stringify(oldValue),
      JSON.stringify(newValue),
      now,
      changedBy,
      changeReason,
      hash,
    ]);

    return this.parseChange(result.rows[0]);
  }

  /**
   * Get change history for a credential
   */
  async getChangeHistory(credentialId: bigint): Promise<ChangeHistory> {
    const pool = getPool();

    const query = `
      SELECT *
      FROM credential_change_history
      WHERE credential_id = $1
      ORDER BY changed_at ASC;
    `;

    const result = await pool.query<CredentialChange>(query, [credentialId]);

    const changes = result.rows.map((row) => this.parseChange(row));

    // Verify integrity of the change history
    const integrityVerified = this.verifyIntegrity(changes);

    return {
      changes,
      integrity_verified: integrityVerified,
      last_verified_at: Date.now(),
    };
  }

  /**
   * Get changes for a specific field
   */
  async getFieldHistory(credentialId: bigint, fieldName: string): Promise<CredentialChange[]> {
    const pool = getPool();

    const query = `
      SELECT *
      FROM credential_change_history
      WHERE credential_id = $1 AND field_name = $2
      ORDER BY changed_at ASC;
    `;

    const result = await pool.query<CredentialChange>(query, [credentialId, fieldName]);

    return result.rows.map((row) => this.parseChange(row));
  }

  /**
   * Get changes made by a specific user
   */
  async getChangesByUser(credentialId: bigint, userAddress: string): Promise<CredentialChange[]> {
    const pool = getPool();

    const query = `
      SELECT *
      FROM credential_change_history
      WHERE credential_id = $1 AND changed_by = $2
      ORDER BY changed_at DESC;
    `;

    const result = await pool.query<CredentialChange>(query, [credentialId, userAddress]);

    return result.rows.map((row) => this.parseChange(row));
  }

  /**
   * Get changes within a time range
   */
  async getChangesBetween(
    credentialId: bigint,
    startTime: number,
    endTime: number
  ): Promise<CredentialChange[]> {
    const pool = getPool();

    const query = `
      SELECT *
      FROM credential_change_history
      WHERE credential_id = $1 AND changed_at >= $2 AND changed_at <= $3
      ORDER BY changed_at DESC;
    `;

    const result = await pool.query<CredentialChange>(query, [credentialId, startTime, endTime]);

    return result.rows.map((row) => this.parseChange(row));
  }

  /**
   * Verify the integrity of the change history
   * Detects if any change record has been tampered with
   */
  verifyIntegrity(changes: CredentialChange[]): boolean {
    for (const change of changes) {
      const changeData = {
        change_id: change.change_id,
        credential_id: change.credential_id.toString(),
        field_name: change.field_name,
        old_value: change.old_value,
        new_value: change.new_value,
        changed_at: change.changed_at,
        changed_by: change.changed_by,
        change_reason: change.change_reason,
      };

      const expectedHash = crypto.createHash('sha256').update(JSON.stringify(changeData)).digest('hex');

      if (expectedHash !== change.hash) {
        return false; // Tampering detected
      }
    }

    return true;
  }

  /**
   * Detect suspicious activity in change history
   */
  detectAnomalies(changes: CredentialChange[]): Array<{
    type: string;
    description: string;
    change_ids: string[];
  }> {
    const anomalies: Array<{
      type: string;
      description: string;
      change_ids: string[];
    }> = [];

    // Check for rapid changes
    for (let i = 1; i < changes.length; i++) {
      const timeDiff = changes[i].changed_at - changes[i - 1].changed_at;
      if (timeDiff < 100) { // Less than 100ms apart
        anomalies.push({
          type: 'rapid_changes',
          description: `Suspiciously rapid changes detected (${timeDiff}ms apart)`,
          change_ids: [changes[i - 1].change_id, changes[i].change_id],
        });
      }
    }

    // Check for same-field reversals
    const fieldChanges = new Map<string, CredentialChange[]>();
    for (const change of changes) {
      if (!fieldChanges.has(change.field_name)) {
        fieldChanges.set(change.field_name, []);
      }
      fieldChanges.get(change.field_name)!.push(change);
    }

    for (const [field, fieldChangeList] of fieldChanges) {
      for (let i = 1; i < fieldChangeList.length; i++) {
        const prev = fieldChangeList[i - 1];
        const curr = fieldChangeList[i];

        // Check if new_value of prev equals old_value of curr (revert pattern)
        if (JSON.stringify(prev.new_value) === JSON.stringify(curr.old_value) &&
            JSON.stringify(prev.old_value) === JSON.stringify(curr.new_value)) {
          anomalies.push({
            type: 'value_reversal',
            description: `Field "${field}" was changed and then reverted`,
            change_ids: [prev.change_id, curr.change_id],
          });
        }
      }
    }

    // Check for bulk changes by same user
    const userChangeCounts = new Map<string, CredentialChange[]>();
    for (const change of changes) {
      if (!userChangeCounts.has(change.changed_by)) {
        userChangeCounts.set(change.changed_by, []);
      }
      userChangeCounts.get(change.changed_by)!.push(change);
    }

    for (const [user, userChanges] of userChangeCounts) {
      if (userChanges.length > 10) {
        anomalies.push({
          type: 'bulk_changes',
          description: `User "${user}" made ${userChanges.length} changes (suspicious bulk activity)`,
          change_ids: userChanges.map((c) => c.change_id),
        });
      }
    }

    return anomalies;
  }

  /**
   * Export change history as audit report
   */
  async generateAuditReport(credentialId: bigint): Promise<{
    credential_id: string;
    total_changes: number;
    unique_changers: number;
    date_range: { from: string; to: string };
    integrity_status: string;
    anomalies: Array<{
      type: string;
      description: string;
    }>;
    changes: CredentialChange[];
  }> {
    const changeHistory = await this.getChangeHistory(credentialId);
    const anomalies = this.detectAnomalies(changeHistory.changes);

    const uniqueChangers = new Set(changeHistory.changes.map((c) => c.changed_by)).size;
    const dateRange = changeHistory.changes.length > 0
      ? {
        from: new Date(changeHistory.changes[0].changed_at).toISOString(),
        to: new Date(changeHistory.changes[changeHistory.changes.length - 1].changed_at).toISOString(),
      }
      : { from: 'N/A', to: 'N/A' };

    return {
      credential_id: credentialId.toString(),
      total_changes: changeHistory.changes.length,
      unique_changers: uniqueChangers,
      date_range: dateRange as any,
      integrity_status: changeHistory.integrity_verified ? 'verified' : 'tampered',
      anomalies: anomalies.map((a) => ({
        type: a.type,
        description: a.description,
      })),
      changes: changeHistory.changes,
    };
  }

  /**
   * Parse a change record from database query result
   */
  private parseChange(row: CredentialChange): CredentialChange {
    return {
      ...row,
      old_value: typeof row.old_value === 'string' ? JSON.parse(row.old_value) : row.old_value,
      new_value: typeof row.new_value === 'string' ? JSON.parse(row.new_value) : row.new_value,
    };
  }
}
