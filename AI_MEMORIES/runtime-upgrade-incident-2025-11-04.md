# Runtime Upgrade Incident - 2025-11-04

## Summary

Successfully recovered from a failed runtime upgrade that caused the parachain to stop producing blocks, then correctly re-executed the upgrade with proper migrations.

## Incident Timeline

### 1. Initial Failed Upgrade (spec_version 2 → 3)

**Problem:**
- Runtime upgrade from v2 to v3 was submitted
- Transaction confirmed on-chain
- **Parachain stopped producing blocks (stuck)**

**Root Cause:**
```rust
// Executive was missing the Migrations parameter
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    // ❌ Missing Migrations tuple!
>;
```

**Impact:**
- TrustlessAgent pallet migrations never executed
- Storage version remained at 0 (should be 1)
- All counters uninitialized
- Parachain became unresponsive

### 2. Emergency Recovery via Relay Chain

**Solution:**
Used relay chain's `sudo.sudo(paras.forceSetCurrentCode)` to force-revert the parachain runtime to v2.

**Steps:**
1. Connected to relay chain
2. Extracted old runtime WASM from block #6079664 (before upgrade)
3. Executed: `sudo.sudo(paras.forceSetCurrentCode(para_id, old_wasm))`
4. Waited 1-2 relay chain blocks for effect
5. Parachain resumed block production at spec_version 2

**Why this works:**
- Relay chain has authority over parachain validation
- `forceSetCurrentCode` bypasses parachain consensus
- Immediately effective at next relay chain session

### 3. Fix and Correct Re-upgrade

**Fix Applied:**
```rust
// Added Migrations type
pub type Migrations = (
    pallet_trustless_agent::migrations::Migrations<Runtime>,
);

// Updated Executive to include Migrations
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,  // ✅ Now included!
>;

// Kept spec_version at 3 (to properly upgrade from reverted v2)
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_version: 3,
    // ...
};
```

**Deployment:**
1. Recompiled runtime with fixes
2. Generated new WASM:
   - Size: 865,239 bytes (844.96 KB)
   - Blake2-256: `0x8bdf224151436f4a4721b55979daf60f0b74eef907b5881f790c1bddd022d0d4`
3. Submitted upgrade via parachain sudo: `sudo.sudoUncheckedWeight(system.setCode(wasm))`
4. Upgrade successful

### 4. Verification Results

**Runtime Upgrade:**
- ✅ spec_version: 2 → 3
- ✅ Block production continues
- ✅ No errors in logs

**Migration Execution:**
- ✅ TrustlessAgent storage version: 0 → 1
- ✅ All counters initialized to 0:
  - nextAgentId: 0
  - nextFeedbackId: 0
  - nextAuthorizationId: 0
  - nextRequestId: 0
  - nextEscrowId: 0
  - nextDisputeId: 0

## Key Lessons

### 1. Always Include Migrations in Executive

When adding pallets with migrations, the Executive type MUST include the Migrations tuple:

```rust
pub type Migrations = (
    pallet_foo::migrations::Migrations<Runtime>,
    pallet_bar::migrations::Migrations<Runtime>,
);

pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
    Migrations,  // Critical!
>;
```

### 2. Test Before Mainnet

**Recommended testing flow:**
1. ✅ Unit tests: `cargo test`
2. ✅ Try-runtime: Simulate upgrade on live chain state
3. ✅ Local devnet: Test upgrade on local chain
4. ✅ Testnet: Full integration test
5. ✅ Mainnet: Only after all above pass

**Try-runtime example:**
```bash
cargo build --release --features=try-runtime

./target/release/thxnet-leafchain try-runtime \
  --runtime ./target/release/wbuild/general-runtime/general_runtime.wasm \
  on-runtime-upgrade \
  live --uri wss://testnet-rpc
```

### 3. Emergency Recovery Procedures

**For stuck parachain:**

1. **Diagnose:**
   - Check node logs for errors
   - Verify relay chain connection
   - Check if upgrade transaction confirmed

2. **Recover via Relay Chain:**
   ```javascript
   // Extract old WASM from historical block
   const blockHash = await api.rpc.chain.getBlockHash(historical_block_number);
   const apiAt = await api.at(blockHash);
   const code = await apiAt.query.system.code();

   // Force revert via relay chain
   relay_api.tx.sudo.sudo(
     relay_api.tx.paras.forceSetCurrentCode(para_id, code)
   )
   ```

3. **Fix and Re-deploy:**
   - Fix the runtime issue
   - Test thoroughly
   - Re-submit upgrade

### 4. Storage Version Management

Every pallet with storage should declare its version:

```rust
#[pallet::pallet]
#[pallet::storage_version(STORAGE_VERSION)]
pub struct Pallet<T>(_);

pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
```

Migrations should update storage version:

```rust
impl<T: Config> OnRuntimeUpgrade for MyMigration<T> {
    fn on_runtime_upgrade() -> Weight {
        if Pallet::<T>::on_chain_storage_version() == StorageVersion::new(0) {
            // Perform migration
            STORAGE_VERSION.put::<Pallet<T>>();
            // ...
        }
        weight
    }
}
```

## Technical Details

### Migration Code Structure

**Location:** `pallets/trustless-agent/src/migrations.rs`

**Implementation:**
```rust
pub mod v1 {
    pub struct InitialDeployment<T>(PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for InitialDeployment<T> {
        fn on_runtime_upgrade() -> Weight {
            let current_version = Pallet::<T>::on_chain_storage_version();

            if current_version == StorageVersion::new(0) {
                // Set storage version
                STORAGE_VERSION.put::<Pallet<T>>();

                // Initialize counters
                NextAgentId::<T>::put(0u64);
                NextFeedbackId::<T>::put(0u64);
                // ... etc
            }

            weight
        }
    }
}

pub type Migrations<T> = (v1::InitialDeployment<T>,);
```

### Files Modified

1. **runtime/general/src/lib.rs:**
   - Added `Migrations` type definition
   - Updated `Executive` type
   - Spec version remains 3

2. **pallets/trustless-agent/src/lib.rs:**
   - Already had `#[pallet::storage_version(STORAGE_VERSION)]`
   - No changes needed

3. **pallets/trustless-agent/src/migrations.rs:**
   - Already existed with correct logic
   - No changes needed

## Prevention Checklist

Before every runtime upgrade:

- [ ] All pallets with storage have `#[pallet::storage_version]`
- [ ] Migrations are defined for all storage changes
- [ ] `Migrations` tuple includes all pallet migrations
- [ ] `Executive` includes `Migrations` parameter
- [ ] `spec_version` is incremented
- [ ] Unit tests pass
- [ ] Try-runtime simulation passes
- [ ] Testnet deployment successful
- [ ] All storage migrations verified
- [ ] Block production continues after upgrade

## Monitoring After Upgrade

**Immediate checks (within 5 minutes):**
- [ ] Spec version updated
- [ ] Blocks still being produced
- [ ] No errors in node logs
- [ ] Storage versions updated
- [ ] Test basic extrinsics

**Post-upgrade validation (within 1 hour):**
- [ ] All pallet functionality tested
- [ ] Storage queries return expected values
- [ ] Events are emitted correctly
- [ ] RPC endpoints responding
- [ ] Substrate explorers show correct version

## Related Files

- Runtime config: `runtime/general/src/lib.rs`
- Pallet code: `pallets/trustless-agent/src/lib.rs`
- Migrations: `pallets/trustless-agent/src/migrations.rs`
- Verification script: `tools/verify-runtime-upgrade.js`
- Test script: `tools/test-trustless-agent.js`

## Status

**✅ RESOLVED**

- Parachain recovered successfully
- Runtime upgraded correctly to v3
- All migrations executed
- System fully operational

## Date

2025-11-04

## Chain

Testnet: Leafchain Sand
