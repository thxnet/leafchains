//! Runtime-level migrations
//!
//! This module contains migrations that need to be executed at the runtime
//! level, typically for fixing storage version mismatches after emergency
//! recoveries.

use frame_support::{
    traits::{GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
    weights::Weight,
};

/// Migration to align Cumulus pallet storage versions with on-chain state
///
/// This migration updates the storage versions of XcmpQueue and DmpQueue
/// pallets to match the on-chain storage versions that exist after a relay
/// chain force code set recovery.
///
/// Background:
/// - After using relay chain's `forceSetCurrentCode` to revert the runtime, the
///   on-chain storage versions were higher than what the reverted code declared
/// - XcmpQueue: on-chain v3 vs code declares v2
/// - DmpQueue: on-chain v2 vs code declares v1
///
/// This migration simply updates the code's declared version to match the chain
/// state.
pub mod align_cumulus_storage_versions {
    use super::*;

    /// Target storage versions (matching on-chain state)
    const XCMP_QUEUE_TARGET_VERSION: u16 = 3;
    const DMP_QUEUE_TARGET_VERSION: u16 = 2;

    pub struct AlignCumulusStorageVersions;

    impl OnRuntimeUpgrade for AlignCumulusStorageVersions {
        fn on_runtime_upgrade() -> Weight {
            let mut weight = Weight::zero();

            let xcmp_target = StorageVersion::new(XCMP_QUEUE_TARGET_VERSION);
            let dmp_target = StorageVersion::new(DMP_QUEUE_TARGET_VERSION);

            // Update XcmpQueue storage version
            let xcmp_current =
                cumulus_pallet_xcmp_queue::Pallet::<crate::Runtime>::on_chain_storage_version();
            log::info!(
                target: "runtime::migrations",
                "XcmpQueue migration: current on-chain version = {:?}",
                xcmp_current
            );

            if xcmp_current < xcmp_target {
                log::info!(
                    target: "runtime::migrations",
                    "Updating XcmpQueue storage version to {:?}",
                    xcmp_target
                );
                xcmp_target.put::<cumulus_pallet_xcmp_queue::Pallet<crate::Runtime>>();
                weight = weight.saturating_add(crate::RocksDbWeight::get().writes(1));
            } else {
                log::info!(
                    target: "runtime::migrations",
                    "XcmpQueue storage version already at target"
                );
            }

            // Update DmpQueue storage version
            let dmp_current =
                cumulus_pallet_dmp_queue::Pallet::<crate::Runtime>::on_chain_storage_version();
            log::info!(
                target: "runtime::migrations",
                "DmpQueue migration: current on-chain version = {:?}",
                dmp_current
            );

            if dmp_current < dmp_target {
                log::info!(
                    target: "runtime::migrations",
                    "Updating DmpQueue storage version to {:?}",
                    dmp_target
                );
                dmp_target.put::<cumulus_pallet_dmp_queue::Pallet<crate::Runtime>>();
                weight = weight.saturating_add(crate::RocksDbWeight::get().writes(1));
            } else {
                log::info!(
                    target: "runtime::migrations",
                    "DmpQueue storage version already at target"
                );
            }

            weight.saturating_add(crate::RocksDbWeight::get().reads(2))
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            use codec::Encode;

            let xcmp_version =
                cumulus_pallet_xcmp_queue::Pallet::<crate::Runtime>::on_chain_storage_version();
            let dmp_version =
                cumulus_pallet_dmp_queue::Pallet::<crate::Runtime>::on_chain_storage_version();

            log::info!(
                target: "runtime::migrations",
                "Pre-upgrade: XcmpQueue version = {:?}, DmpQueue version = {:?}",
                xcmp_version,
                dmp_version
            );

            Ok(vec![])
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
            let xcmp_target = StorageVersion::new(XCMP_QUEUE_TARGET_VERSION);
            let dmp_target = StorageVersion::new(DMP_QUEUE_TARGET_VERSION);

            let post_xcmp =
                cumulus_pallet_xcmp_queue::Pallet::<crate::Runtime>::on_chain_storage_version();
            let post_dmp =
                cumulus_pallet_dmp_queue::Pallet::<crate::Runtime>::on_chain_storage_version();

            log::info!(
                target: "runtime::migrations",
                "Post-upgrade: XcmpQueue version = {:?}, DmpQueue version = {:?}",
                post_xcmp,
                post_dmp
            );

            // Verify XcmpQueue
            if post_xcmp < xcmp_target {
                return Err("XcmpQueue storage version not updated correctly");
            }

            // Verify DmpQueue
            if post_dmp < dmp_target {
                return Err("DmpQueue storage version not updated correctly");
            }

            log::info!(
                target: "runtime::migrations",
                "✅ Storage versions aligned successfully"
            );

            Ok(())
        }
    }
}
