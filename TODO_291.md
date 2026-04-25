# TODO: Issue #291 — Implement Credential Type Hierarchy

## Plan

### 1. `contracts/quorum_proof/src/lib.rs`

#### Data Structures
- [x] Extend `CredentialTypeDef` to include optional `parent_type` field
- [x] Add new `ContractError` variants:
  - `InvalidParentType` (parent type not registered)
  - `CircularHierarchy` (detected circular dependency)
  - `CredentialTypeNotFound` (for future use)

#### DataKey variants
- [x] Add `CredentialTypeParent(u32)` - maps parent type to child types
- [x] Add `CredentialTypeChildren(u32)` - stores all child types for a parent

#### Functions to implement
- [x] `register_credential_type(admin, type_id, name, description, parent_type: Option<u32>)`
  - Validates parent exists (if provided)
  - Checks for circular dependencies
  - Stores parent relationship
  - Stores child relationship

- [x] `get_credential_type_hierarchy(type_id) -> Option<u32>`
  - Returns parent type if it exists
  - Renamed to: `get_credential_type_parent`

- [x] `get_credential_type_children(parent_id) -> Vec<u32>`
  - Returns all direct children of a parent type

- [x] `get_credential_type_ancestors(type_id) -> Vec<u32>`
  - Returns full lineage up the hierarchy

- [x] `is_child_of(child_id, parent_id) -> bool`
  - Transitive check - is child_id anywhere in parent_id's descendants
  - Renamed to: `is_credential_type_child_of`

- [x] `inherit_verification_rules(child_id) -> Vec<u32>`
  - Returns list of ancestor types whose verification rules apply
  - Returns types in order: [self, parent, grandparent, ..., root]

#### Helper functions
- [x] `validate_no_circular_dependency(type_id, potential_parent) -> bool`
  - DFS-based check to ensure adding potential_parent as parent to type_id won't create cycle
  - Renamed to: `would_create_cycle`
  
- [x] `validate_parent_exists(env, parent_type) -> bool`
  - Checks if parent type is registered
  - Renamed to: `parent_type_exists`

#### Tests
- [x] Test registering type with valid parent
- [x] Test registering type with invalid parent (not found)
- [x] Test detecting circular hierarchy (A -> B -> A)
- [x] Test three-level hierarchy (A -> B -> C)
- [x] Test getting children
- [x] Test getting ancestors
- [x] Test transitive child relationship check
- [x] Test multiple children of same parent
- [x] Test verification rule inheritance chain
- [x] Test invalid hierarchy configurations
- [x] Test backward compatibility (registering types without parent)
- [x] Test overwriting existing types

### 2. Testing & Verification
- [x] Write all 13 comprehensive tests
- [x] Verify code structure and syntax
- [ ] Run `cargo test` in `contracts/quorum_proof` (Rust toolchain issue)
- [ ] Verify generated snapshots

## Implementation Details

### Key Functions Added
1. **register_credential_type** - Extended to accept parent_type parameter
2. **get_credential_type_parent** - Get direct parent of a type
3. **get_credential_type_children** - Get all children of a type
4. **get_credential_type_ancestors** - Get full lineage
5. **is_credential_type_child_of** - Check transitive relationships
6. **inherit_verification_rules** - Get verification rule chain

### Helper Functions
1. **parent_type_exists** - Validate parent is registered
2. **would_create_cycle** - Prevent circular dependencies

### Error Variants Added
1. **InvalidParentType** (28) - Parent type not found
2. **CircularHierarchy** (29) - Would create cycle
3. **CredentialTypeNotFound** (30) - Type not registered

### DataKey Variants Added
1. **CredentialTypeParent(u32)** - Map type to parent
2. **CredentialTypeChildren(u32)** - Map parent to children

## Progress Tracking
- Status: IMPLEMENTATION COMPLETE ✓
- Tests Written: 13/13 ✓
- Documentation: COMPLETE ✓
- Code Review: READY ✓
- Test Execution: PENDING (Rust toolchain setup needed)

## Known Issues
- Rust toolchain missing manifest: "Missing manifest in toolchain 'stable-x86_64-unknown-linux-gnu'"
- Affects test execution in current WSL environment
- Code structure verified manually - all syntax appears correct

## Next Steps
1. Fix Rust toolchain installation
2. Run `cargo test` to verify all 13 tests pass
3. Generate test snapshots if needed
4. Commit changes to branch

