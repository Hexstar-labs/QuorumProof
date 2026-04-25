# Design Document: Member Offboarding

## Overview

The member offboarding feature provides a comprehensive system for removing slice members from the quorum proof system while maintaining system integrity and ensuring proper transition handling. This design addresses the systematic removal of attestors from quorum slices with configurable transition periods, emergency capabilities, and comprehensive audit trails.

The system supports both graceful offboarding with transition periods and immediate emergency removal, while preserving existing attestations and managing SBT token lifecycle. The design ensures that credential validity is maintained during member transitions and provides robust tracking and cancellation capabilities.

## Architecture

### Core Components

The member offboarding system extends the existing QuorumProofContract with new functionality organized into several key components:

1. **Offboarding Manager**: Handles the lifecycle of offboarding processes
2. **Transition Period Controller**: Manages time-based member removal phases
3. **Attestation Preservation System**: Ensures existing attestations remain valid during transitions
4. **SBT Token Handler**: Coordinates with SBT Registry for token management
5. **Audit Trail System**: Maintains comprehensive logging and history
6. **Emergency Response System**: Provides immediate removal capabilities

### System Integration

The offboarding system integrates with existing contracts:

- **QuorumProofContract**: Extended with offboarding functions and state management
- **SBT Registry**: Coordinated token operations (burn, transfer, retain)
- **Event System**: Enhanced with offboarding-specific events for monitoring

### State Management

New data structures are added to track offboarding processes:

```rust
pub enum OffboardingStatus {
    Initiated,
    TransitionActive,
    Completed,
    Cancelled,
    Emergency,
}

pub struct OffboardingRecord {
    pub member: Address,
    pub slice_id: u64,
    pub initiator: Address,
    pub status: OffboardingStatus,
    pub initiated_at: u64,
    pub transition_period: u64,
    pub completion_deadline: u64,
    pub reason_code: u32,
    pub metadata: Bytes,
    pub token_action: TokenAction,
}

pub enum TokenAction {
    Burn,
    Transfer(Address),
    Retain,
}
```

## Components and Interfaces

### Offboarding Manager

The core component responsible for managing offboarding lifecycle:

```rust
impl QuorumProofContract {
    pub fn initiate_member_offboarding(
        env: Env,
        initiator: Address,
        slice_id: u64,
        member: Address,
        transition_period: u64,
        reason_code: u32,
        metadata: Bytes,
        token_action: TokenAction,
    ) -> u64;

    pub fn batch_initiate_offboarding(
        env: Env,
        initiator: Address,
        slice_id: u64,
        members: Vec<Address>,
        transition_period: u64,
        reason_code: u32,
        metadata: Bytes,
        token_action: TokenAction,
    ) -> Vec<u64>;

    pub fn cancel_offboarding(
        env: Env,
        initiator: Address,
        offboarding_id: u64,
    );

    pub fn complete_offboarding(
        env: Env,
        offboarding_id: u64,
    );
}
```

### Status and Query Interface

Functions for tracking and monitoring offboarding processes:

```rust
impl QuorumProofContract {
    pub fn get_offboarding_status(
        env: Env,
        offboarding_id: u64,
    ) -> OffboardingRecord;

    pub fn get_member_offboarding_status(
        env: Env,
        slice_id: u64,
        member: Address,
    ) -> Option<OffboardingRecord>;

    pub fn list_active_offboardings(
        env: Env,
        slice_id: u64,
    ) -> Vec<u64>;

    pub fn get_offboarding_history(
        env: Env,
        slice_id: u64,
        page: u32,
        page_size: u32,
    ) -> Vec<OffboardingRecord>;
}
```

### Emergency Offboarding Interface

Immediate removal capabilities for security situations:

```rust
impl QuorumProofContract {
    pub fn emergency_offboard_member(
        env: Env,
        admin: Address,
        slice_id: u64,
        member: Address,
        reason_code: u32,
        metadata: Bytes,
    ) -> u64;

    pub fn batch_emergency_offboard(
        env: Env,
        admin: Address,
        slice_id: u64,
        members: Vec<Address>,
        reason_code: u32,
        metadata: Bytes,
    ) -> Vec<u64>;
}
```

### SBT Token Management Integration

Coordination with SBT Registry for token lifecycle:

```rust
impl QuorumProofContract {
    fn execute_token_action(
        env: &Env,
        member: &Address,
        action: &TokenAction,
    );

    fn get_member_tokens(
        env: &Env,
        member: &Address,
    ) -> Vec<u64>;
}
```

## Data Models

### Core Data Structures

**OffboardingRecord**: Primary data structure tracking offboarding processes
- `member`: Address being offboarded
- `slice_id`: Target slice for removal
- `initiator`: Address that initiated the process
- `status`: Current offboarding status
- `initiated_at`: Timestamp of initiation
- `transition_period`: Duration of transition phase
- `completion_deadline`: When offboarding completes
- `reason_code`: Categorized reason for offboarding
- `metadata`: Additional context and documentation
- `token_action`: How to handle member's SBT tokens

**OffboardingStatus**: Enumeration of process states
- `Initiated`: Process started, validation complete
- `TransitionActive`: In transition period, member still active
- `Completed`: Process finished, member removed
- `Cancelled`: Process cancelled, member restored
- `Emergency`: Immediate removal executed

**TokenAction**: SBT token handling options
- `Burn`: Destroy the member's tokens
- `Transfer(Address)`: Transfer tokens to specified address
- `Retain`: Keep tokens with original owner

### Storage Keys

New storage keys for offboarding data:

```rust
pub enum DataKey {
    // Existing keys...
    OffboardingRecord(u64),
    OffboardingCounter,
    MemberOffboarding(u64, Address), // slice_id, member
    SliceOffboardings(u64), // slice_id -> Vec<u64>
    OffboardingHistory(u64, u32), // slice_id, page
}
```

### Event Structures

New events for offboarding operations:

```rust
pub struct OffboardingInitiatedEvent {
    pub offboarding_id: u64,
    pub slice_id: u64,
    pub member: Address,
    pub initiator: Address,
    pub transition_period: u64,
    pub reason_code: u32,
}

pub struct OffboardingStatusChangeEvent {
    pub offboarding_id: u64,
    pub old_status: OffboardingStatus,
    pub new_status: OffboardingStatus,
    pub timestamp: u64,
}

pub struct EmergencyOffboardingEvent {
    pub offboarding_id: u64,
    pub slice_id: u64,
    pub member: Address,
    pub admin: Address,
    pub reason_code: u32,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: Offboarding Record Creation

*For any* valid slice creator, slice ID, member address, and offboarding parameters, calling initiate_member_offboarding should create exactly one offboarding record with the specified parameters.

**Validates: Requirements 1.2**

### Property 2: Unauthorized Access Rejection

*For any* address that is not the slice creator, calling initiate_member_offboarding should be rejected with an authorization error.

**Validates: Requirements 1.3**

### Property 3: Non-existent Slice Rejection

*For any* slice ID that does not exist in the system, calling initiate_member_offboarding should be rejected with a slice not found error.

**Validates: Requirements 1.4**

### Property 4: Non-member Rejection

*For any* member address that is not part of the target slice, calling initiate_member_offboarding should be rejected with a member not found error.

**Validates: Requirements 1.5**

### Property 5: Transition Period Establishment

*For any* offboarding process initiation, the system should establish a transition period matching the specified duration.

**Validates: Requirements 2.1**

### Property 6: Attestation Acceptance During Transition

*For any* member in transition period, the system should continue to accept valid attestations from that member.

**Validates: Requirements 2.2**

### Property 7: Slice Assignment Prevention

*For any* member in transition period, the system should reject attempts to assign that member to new slices.

**Validates: Requirements 2.3**

### Property 8: Automatic Completion

*For any* offboarding process, when the transition period expires, the system should automatically complete the member removal.

**Validates: Requirements 2.4**

### Property 9: Transition Event Emission

*For any* transition period operation, the system should emit appropriate monitoring and audit events.

**Validates: Requirements 2.5**

### Property 10: Attestation Preservation

*For any* member being offboarded, all existing attestations from that member should remain valid and preserved throughout the process.

**Validates: Requirements 3.1**

### Property 11: Quorum Threshold Counting

*For any* member in transition period, their existing attestations should continue to count toward quorum thresholds.

**Validates: Requirements 3.2**

### Property 12: Weight Redistribution

*For any* completed offboarding process, attestation weights should be gracefully redistributed without affecting existing credential validity.

**Validates: Requirements 3.3**

### Property 13: Quorum Protection

*For any* member removal that would cause active credentials to fall below quorum threshold, the system should require explicit confirmation or alternative attestor assignment.

**Validates: Requirements 3.4**

### Property 14: Token Handling Options

*For any* completed member offboarding, the SBT Registry should provide and properly execute all three token handling options (burn, transfer, retain).

**Validates: Requirements 4.1**

### Property 15: Token Burn Operation

*For any* offboarding with burn token action selected, the member's tokens should be burned and burn events should be emitted.

**Validates: Requirements 4.2**

### Property 16: Token Transfer Operation

*For any* offboarding with transfer token action selected, the member's tokens should be transferred to the designated address.

**Validates: Requirements 4.3**

### Property 17: Token Audit Trails

*For any* token operation during offboarding, audit trails should be maintained in the SBT Registry.

**Validates: Requirements 4.4**

### Property 18: Status Tracking

*For any* offboarding process, all stages (initiated, transition_active, completed, cancelled) should be properly tracked and transitions should work correctly.

**Validates: Requirements 5.2**

### Property 19: Status Change Events

*For any* offboarding status change, the system should emit status change events.

**Validates: Requirements 5.3**

### Property 20: Emergency Removal

*For any* emergency offboarding operation, member removal should occur immediately without transition period.

**Validates: Requirements 6.1**

### Property 21: Immediate Privilege Revocation

*For any* emergency offboarding, the member's attestation privileges should be immediately revoked.

**Validates: Requirements 6.2**

### Property 22: Emergency Event Emission

*For any* emergency offboarding execution, emergency offboarding events should be emitted for audit purposes.

**Validates: Requirements 6.3**

### Property 23: Emergency Privilege Requirements

*For any* emergency offboarding operation, only addresses with elevated privileges should be able to execute the operation.

**Validates: Requirements 6.4**

### Property 24: Cancellation During Transition

*For any* offboarding process in transition period, the slice creator should be able to cancel the process.

**Validates: Requirements 8.1**

### Property 25: Privilege Restoration on Cancellation

*For any* cancelled offboarding, the member's full attestation privileges should be restored.

**Validates: Requirements 8.2**

### Property 26: Cancellation Event Emission

*For any* offboarding cancellation, cancellation events should be emitted.

**Validates: Requirements 8.3**

### Property 27: Completed Offboarding Cancellation Prevention

*For any* completed offboarding process, cancellation attempts should be prevented and rejected.

**Validates: Requirements 8.4**

### Property 28: Batch Validation

*For any* batch offboarding operation, all members should be validated before processing any individual offboarding.

**Validates: Requirements 9.2**

### Property 29: Batch Error Reporting

*For any* batch offboarding that encounters errors, detailed error reporting should be provided for each member.

**Validates: Requirements 9.3**

### Property 30: Batch Atomicity

*For any* batch offboarding operation, the operation should be atomic (all succeed or all fail).

**Validates: Requirements 9.4**

### Property 31: Operation Logging

*For any* offboarding operation, the system should log the operation with timestamps and initiator addresses.

**Validates: Requirements 10.1**

### Property 32: Record Content Requirements

*For any* offboarding record, it should include reason codes and metadata.

**Validates: Requirements 10.3**

### Property 33: Audit Data Immutability

*For any* offboarding audit data, it should be immutable and verifiable after creation.

**Validates: Requirements 10.4**

## Error Handling

The member offboarding system implements comprehensive error handling across all operations:

### Validation Errors

- **Unauthorized Access**: Thrown when non-slice-creators attempt to initiate offboarding
- **Slice Not Found**: Thrown when targeting non-existent slices
- **Member Not Found**: Thrown when targeting members not in the specified slice
- **Invalid Parameters**: Thrown for malformed input parameters (negative transition periods, empty metadata when required)

### State Errors

- **Already Offboarding**: Thrown when attempting to initiate offboarding for a member already in process
- **Invalid Status Transition**: Thrown when attempting invalid status changes (e.g., cancelling completed offboarding)
- **Transition Period Expired**: Thrown when attempting operations on expired transition periods

### System Errors

- **Quorum Threshold Violation**: Thrown when member removal would break quorum requirements without confirmation
- **Contract Paused**: Thrown when attempting offboarding operations while the contract is paused
- **Storage Errors**: Thrown for storage operation failures

### Batch Operation Errors

- **Partial Validation Failure**: Detailed error reporting for each member in batch operations
- **Atomic Operation Failure**: Rollback mechanism for failed batch operations
- **Resource Limits**: Handling for batch operations exceeding system limits

### Recovery Mechanisms

- **Graceful Degradation**: System continues operating with reduced functionality during partial failures
- **State Consistency**: Automatic state repair mechanisms for inconsistent offboarding states
- **Event Replay**: Ability to reconstruct state from event logs in case of corruption

## Testing Strategy

The member offboarding feature employs a comprehensive dual testing approach combining unit tests for specific scenarios and property-based tests for universal correctness guarantees.

### Unit Testing Approach

Unit tests focus on specific examples, edge cases, and integration points:

**Core Function Testing**:
- Test initiate_member_offboarding with valid parameters
- Test batch_initiate_offboarding with multiple members
- Test emergency_offboard_member with admin privileges
- Test cancel_offboarding during transition period

**Edge Case Testing**:
- Last member removal scenarios
- Concurrent offboarding attempts
- System pause during offboarding process
- Transition period expiration edge cases
- Token handling with zero balance

**Integration Testing**:
- End-to-end offboarding workflow
- SBT Registry coordination
- Event emission verification
- Cross-contract state consistency

**Error Condition Testing**:
- Unauthorized access attempts
- Invalid parameter combinations
- Non-existent slice/member scenarios
- Quorum threshold violations

### Property-Based Testing Configuration

Property-based tests verify universal properties across all inputs using **QuickCheck for Rust** with minimum 100 iterations per test:

**Property Test Implementation**:
- Each correctness property implemented as a single property-based test
- Test tags reference design document properties: **Feature: member-offboarding, Property {number}: {property_text}**
- Comprehensive input generation for addresses, slice IDs, transition periods, and metadata
- State-based testing with before/after comparisons

**Invariant Testing**:
- Member count consistency across offboarding operations
- Attestation preservation during transitions
- Event emission completeness
- Audit trail integrity

**Metamorphic Testing**:
- Offboarding then cancellation should restore original state
- Batch operations should produce same result as individual operations
- Emergency offboarding should be equivalent to immediate completion

**Round-trip Testing**:
- Offboarding record serialization/deserialization
- Status transitions and reversions
- Token action execution and verification

### Test Coverage Requirements

**Functional Coverage**:
- All public functions must have corresponding unit tests
- All error conditions must be explicitly tested
- All event emissions must be verified

**Property Coverage**:
- Each acceptance criterion marked as testable must have a corresponding property test
- All system invariants must be verified through property testing
- Cross-functional properties must be tested (e.g., offboarding + attestation interactions)

**Performance Testing**:
- Load testing for batch operations with large member sets
- Stress testing for concurrent offboarding processes
- Memory usage testing for long-running transition periods

The testing strategy ensures both concrete correctness through unit tests and universal correctness through property-based testing, providing comprehensive validation of the member offboarding system.