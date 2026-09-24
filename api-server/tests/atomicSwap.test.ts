import { describe, it, expect, beforeEach } from 'vitest';
import { AtomicSwapService } from '../src/services/atomicSwap.js';

describe('AtomicSwapService', () => {
  let swapService: AtomicSwapService;

  beforeEach(() => {
    swapService = new AtomicSwapService();
  });

  it('should initiate a new atomic swap', () => {
    const swap = swapService.initiateSwap(
      'GA123',
      'GA456',
      1,
      2,
    );

    expect(swap.swap_id).toBeDefined();
    expect(swap.initiator.address).toBe('GA123');
    expect(swap.responder.address).toBe('GA456');
    expect(swap.state).toBe('pending');
    expect(swap.created_at).toBeDefined();
    expect(swap.expires_at).toBeDefined();
  });

  it('should reject swap with same initiator and responder', () => {
    expect(() => {
      swapService.initiateSwap('GA123', 'GA123', 1, 2);
    }).toThrow('Initiator and responder must be different parties');
  });

  it('should lock credentials for both participants', () => {
    const swap = swapService.initiateSwap('GA123', 'GA456', 1, 2);

    let lockedSwap = swapService.lockCredential(swap.swap_id, 'GA123');
    expect(lockedSwap.state).toBe('locked_initiator');
    expect(lockedSwap.initiator.locked).toBe(true);

    lockedSwap = swapService.lockCredential(swap.swap_id, 'GA456');
    expect(lockedSwap.state).toBe('locked_both');
    expect(lockedSwap.responder.locked).toBe(true);
  });

  it('should complete a locked swap with valid signatures', () => {
    const swap = swapService.initiateSwap('GA123', 'GA456', 1, 2);
    swapService.lockCredential(swap.swap_id, 'GA123');
    swapService.lockCredential(swap.swap_id, 'GA456');

    const completedSwap = swapService.completeSwap(
      swap.swap_id,
      'sig_initiator',
      'sig_responder'
    );

    expect(completedSwap.state).toBe('completed');
    expect(completedSwap.completed_at).toBeDefined();
  });

  it('should reject completion without locked_both state', () => {
    const swap = swapService.initiateSwap('GA123', 'GA456', 1, 2);

    expect(() => {
      swapService.completeSwap(swap.swap_id, 'sig1', 'sig2');
    }).toThrow('Cannot complete swap that is not in locked_both state');
  });

  it('should cancel a locked swap', () => {
    const swap = swapService.initiateSwap('GA123', 'GA456', 1, 2);
    swapService.lockCredential(swap.swap_id, 'GA123');

    const cancelledSwap = swapService.cancelSwap(swap.swap_id, 'GA123');
    expect(cancelledSwap.state).toBe('cancelled');
  });

  it('should reject cancellation by non-participant', () => {
    const swap = swapService.initiateSwap('GA123', 'GA456', 1, 2);
    swapService.lockCredential(swap.swap_id, 'GA123');

    expect(() => {
      swapService.cancelSwap(swap.swap_id, 'GA999');
    }).toThrow('Address is not a participant in this swap');
  });

  it('should list swaps for a participant', () => {
    const swap1 = swapService.initiateSwap('GA123', 'GA456', 1, 2);
    const swap2 = swapService.initiateSwap('GA123', 'GA789', 3, 4);
    const swap3 = swapService.initiateSwap('GA999', 'GA888', 5, 6);

    const swaps = swapService.listSwapsForParticipant('GA123');
    expect(swaps).toHaveLength(2);
    expect(swaps.some(s => s.swap_id === swap1.swap_id)).toBe(true);
    expect(swaps.some(s => s.swap_id === swap2.swap_id)).toBe(true);
  });

  it('should return swap statistics', () => {
    swapService.initiateSwap('GA123', 'GA456', 1, 2);
    const swap2 = swapService.initiateSwap('GA789', 'GA999', 3, 4);
    swapService.lockCredential(swap2.swap_id, 'GA789');

    const stats = swapService.getSwapStats();
    expect(stats.total).toBe(2);
    expect(stats.by_state.pending).toBe(1);
    expect(stats.by_state.locked_initiator).toBe(1);
  });

  it('should handle expired swaps', (done) => {
    const swap = swapService.initiateSwap('GA123', 'GA456', 1, 2);

    setTimeout(() => {
      const retrievedSwap = swapService.getSwap(swap.swap_id);
      expect(retrievedSwap.state).toBe('expired');
      done();
    }, 100);
  });

  it('should get swap by ID', () => {
    const swap = swapService.initiateSwap('GA123', 'GA456', 1, 2);
    const retrieved = swapService.getSwap(swap.swap_id);

    expect(retrieved.swap_id).toBe(swap.swap_id);
    expect(retrieved.initiator.address).toBe('GA123');
  });

  it('should reject getting non-existent swap', () => {
    expect(() => {
      swapService.getSwap('non_existent');
    }).toThrow('Swap non_existent not found');
  });
});
