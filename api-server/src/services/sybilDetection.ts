/**
 * Sybil Attack Detection Service (Issue #1579)
 *
 * Detects and flags suspicious Sybil attack patterns including:
 * - Rapid credential creation across accounts
 * - Behavioral anomalies
 * - Credential creation scoring
 * - High-risk account alerting
 */

export type RiskLevel = 'low' | 'medium' | 'high' | 'critical';

export interface AccountActivity {
  address: string;
  credential_count: number;
  last_credential_created: string;
  creation_timestamps: number[];
}

export interface SybilScore {
  address: string;
  score: number;
  risk_level: RiskLevel;
  factors: SybilScoreFactor[];
  calculated_at: string;
}

export interface SybilScoreFactor {
  name: string;
  weight: number;
  value: number;
  contribution: number;
}

export interface AnomalyAlert {
  alert_id: string;
  address: string;
  alert_type: 'rapid_creation' | 'unusual_pattern' | 'high_score' | 'bulk_activity';
  severity: RiskLevel;
  description: string;
  evidence: Record<string, unknown>;
  created_at: string;
  dismissed: boolean;
}

export class SybilDetectionService {
  private accountActivity: Map<string, AccountActivity> = new Map();
  private sybilScores: Map<string, SybilScore> = new Map();
  private anomalyAlerts: Map<string, AnomalyAlert> = new Map();

  private readonly RAPID_CREATION_THRESHOLD = 5; // credentials
  private readonly RAPID_CREATION_WINDOW = 60 * 60 * 1000; // 1 hour in ms
  private readonly ANOMALY_SCORE_THRESHOLD = 70;
  private readonly CRITICAL_SCORE_THRESHOLD = 85;

  /**
   * Records a credential creation event
   */
  recordCredentialCreation(address: string): void {
    const now = Date.now();
    const existing = this.accountActivity.get(address);

    if (existing) {
      existing.credential_count++;
      existing.last_credential_created = new Date().toISOString();
      existing.creation_timestamps.push(now);
      // Keep only last 24 hours of timestamps
      existing.creation_timestamps = existing.creation_timestamps.filter(
        ts => now - ts < 24 * 60 * 60 * 1000
      );
    } else {
      this.accountActivity.set(address, {
        address,
        credential_count: 1,
        last_credential_created: new Date().toISOString(),
        creation_timestamps: [now],
      });
    }

    this.checkForAnomalies(address);
    this.updateSybilScore(address);
  }

  /**
   * Analyzes behavioral patterns for Sybil indicators
   */
  private checkForAnomalies(address: string): void {
    const activity = this.accountActivity.get(address);
    if (!activity) return;

    const now = Date.now();
    const recentCreations = activity.creation_timestamps.filter(
      ts => now - ts < this.RAPID_CREATION_WINDOW
    );

    if (recentCreations.length >= this.RAPID_CREATION_THRESHOLD) {
      this.createAlert(
        address,
        'rapid_creation',
        'high',
        `${recentCreations.length} credentials created in the last hour`,
        {
          creation_count: recentCreations.length,
          time_window_minutes: 60,
          threshold: this.RAPID_CREATION_THRESHOLD,
        }
      );
    }

    const timeDiffs = this.calculateTimeDifferences(recentCreations);
    if (this.detectUnusualPattern(timeDiffs)) {
      this.createAlert(
        address,
        'unusual_pattern',
        'medium',
        'Unusual credential creation pattern detected',
        {
          time_differences: timeDiffs,
          pattern: 'automated_or_coordinated',
        }
      );
    }
  }

  /**
   * Calculates Sybil score based on multiple factors
   */
  private updateSybilScore(address: string): void {
    const activity = this.accountActivity.get(address);
    if (!activity) {
      this.sybilScores.delete(address);
      return;
    }

    const now = Date.now();
    const factors: SybilScoreFactor[] = [];

    const creationRateFactor = this.calculateCreationRateFactor(activity);
    factors.push(creationRateFactor);

    const timingPatternFactor = this.calculateTimingPatternFactor(activity.creation_timestamps);
    factors.push(timingPatternFactor);

    const volumeFactor = this.calculateVolumeFactor(activity.credential_count);
    factors.push(volumeFactor);

    const recencyFactor = this.calculateRecencyFactor(activity.last_credential_created);
    factors.push(recencyFactor);

    const totalScore = factors.reduce((sum, f) => sum + f.contribution, 0);
    const normalizedScore = Math.min(100, Math.max(0, totalScore));

    const riskLevel = this.scoreToRiskLevel(normalizedScore);

    const sybilScore: SybilScore = {
      address,
      score: normalizedScore,
      risk_level: riskLevel,
      factors,
      calculated_at: new Date().toISOString(),
    };

    this.sybilScores.set(address, sybilScore);

    if (normalizedScore >= this.CRITICAL_SCORE_THRESHOLD) {
      this.createAlert(
        address,
        'high_score',
        'critical',
        `Critical Sybil score: ${normalizedScore.toFixed(1)}`,
        { score: normalizedScore, factors }
      );
    }
  }

  /**
   * Calculates creation rate factor (how fast credentials are created)
   */
  private calculateCreationRateFactor(activity: AccountActivity): SybilScoreFactor {
    const now = Date.now();
    const oneHourAgo = now - this.RAPID_CREATION_WINDOW;
    const recentCount = activity.creation_timestamps.filter(ts => ts >= oneHourAgo).length;

    const value = Math.min(recentCount / this.RAPID_CREATION_THRESHOLD, 1);
    const weight = 0.35;

    return {
      name: 'creation_rate',
      weight,
      value,
      contribution: value * weight * 100,
    };
  }

  /**
   * Calculates timing pattern factor (unusually synchronized)
   */
  private calculateTimingPatternFactor(timestamps: number[]): SybilScoreFactor {
    if (timestamps.length < 2) {
      return { name: 'timing_pattern', weight: 0.25, value: 0, contribution: 0 };
    }

    const diffs = this.calculateTimeDifferences(timestamps);
    const avgDiff = diffs.reduce((a, b) => a + b, 0) / diffs.length;
    const stdDev = this.calculateStdDev(diffs, avgDiff);

    const isAnomalous = stdDev < 1000; // < 1 second std dev suggests automation
    const value = isAnomalous ? 1 : Math.max(0, 1 - stdDev / 10000);
    const weight = 0.25;

    return {
      name: 'timing_pattern',
      weight,
      value,
      contribution: value * weight * 100,
    };
  }

  /**
   * Calculates volume factor (total credentials created)
   */
  private calculateVolumeFactor(count: number): SybilScoreFactor {
    const weight = 0.2;
    const value = Math.min(count / 100, 1);

    return {
      name: 'credential_volume',
      weight,
      value,
      contribution: value * weight * 100,
    };
  }

  /**
   * Calculates recency factor (how recent the activity is)
   */
  private calculateRecencyFactor(lastCreated: string): SybilScoreFactor {
    const lastCreatedTime = new Date(lastCreated).getTime();
    const now = Date.now();
    const ageMs = now - lastCreatedTime;
    const oneDayMs = 24 * 60 * 60 * 1000;

    const value = Math.max(0, 1 - ageMs / oneDayMs);
    const weight = 0.2;

    return {
      name: 'recency',
      weight,
      value,
      contribution: value * weight * 100,
    };
  }

  /**
   * Gets the Sybil score for an address
   */
  getSybilScore(address: string): SybilScore | null {
    return this.sybilScores.get(address) || null;
  }

  /**
   * Gets account activity for an address
   */
  getAccountActivity(address: string): AccountActivity | null {
    return this.accountActivity.get(address) || null;
  }

  /**
   * Creates an anomaly alert
   */
  private createAlert(
    address: string,
    alertType: AnomalyAlert['alert_type'],
    severity: RiskLevel,
    description: string,
    evidence: Record<string, unknown>
  ): void {
    const alertId = `alert_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`;

    const alert: AnomalyAlert = {
      alert_id: alertId,
      address,
      alert_type: alertType,
      severity,
      description,
      evidence,
      created_at: new Date().toISOString(),
      dismissed: false,
    };

    this.anomalyAlerts.set(alertId, alert);
  }

  /**
   * Gets active alerts for an address
   */
  getAlertsForAddress(address: string): AnomalyAlert[] {
    return Array.from(this.anomalyAlerts.values()).filter(
      alert => alert.address === address && !alert.dismissed
    );
  }

  /**
   * Dismisses an alert
   */
  dismissAlert(alertId: string): boolean {
    const alert = this.anomalyAlerts.get(alertId);
    if (alert) {
      alert.dismissed = true;
      return true;
    }
    return false;
  }

  /**
   * Gets all active alerts
   */
  getAllActiveAlerts(): AnomalyAlert[] {
    return Array.from(this.anomalyAlerts.values()).filter(alert => !alert.dismissed);
  }

  /**
   * Gets high-risk accounts
   */
  getHighRiskAccounts(): SybilScore[] {
    return Array.from(this.sybilScores.values()).filter(
      score => score.risk_level === 'high' || score.risk_level === 'critical'
    );
  }

  private detectUnusualPattern(timeDiffs: number[]): boolean {
    if (timeDiffs.length < 2) return false;
    const stdDev = this.calculateStdDev(timeDiffs, this.calculateMean(timeDiffs));
    return stdDev < 1000;
  }

  private calculateTimeDifferences(timestamps: number[]): number[] {
    const sorted = [...timestamps].sort((a, b) => a - b);
    const diffs: number[] = [];
    for (let i = 1; i < sorted.length; i++) {
      diffs.push(sorted[i] - sorted[i - 1]);
    }
    return diffs;
  }

  private calculateMean(values: number[]): number {
    return values.reduce((a, b) => a + b, 0) / values.length;
  }

  private calculateStdDev(values: number[], mean: number): number {
    const variance = values.reduce((sum, v) => sum + Math.pow(v - mean, 2), 0) / values.length;
    return Math.sqrt(variance);
  }

  private scoreToRiskLevel(score: number): RiskLevel {
    if (score >= this.CRITICAL_SCORE_THRESHOLD) return 'critical';
    if (score >= this.ANOMALY_SCORE_THRESHOLD) return 'high';
    if (score >= 40) return 'medium';
    return 'low';
  }
}
