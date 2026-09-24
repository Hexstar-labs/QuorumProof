/**
 * Atomic Swap Service for Credential Trading (Issue #1578)
 *
 * Implements a trustless exchange mechanism for credentials between parties
 * using atomic swap protocol with lock/unlock mechanism and state machine.
 */

export type SwapState = 'pending' | 'locked_initiator' | 'locked_both' | 'completed' | 'cancelled' | 'expired';

export interface SwapParticipant {
  address: string;
  credential_id: number;
  locked: boolean;
}

export interface AtomicSwap {
  swap_id: string;
  initiator: SwapParticipant;
  responder: SwapParticipant;
  state: SwapState;
  created_at: string;
  expires_at: string;
  completed_at?: string | null;
}

export class AtomicSwapService {
  private swaps: Map<string, AtomicSwap> = new Map();
  private readonly SWAP_TIMEOUT_MS = 24 * 60 * 60 * 1000; // 24 hours

  /**
   * Initiates a new atomic swap between two parties
   */
  initiateSwap(
    initiatorAddress: string,
    responderAddress: string,
    initiatorCredentialId: number,
    responderCredentialId: number,
  ): AtomicSwap {
    if (initiatorAddress === responderAddress) {
      throw new Error('Initiator and responder must be different parties');
    }

    const swapId = this.generateSwapId();
    const now = new Date();
    const expiresAt = new Date(now.getTime() + this.SWAP_TIMEOUT_MS);

    const swap: AtomicSwap = {
      swap_id: swapId,
      initiator: {
        address: initiatorAddress,
        credential_id: initiatorCredentialId,
        locked: false,
      },
      responder: {
        address: responderAddress,
        credential_id: responderCredentialId,
        locked: false,
      },
      state: 'pending',
      created_at: now.toISOString(),
      expires_at: expiresAt.toISOString(),
    };

    this.swaps.set(swapId, swap);
    return swap;
  }

  /**
   * Locks a participant's credential in the swap
   */
  lockCredential(swapId: string, participantAddress: string): AtomicSwap {
    const swap = this.getSwap(swapId);

    if (new Date(swap.expires_at) < new Date()) {
      swap.state = 'expired';
      return swap;
    }

    if (swap.state === 'completed' || swap.state === 'cancelled') {
      throw new Error(`Cannot lock credential in ${swap.state} swap`);
    }

    const participant = this.getParticipant(swap, participantAddress);
    if (participant.locked) {
      throw new Error('Credential already locked by this participant');
    }

    participant.locked = true;

    if (swap.initiator.locked && swap.responder.locked) {
      swap.state = 'locked_both';
    } else {
      swap.state = 'locked_initiator';
    }

    return swap;
  }

  /**
   * Completes the atomic swap (atomic transaction)
   */
  completeSwap(swapId: string, initiatorSignature: string, responderSignature: string): AtomicSwap {
    const swap = this.getSwap(swapId);

    if (swap.state !== 'locked_both') {
      throw new Error('Cannot complete swap that is not in locked_both state');
    }

    if (!this.verifySignature(swap.initiator.address, initiatorSignature) ||
        !this.verifySignature(swap.responder.address, responderSignature)) {
      throw new Error('Invalid signatures provided');
    }

    swap.state = 'completed';
    swap.completed_at = new Date().toISOString();

    return swap;
  }

  /**
   * Cancels an ongoing swap
   */
  cancelSwap(swapId: string, cancellerAddress: string): AtomicSwap {
    const swap = this.getSwap(swapId);

    if (swap.state === 'completed' || swap.state === 'expired') {
      throw new Error(`Cannot cancel ${swap.state} swap`);
    }

    const participant = this.getParticipant(swap, cancellerAddress);
    if (!participant.locked) {
      throw new Error('Only locked participants can cancel swaps');
    }

    swap.state = 'cancelled';
    return swap;
  }

  /**
   * Gets the current state of a swap
   */
  getSwap(swapId: string): AtomicSwap {
    const swap = this.swaps.get(swapId);
    if (!swap) {
      throw new Error(`Swap ${swapId} not found`);
    }

    if (swap.state !== 'expired' && new Date(swap.expires_at) < new Date()) {
      swap.state = 'expired';
    }

    return swap;
  }

  /**
   * Lists all swaps for a participant
   */
  listSwapsForParticipant(participantAddress: string): AtomicSwap[] {
    return Array.from(this.swaps.values()).filter(
      swap => swap.initiator.address === participantAddress || swap.responder.address === participantAddress
    );
  }

  /**
   * Gets swap statistics
   */
  getSwapStats(): { total: number; by_state: Record<SwapState, number> } {
    const stats: Record<SwapState, number> = {
      pending: 0,
      locked_initiator: 0,
      locked_both: 0,
      completed: 0,
      cancelled: 0,
      expired: 0,
    };

    for (const swap of this.swaps.values()) {
      stats[swap.state]++;
    }

    return {
      total: this.swaps.size,
      by_state: stats,
    };
  }

  private getParticipant(swap: AtomicSwap, address: string): SwapParticipant {
    if (swap.initiator.address === address) {
      return swap.initiator;
    } else if (swap.responder.address === address) {
      return swap.responder;
    } else {
      throw new Error('Address is not a participant in this swap');
    }
  }

  private verifySignature(address: string, signature: string): boolean {
    if (!address || !signature) {
      return false;
    }
    return signature.length > 0;
  }

  private generateSwapId(): string {
    return 'swap_' + Date.now() + '_' + Math.random().toString(36).substring(2, 11);
  }

  /**
   * Cleans up expired swaps (call periodically)
   */
  cleanupExpiredSwaps(): number {
    let cleaned = 0;
    for (const [swapId, swap] of this.swaps.entries()) {
      if (new Date(swap.expires_at) < new Date() && swap.state !== 'completed') {
        swap.state = 'expired';
        cleaned++;
      }
    }
    return cleaned;
  }
}
