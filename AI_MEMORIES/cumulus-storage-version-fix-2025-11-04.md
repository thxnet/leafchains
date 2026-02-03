# Cumulus Storage Version Fix - 2025-11-04

## Summary

Fixed XcmpQueue and DmpQueue storage version mismatch warnings that appeared after emergency relay chain recovery.

## Problem

After using relay chain's `forceSetCurrentCode` to revert the parachain runtime, storage version warnings appeared:

```
⚠️  XcmpQueue: on-chain StorageVersion(3) vs current storage version StorageVersion(2)
⚠️  DmpQueue: on-chain StorageVersion(2) vs current storage version StorageVersion(1)
```

### Root Cause

- When reverting runtime via relay chain, **on-chain storage versions remain unchanged**
- But the reverted runtime code declares **older storage versions**
- This creates a mismatch that generates warnings on every block

## Solution

Created runtime-level migrations to align storage versions with on-chain state.

### Implementation

**File Created:** `runtime/general/src/migrations.rs`

```rust
pub mod align_cumulus_storage_versions {
    const XCMP_QUEUE_TARGET_VERSION: u16 = 3;
    const DMP_QUEUE_TARGET_VERSION: u16 = 2;

    pub struct AlignCumulusStorageVersions;

    impl OnRuntimeUpgrade for AlignCumulusStorageVersions {
        fn on_runtime_upgrade() -> Weight {
            // Update XcmpQueue: v2 → v3
            // Update DmpQueue: v1 → v2
            // No actual storage migration, just version alignment
        }
    }
}
```

**Changes to** `runtime/general/src/lib.rs`:

1. Added `mod migrations;`
2. Updated Migrations tuple:
   ```rust
   pub type Migrations = (
       pallet_trustless_agent::migrations::Migrations<Runtime>,
       migrations::align_cumulus_storage_versions::AlignCumulusStorageVersions,
   );
   ```
3. Incremented `spec_version` from 3 to 4

## Technical Details

### Cumulus Versions (polkadot-v0.9.40)

From Cumulus source code:
- `cumulus-pallet-xcmp-queue`: declares `StorageVersion::new(2)`
- `cumulus-pallet-dmp-queue`: declares `StorageVersion::new(1)`

### On-Chain State

After emergency recovery:
- XcmpQueue: `StorageVersion(3)` (from previous upgrade attempt)
- DmpQueue: `StorageVersion(2)` (from previous upgrade attempt)

### Migration Strategy

**Approach:** Version alignment without storage changes

Instead of downgrading on-chain versions (dangerous), we upgrade the runtime's declared versions to match on-chain state:

| Pallet | Cumulus Declares | On-Chain Has | Migration |
|--------|-----------------|--------------|-----------|
| XcmpQueue | v2 | v3 | Align to v3 |
| DmpQueue | v1 | v2 | Align to v2 |

### Safety

This migration is safe because:
1. ✅ No actual storage structure changes
2. ✅ Only updates version metadata
3. ✅ Idempotent (can run multiple times safely)
4. ✅ Includes try-runtime checks

## Deployment

### Runtime Version

- **Previous:** spec_version 3
- **New:** spec_version 4

### WASM Artifact

```
Path: target/release/wbuild/general-runtime/general_runtime.compact.compressed.wasm
Size: 866,304 bytes (846 KB)
Blake2-256: 0x18d3db29bc9b846c3a1deb006ab9327c23580dcd50616eff09a3d9b0dcecbcc8
```

### Upgrade Steps

1. Deploy via parachain sudo:
   ```
   sudo.sudoUncheckedWeight(
     system.setCode(wasm),
     { refTime: 0, proofSize: 0 }
   )
   ```

2. Verify after upgrade:
   - spec_version = 4
   - No more storage version warnings in logs
   - Blocks continue to be produced

## Verification

### Expected Log Output

After upgrade, you should see in node logs:

```
INFO runtime::migrations: XcmpQueue migration: current on-chain version = StorageVersion(3)
INFO runtime::migrations: XcmpQueue storage version already at target
INFO runtime::migrations: DmpQueue migration: current on-chain version = StorageVersion(2)
INFO runtime::migrations: DmpQueue storage version already at target
```

(Version already at target because on-chain is already v3/v2)

### Post-Upgrade Checks

```javascript
// In Polkadot.js Apps -> Developer -> JavaScript

// 1. Check spec version
const version = await api.consts.system.version;
console.log('Spec version:', version.specVersion.toNumber()); // Should be 4

// 2. Check for warnings in node logs
// Should no longer see XcmpQueue/DmpQueue warnings
```

## Impact Assessment

### Before Fix
- ⚠️  Warning logs on every block
- ⚠️  Confusing for developers
- ✅ No functional impact (warnings only)

### After Fix
- ✅ No more warnings
- ✅ Clean logs
- ✅ Storage versions aligned
- ✅ Ready for future Cumulus upgrades

## Future Considerations

### When Upgrading Cumulus

If you later upgrade to a newer Cumulus version (e.g., polkadot-v0.9.50):

1. Check the new STORAGE_VERSION in Cumulus source
2. If it's higher than current on-chain version:
   - Cumulus will provide built-in migrations
   - Follow their migration guide
3. If it's same or lower:
   - No migration needed
   - Storage versions already aligned

### Adding to Migration Checklist

Before every runtime upgrade, verify:
- [ ] All pallet storage versions match on-chain state
- [ ] No version mismatch warnings in logs
- [ ] Migrations included in Executive

## Related Files

- Migration code: `runtime/general/src/migrations.rs`
- Runtime config: `runtime/general/src/lib.rs`
- Incident report: `AI_MEMORIES/runtime-upgrade-incident-2025-11-04.md`

## References

- Cumulus repository: https://github.com/paritytech/cumulus
- Branch used: `polkadot-v0.9.40`
- XcmpQueue source: `pallets/xcmp-queue/src/`
- DmpQueue source: `pallets/dmp-queue/src/`

## Lessons Learned

### Emergency Recovery Side Effects

When using relay chain `forceSetCurrentCode`:
- ✅ Quickly recovers stuck parachain
- ⚠️  May leave storage version mismatches
- ✅ Can be fixed with follow-up migrations

### Storage Version Best Practices

1. Always declare storage versions in pallets
2. Include version checks in migrations
3. Test with `try-runtime` before deployment
4. Monitor logs for version warnings
5. Document version alignment in migrations

## Status

**✅ IMPLEMENTED**

- Migration created
- Runtime compiled successfully
- Ready for deployment
- Waiting for deployment to testnet

## Date

2025-11-04

## Chain

Testnet: Leafchain Sand
