/// Storage migrations for pallet-rwa.
///
/// # Version History
///
/// - **V1–V4**: Implicit fixes and feature additions applied during early
///   development.  No on-chain `StorageVersion` marker existed at those stages,
///   so there are no migration modules for these versions.
/// - **V5** (current): Added `MinParticipationDeposit` policy enforcement. This
///   is the first version tracked by an explicit `StorageVersion`. No schema
///   change — the storage layout itself is unchanged; only the extrinsic
///   validation logic was tightened.
///
/// # How to add a future migration (e.g. V6)
///
/// 1. Create a new sub-module `pub mod v6 { ... }` below.
/// 2. Inside the module, define any `OldFoo` structs that capture the
///    **pre-migration** layout (copy the current struct and rename it).
/// 3. Implement `OnRuntimeUpgrade` for `MigrateToV6<T>`:
///    - In `on_runtime_upgrade()`: a. Guard: if on-chain version != 5, skip. b.
///      Translate / mutate storage as needed. c. Call
///      `StorageVersion::new(6).put::<pallet::Pallet<T>>()`. d. Return the
///      consumed weight.
///    - In `pre_upgrade()` (behind `try-runtime`): snapshot any counters needed
///      for post-upgrade assertions.
///    - In `post_upgrade()` (behind `try-runtime`): assert invariants.
/// 4. Wire the migration into the runtime's `Executive` type or
///    `type Migrations` tuple so it runs on the next upgrade.

/// Noop migration that confirms the on-chain storage is already at V5.
///
/// This module exists so the pallet ships with a working migration scaffold.
/// It performs no data transformations — it only logs the current version and
/// returns zero weight when the chain is already at V5.
pub mod v5 {
    use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade, weights::Weight};

    use crate::pallet::{self, Config};

    pub struct MigrateToV5<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV5<T> {
        fn on_runtime_upgrade() -> Weight {
            let on_chain = pallet::Pallet::<T>::on_chain_storage_version();

            if on_chain >= 5 {
                frame_support::log::info!(
                    target: "pallet-rwa",
                    "MigrateToV5: on-chain version is {:?}, already at V5+. Skipping.",
                    on_chain,
                );
                return Weight::zero();
            }

            // If the chain is somehow below V5 (e.g. a fresh genesis with no
            // explicit storage version), stamp V5 so future migrations have a
            // clean baseline.
            frame_support::log::info!(
                target: "pallet-rwa",
                "MigrateToV5: on-chain version is {:?}, stamping V5.",
                on_chain,
            );

            StorageVersion::new(5).put::<pallet::Pallet<T>>();

            // One write for the storage version key.
            T::DbWeight::get().writes(1)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            let on_chain = pallet::Pallet::<T>::on_chain_storage_version();
            frame_support::log::info!(
                target: "pallet-rwa",
                "MigrateToV5::pre_upgrade – on-chain version: {:?}",
                on_chain,
            );
            Ok(on_chain.encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
            let pre_version =
                StorageVersion::decode(&mut &state[..]).map_err(|_| "decode failed")?;
            let post_version = pallet::Pallet::<T>::on_chain_storage_version();

            frame_support::log::info!(
                target: "pallet-rwa",
                "MigrateToV5::post_upgrade – pre: {:?}, post: {:?}",
                pre_version,
                post_version,
            );

            frame_support::ensure!(
                post_version >= 5,
                "pallet-rwa: on-chain version should be >= 5 after MigrateToV5"
            );

            Ok(())
        }
    }
}
