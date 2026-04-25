/**
 * AuditTrail.test.tsx
 * Tests for the credential audit trail UI — issue #321
 */

import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { AuditTrail, buildAuditEvents } from '../components/AuditTrail';
import type { Credential } from '../lib/contracts/quorumProof';

// ── Fixtures ──────────────────────────────────────────────────────────────────

const BASE_CREDENTIAL: Credential = {
  id: 1n,
  subject: 'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNN',
  issuer:  'GBAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNN',
  credential_type: 1,
  metadata_hash: new Uint8Array(),
  revoked: false,
  expires_at: null,
};

const ATTESTORS = [
  'GCAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNN',
  'GDAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWNN',
];

// ── buildAuditEvents (pure logic) ─────────────────────────────────────────────

describe('buildAuditEvents', () => {
  it('always includes an issued event as the first entry', () => {
    const events = buildAuditEvents(BASE_CREDENTIAL, [], false);
    expect(events[0].type).toBe('issued');
    expect(events[0].label).toBe('Credential Issued');
  });

  it('includes one attested event per attestor', () => {
    const events = buildAuditEvents(BASE_CREDENTIAL, ATTESTORS, false);
    const attested = events.filter((e) => e.type === 'attested');
    expect(attested).toHaveLength(2);
  });

  it('labels attestation events with sequential numbers', () => {
    const events = buildAuditEvents(BASE_CREDENTIAL, ATTESTORS, false);
    const attested = events.filter((e) => e.type === 'attested');
    expect(attested[0].label).toBe('Attestation #1');
    expect(attested[1].label).toBe('Attestation #2');
  });

  it('includes a revoked event when credential is revoked', () => {
    const cred = { ...BASE_CREDENTIAL, revoked: true };
    const events = buildAuditEvents(cred, [], false);
    expect(events.some((e) => e.type === 'revoked')).toBe(true);
  });

  it('does not include a revoked event when credential is not revoked', () => {
    const events = buildAuditEvents(BASE_CREDENTIAL, [], false);
    expect(events.some((e) => e.type === 'revoked')).toBe(false);
  });

  it('includes an expired event when expired and has expires_at', () => {
    const cred = { ...BASE_CREDENTIAL, expires_at: 1700000000n };
    const events = buildAuditEvents(cred, [], true);
    expect(events.some((e) => e.type === 'expired')).toBe(true);
  });

  it('does not include an expired event when not expired', () => {
    const events = buildAuditEvents(BASE_CREDENTIAL, [], false);
    expect(events.some((e) => e.type === 'expired')).toBe(false);
  });

  it('returns only the issued event for a fresh credential with no attestors', () => {
    const events = buildAuditEvents(BASE_CREDENTIAL, [], false);
    expect(events).toHaveLength(1);
    expect(events[0].type).toBe('issued');
  });

  it('orders events: issued → attested → revoked', () => {
    const cred = { ...BASE_CREDENTIAL, revoked: true };
    const events = buildAuditEvents(cred, ATTESTORS, false);
    const types = events.map((e) => e.type);
    expect(types[0]).toBe('issued');
    expect(types[1]).toBe('attested');
    expect(types[2]).toBe('attested');
    expect(types[3]).toBe('revoked');
  });
});

// ── AuditTrail component ──────────────────────────────────────────────────────

describe('AuditTrail component', () => {
  it('renders the audit trail container', () => {
    render(<AuditTrail credential={BASE_CREDENTIAL} attestors={[]} expired={false} />);
    expect(screen.getByLabelText('Credential audit trail')).toBeInTheDocument();
  });

  it('renders the issued event', () => {
    render(<AuditTrail credential={BASE_CREDENTIAL} attestors={[]} expired={false} />);
    expect(screen.getByText('Credential Issued')).toBeInTheDocument();
  });

  it('renders an attestation event for each attestor', () => {
    render(<AuditTrail credential={BASE_CREDENTIAL} attestors={ATTESTORS} expired={false} />);
    expect(screen.getByText('Attestation #1')).toBeInTheDocument();
    expect(screen.getByText('Attestation #2')).toBeInTheDocument();
  });

  it('renders the revoked event when credential is revoked', () => {
    const cred = { ...BASE_CREDENTIAL, revoked: true };
    render(<AuditTrail credential={cred} attestors={[]} expired={false} />);
    expect(screen.getByText('Credential Revoked')).toBeInTheDocument();
  });

  it('does not render the revoked event when not revoked', () => {
    render(<AuditTrail credential={BASE_CREDENTIAL} attestors={[]} expired={false} />);
    expect(screen.queryByText('Credential Revoked')).not.toBeInTheDocument();
  });

  it('renders the expired event when expired', () => {
    const cred = { ...BASE_CREDENTIAL, expires_at: 1700000000n };
    render(<AuditTrail credential={cred} attestors={[]} expired={true} />);
    expect(screen.getByText('Credential Expired')).toBeInTheDocument();
  });

  it('renders event detail text for the issued event', () => {
    render(<AuditTrail credential={BASE_CREDENTIAL} attestors={[]} expired={false} />);
    expect(screen.getByText(/issued by/i)).toBeInTheDocument();
  });

  it('renders event detail text for each attestor', () => {
    render(<AuditTrail credential={BASE_CREDENTIAL} attestors={ATTESTORS} expired={false} />);
    const details = screen.getAllByText(/attested by/i);
    expect(details).toHaveLength(2);
  });

  it('renders the correct number of timeline items', () => {
    render(<AuditTrail credential={BASE_CREDENTIAL} attestors={ATTESTORS} expired={false} />);
    // issued + 2 attested = 3
    const items = screen.getAllByRole('listitem');
    expect(items).toHaveLength(3);
  });

  it('does not render the connector line on the last event', () => {
    render(<AuditTrail credential={BASE_CREDENTIAL} attestors={[]} expired={false} />);
    // Only 1 event (issued), so no audit-line should be rendered
    const lines = document.querySelectorAll('.audit-line');
    expect(lines).toHaveLength(0);
  });

  it('renders connector lines between events', () => {
    render(<AuditTrail credential={BASE_CREDENTIAL} attestors={ATTESTORS} expired={false} />);
    // 3 events → 2 lines
    const lines = document.querySelectorAll('.audit-line');
    expect(lines).toHaveLength(2);
  });
});
