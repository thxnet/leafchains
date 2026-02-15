//! Storage migrations for the Trustless Agent pallet
//!
//! This module handles storage migrations between pallet versions, ensuring
//! safe upgrades without data loss or corruption.

#[cfg(feature = "try-runtime")]
use frame_support::ensure;
use frame_support::{
    traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
    weights::Weight,
};
use sp_std::marker::PhantomData;

use crate::*;

/// Current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

/// Migration from v0 (no pallet deployed) to v1 (initial deployment)
pub mod v1 {
    use super::*;

    /// Initial deployment migration
    ///
    /// This migration:
    /// 1. Sets the storage version to v1
    /// 2. Initializes counter storages to 0
    /// 3. Validates no pre-existing data conflicts
    pub struct InitialDeployment<T>(PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for InitialDeployment<T> {
        fn on_runtime_upgrade() -> Weight {
            let mut weight = T::DbWeight::get().reads(1);
            let current_version = Pallet::<T>::on_chain_storage_version();

            // Only run migration if we're at version 0
            if current_version == StorageVersion::new(0) {
                // Set storage version
                STORAGE_VERSION.put::<Pallet<T>>();
                weight = weight.saturating_add(T::DbWeight::get().writes(1));

                // Initialize counters (they should already be 0, but explicit is better)
                NextAgentId::<T>::put(0u64);
                NextFeedbackId::<T>::put(0u64);
                NextAuthorizationId::<T>::put(0u64);
                NextRequestId::<T>::put(0u64);
                NextEscrowId::<T>::put(0u64);
                NextDisputeId::<T>::put(0u64);
                weight = weight.saturating_add(T::DbWeight::get().writes(6));
            }

            weight
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            let current_version = Pallet::<T>::on_chain_storage_version();

            // Ensure we're migrating from v0
            ensure!(current_version == StorageVersion::new(0), "Can only upgrade from v0 to v1");

            // Check that no data exists (this is initial deployment)
            let agent_count = NextAgentId::<T>::get();
            let feedback_count = NextFeedbackId::<T>::get();

            ensure!(agent_count == 0, "Unexpected agents found before migration");
            ensure!(feedback_count == 0, "Unexpected feedbacks found before migration");

            Ok(Vec::new())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(_state: Vec<u8>) -> Result<(), &'static str> {
            let current_version = Pallet::<T>::on_chain_storage_version();

            // Ensure we're now at v1
            ensure!(current_version == STORAGE_VERSION, "Storage version not updated correctly");

            // Verify counters are initialized
            let agent_id = NextAgentId::<T>::get();
            let feedback_id = NextFeedbackId::<T>::get();
            let auth_id = NextAuthorizationId::<T>::get();

            ensure!(agent_id == 0, "NextAgentId not initialized correctly");
            ensure!(feedback_id == 0, "NextFeedbackId not initialized correctly");
            ensure!(auth_id == 0, "NextAuthorizationId not initialized correctly");

            Ok(())
        }
    }
}

// Placeholder for future v2 migration
// pub mod v2 {
//     use super::*;
//
//     /// Example: Migration to chunked feedback storage
//     pub struct ChunkedFeedbackStorage<T>(PhantomData<T>);
//
//     impl<T: Config> OnRuntimeUpgrade for ChunkedFeedbackStorage<T> {
//         fn on_runtime_upgrade() -> Weight {
//             // Migration logic here
//             Weight::zero()
//         }
//     }
// }

/// Type alias for all migrations
///
/// To add a new migration, add it to this tuple in order:
/// ```ignore
/// pub type Migrations<T> = (
///     v1::InitialDeployment<T>,
///     v2::SomeNewMigration<T>,
/// );
/// ```
pub type Migrations<T> = (v1::InitialDeployment<T>,);

#[cfg(test)]
mod tests {
    use frame_support::traits::OnRuntimeUpgrade;

    use super::*;
    use crate::mock::*;

    #[test]
    fn initial_deployment_migration_works() {
        new_test_ext().execute_with(|| {
            // Ensure we're at version 0
            assert_eq!(Pallet::<Test>::on_chain_storage_version(), StorageVersion::new(0));

            // Run migration
            let _weight = v1::InitialDeployment::<Test>::on_runtime_upgrade();

            // Verify storage version updated
            assert_eq!(Pallet::<Test>::on_chain_storage_version(), StorageVersion::new(1));

            // Verify counters initialized
            assert_eq!(NextAgentId::<Test>::get(), 0);
            assert_eq!(NextFeedbackId::<Test>::get(), 0);
            assert_eq!(NextAuthorizationId::<Test>::get(), 0);
        });
    }

    #[test]
    fn migration_is_idempotent() {
        new_test_ext().execute_with(|| {
            // Run migration once
            v1::InitialDeployment::<Test>::on_runtime_upgrade();

            // Ensure migration completed
            assert_eq!(Pallet::<Test>::on_chain_storage_version(), StorageVersion::new(1));

            // Run migration again (should be skipped)
            let _weight = v1::InitialDeployment::<Test>::on_runtime_upgrade();

            // Version should still be v1 (not changed)
            assert_eq!(Pallet::<Test>::on_chain_storage_version(), StorageVersion::new(1));

            // Counters should still be 0
            assert_eq!(NextAgentId::<Test>::get(), 0);
        });
    }
}
