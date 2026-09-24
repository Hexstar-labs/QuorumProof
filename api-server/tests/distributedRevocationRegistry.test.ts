import { describe, it, expect, beforeEach } from 'vitest';
import { DistributedRevocationRegistry } from '../src/services/distributedRevocationRegistry.js';

describe('DistributedRevocationRegistry', () => {
  let registry: DistributedRevocationRegistry;

  beforeEach(() => {
    registry = new DistributedRevocationRegistry();
  });

  it('should create a registry with a node ID', () => {
    expect(registry.getNetworkTopology().node_id).toBeDefined();
    expect(registry.getNetworkTopology().role).toBeDefined();
  });

  it('should add revocation entries', () => {
    const record = registry.addRevocationEntry(1, 'GA123', 'compromised');

    expect(record.credential_id).toBe(1);
    expect(record.revoked_by).toBe('GA123');
    expect(record.reason).toBe('compromised');
    expect(record.revoked_at).toBeDefined();
  });

  it('should track revocation consensus version', () => {
    registry.addRevocationEntry(1, 'GA123');
    registry.addRevocationEntry(2, 'GA456');

    const stats = registry.getRegistryStats();
    expect(stats.total_revocations).toBe(2);
    expect(stats.consensus_version).toBeGreaterThan(0);
  });

  it('should register peer nodes', () => {
    const peer = registry.registerPeer('peer1', 'GA_PEER_1');

    expect(peer.peer_id).toBe('peer1');
    expect(peer.address).toBe('GA_PEER_1');
    expect(peer.reachable).toBe(true);
  });

  it('should track multiple peers', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.registerPeer('peer2', 'GA_PEER_2');
    registry.registerPeer('peer3', 'GA_PEER_3');

    const topology = registry.getNetworkTopology();
    expect(topology.total_peers).toBe(3);
    expect(topology.reachable_peers).toBe(3);
  });

  it('should mark peers as unreachable (network partition)', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.registerPeer('peer2', 'GA_PEER_2');

    registry.markPeerUnreachable('peer1');

    const topology = registry.getNetworkTopology();
    expect(topology.reachable_peers).toBe(1);
  });

  it('should mark peers as reachable again', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.markPeerUnreachable('peer1');
    registry.markPeerReachable('peer1');

    const topology = registry.getNetworkTopology();
    expect(topology.reachable_peers).toBe(1);
  });

  it('should check revocation status with consensus', () => {
    registry.addRevocationEntry(1, 'GA123');

    const status = registry.getRevocationStatus(1);
    expect(status.revoked).toBe(true);
    expect(status.consensus_level).toBeGreaterThan(0);
  });

  it('should return not revoked status for non-revoked credential', () => {
    const status = registry.getRevocationStatus(999);
    expect(status.revoked).toBe(false);
  });

  it('should achieve Byzantine fault tolerance with quorum', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.registerPeer('peer2', 'GA_PEER_2');
    registry.registerPeer('peer3', 'GA_PEER_3');

    registry.addRevocationEntry(1, 'GA123');

    const stats = registry.getRegistryStats();
    expect(stats.byzantine_fault_tolerant).toBe(true);
  });

  it('should lose Byzantine fault tolerance with network partitions', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.registerPeer('peer2', 'GA_PEER_2');
    registry.registerPeer('peer3', 'GA_PEER_3');

    registry.markPeerUnreachable('peer1');
    registry.markPeerUnreachable('peer2');

    registry.addRevocationEntry(1, 'GA123');

    const stats = registry.getRegistryStats();
    expect(stats.byzantine_fault_tolerant).toBe(false);
  });

  it('should maintain partition resilience with quorum', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.registerPeer('peer2', 'GA_PEER_2');
    registry.registerPeer('peer3', 'GA_PEER_3');
    registry.registerPeer('peer4', 'GA_PEER_4');
    registry.registerPeer('peer5', 'GA_PEER_5');

    registry.markPeerUnreachable('peer4');
    registry.markPeerUnreachable('peer5');

    registry.addRevocationEntry(1, 'GA123');

    const stats = registry.getRegistryStats();
    expect(stats.network_partition_resilient).toBe(true);
  });

  it('should sync revocation entries from peers', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');

    const record = registry.addRevocationEntry(1, 'GA123');
    const synced = registry.receiveRevocationUpdate('peer1', record);

    expect(synced).toBe(true);
  });

  it('should reject updates from unreachable peers', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.markPeerUnreachable('peer1');

    const record = registry.addRevocationEntry(1, 'GA123');
    const synced = registry.receiveRevocationUpdate('peer1', record);

    expect(synced).toBe(false);
  });

  it('should ignore conflicting updates with older consensus version', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');

    const record1 = registry.addRevocationEntry(1, 'GA123');
    registry.addRevocationEntry(1, 'GA456');

    const oldRecord = {
      ...record1,
      consensus_version: record1.consensus_version - 1,
    };

    const synced = registry.receiveRevocationUpdate('peer1', oldRecord);
    expect(synced).toBe(false);
  });

  it('should track sync log for audit trail', () => {
    registry.addRevocationEntry(1, 'GA123');
    registry.addRevocationEntry(2, 'GA456');

    const logs = registry.getSyncLog();
    expect(logs.length).toBeGreaterThan(0);
  });

  it('should limit sync log size', () => {
    for (let i = 0; i < 100; i++) {
      registry.addRevocationEntry(i, 'GA123');
    }

    const logs = registry.getSyncLog(50);
    expect(logs.length).toBeLessThanOrEqual(50);
  });

  it('should compute registry statistics', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.addRevocationEntry(1, 'GA123');
    registry.addRevocationEntry(2, 'GA456');

    const stats = registry.getRegistryStats();
    expect(stats.total_revocations).toBe(2);
    expect(stats.merkle_root).toBeDefined();
    expect(stats.consensus_version).toBeGreaterThan(0);
  });

  it('should support state commitment', () => {
    registry.addRevocationEntry(1, 'GA123');

    const statsBefore = registry.getRegistryStats();
    registry.commitState();
    const statsAfter = registry.getRegistryStats();

    expect(statsAfter.committed_version).toBe(statsBefore.consensus_version);
  });

  it('should perform peer sync', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.addRevocationEntry(1, 'GA123');

    const syncSuccess = registry.syncWithPeer('peer1');
    expect(syncSuccess).toBe(true);
  });

  it('should handle sync with unreachable peers', () => {
    registry.registerPeer('peer1', 'GA_PEER_1');
    registry.markPeerUnreachable('peer1');

    const syncSuccess = registry.syncWithPeer('peer1');
    expect(syncSuccess).toBe(false);
  });

  it('should update merkle root when adding entries', () => {
    const statsBefore = registry.getRegistryStats();
    registry.addRevocationEntry(1, 'GA123');
    const statsAfter = registry.getRegistryStats();

    expect(statsAfter.merkle_root).not.toEqual(statsBefore.merkle_root);
  });
});
