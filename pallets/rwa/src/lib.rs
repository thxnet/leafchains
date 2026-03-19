#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

pub mod migrations;
pub mod types;
pub mod weights;

pub use pallet::*;
pub use types::*;
pub use weights::WeightInfo;

#[cfg(test)]
mod attack_tests;
#[cfg(any(test, all(feature = "std", feature = "runtime-benchmarks")))]
pub mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{
            tokens::fungibles, Currency, ExistenceRequirement, ReservableCurrency, WithdrawReasons,
        },
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{AccountIdConversion, Saturating, Zero},
        Permill,
    };
    use sp_std::vec::Vec;

    use super::*;

    pub type BalanceOf<T> = <<T as Config>::NativeCurrency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    pub type AssetInfoOf<T> = AssetInfo<
        <T as frame_system::Config>::AccountId,
        BalanceOf<T>,
        <T as frame_system::Config>::BlockNumber,
        <T as Config>::AssetId,
        <T as Config>::MaxMetadataLen,
    >;

    pub type ParticipationOf<T> = Participation<
        <T as frame_system::Config>::AccountId,
        BalanceOf<T>,
        <T as frame_system::Config>::BlockNumber,
        <T as Config>::MaxGroupSize,
    >;

    pub type SlashRecipientOf<T> = SlashRecipient<<T as frame_system::Config>::AccountId>;

    /// Current storage version.
    ///
    /// V5 corresponds to the `MinParticipationDeposit` policy addition.
    /// Without a declared `StorageVersion`, future runtime upgrades cannot
    /// detect which migration has been applied, risking data corruption.
    const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type AssetId: Member + Parameter + Copy + MaxEncodedLen;
        type NativeCurrency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
        type Fungibles: fungibles::Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = BalanceOf<Self>>
            + fungibles::Mutate<Self::AccountId>
            + fungibles::Transfer<Self::AccountId>;
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        #[pallet::constant]
        type PalletId: Get<PalletId>;
        #[pallet::constant]
        type AssetRegistrationDeposit: Get<BalanceOf<Self>>;
        #[pallet::constant]
        type MaxAssetsPerOwner: Get<u32>;
        #[pallet::constant]
        type MaxMetadataLen: Get<u32>;
        #[pallet::constant]
        type MaxSlashRecipients: Get<u32>;
        #[pallet::constant]
        type MaxGroupSize: Get<u32>;
        #[pallet::constant]
        type MaxPendingApprovals: Get<u32>;
        #[pallet::constant]
        type MaxSunsettingPerBlock: Get<u32>;
        #[pallet::constant]
        type MaxParticipationsPerHolder: Get<u32>;
        /// V5 fix: minimum deposit required for participation policies.
        /// Prevents zero-deposit licenses that provide false trust signals.
        /// Set to zero to disable this check.
        #[pallet::constant]
        type MinParticipationDeposit: Get<BalanceOf<Self>>;
        type WeightInfo: WeightInfo;
        type ParticipationFilter: ParticipationFilter<Self::AccountId>;
        /// Optional guard that checks cross-pallet constraints before
        /// asset retirement or participation slashing.
        type AssetLifecycleGuard: AssetLifecycleGuard<Self::AccountId>;
    }

    // ── Storage ──────────────────────────────────────────────────────────

    #[pallet::storage]
    pub type RwaAssets<T: Config> = StorageMap<_, Blake2_128Concat, u32, AssetInfoOf<T>>;

    #[pallet::storage]
    pub type NextRwaAssetId<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    pub type Participations<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, u32, ParticipationOf<T>>;

    #[pallet::storage]
    pub type NextParticipationId<T: Config> = StorageMap<_, Blake2_128Concat, u32, u32, ValueQuery>;

    #[pallet::storage]
    pub type HolderIndex<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, T::AccountId, u32>;

    #[pallet::storage]
    pub type PendingApprovals<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, BoundedVec<u32, T::MaxPendingApprovals>, ValueQuery>;

    #[pallet::storage]
    pub type AssetSlashDistribution<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        u32,
        BoundedVec<SlashRecipientOf<T>, T::MaxSlashRecipients>,
    >;

    #[pallet::storage]
    pub type SunsettingAssets<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::BlockNumber,
        BoundedVec<u32, T::MaxSunsettingPerBlock>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type OwnerAssets<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<u32, T::MaxAssetsPerOwner>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type HolderAssets<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<u32, T::MaxParticipationsPerHolder>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type PendingOwnershipTransfer<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, T::AccountId>;

    // ── Events ───────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        #[codec(index = 0)]
        AssetRegistered { asset_id: u32, owner: T::AccountId, beneficiary: T::AccountId },
        #[codec(index = 1)]
        AssetPolicyUpdated { asset_id: u32 },
        #[codec(index = 2)]
        AssetDeactivated { asset_id: u32 },
        #[codec(index = 3)]
        AssetReactivated { asset_id: u32 },
        #[codec(index = 4)]
        AssetSunsetting { asset_id: u32, expiry_block: T::BlockNumber },
        #[codec(index = 5)]
        AssetRetired { asset_id: u32, deposit_returned: BalanceOf<T> },
        #[codec(index = 6)]
        ParticipationRequested {
            asset_id: u32,
            participation_id: u32,
            payer: T::AccountId,
            holders: Vec<T::AccountId>,
        },
        #[codec(index = 7)]
        ParticipationApproved { asset_id: u32, participation_id: u32 },
        #[codec(index = 8)]
        ParticipationRejected {
            asset_id: u32,
            participation_id: u32,
            deposit_refunded: BalanceOf<T>,
            fee_refunded: BalanceOf<T>,
        },
        #[codec(index = 9)]
        ParticipationExited { asset_id: u32, participation_id: u32, deposit_refunded: BalanceOf<T> },
        #[codec(index = 10)]
        ParticipationExpired {
            asset_id: u32,
            participation_id: u32,
            deposit_refunded: BalanceOf<T>,
        },
        #[codec(index = 11)]
        ParticipationRenewed {
            asset_id: u32,
            participation_id: u32,
            new_expires_at: Option<T::BlockNumber>,
        },
        #[codec(index = 12)]
        ParticipationSlashed {
            asset_id: u32,
            participation_id: u32,
            amount: BalanceOf<T>,
            reporter: Option<T::AccountId>,
        },
        #[codec(index = 13)]
        ParticipationRevoked {
            asset_id: u32,
            participation_id: u32,
            deposit_refunded: BalanceOf<T>,
        },
        #[codec(index = 14)]
        HolderAdded { asset_id: u32, participation_id: u32, holder: T::AccountId },
        #[codec(index = 15)]
        HolderRemoved { asset_id: u32, participation_id: u32, holder: T::AccountId },
        #[codec(index = 16)]
        HolderLeft { asset_id: u32, participation_id: u32, holder: T::AccountId },
        #[codec(index = 17)]
        SlashDistributionSet { asset_id: u32, recipient_count: u32 },
        #[codec(index = 18)]
        OwnershipTransferProposed { asset_id: u32, from: T::AccountId, to: T::AccountId },
        #[codec(index = 19)]
        OwnershipTransferred { asset_id: u32, old_owner: T::AccountId, new_owner: T::AccountId },
        #[codec(index = 20)]
        OwnershipTransferCancelled { asset_id: u32 },
        #[codec(index = 21)]
        BeneficiaryUpdated {
            asset_id: u32,
            old_beneficiary: T::AccountId,
            new_beneficiary: T::AccountId,
        },
        #[codec(index = 22)]
        MetadataUpdated { asset_id: u32 },
        #[codec(index = 23)]
        ParticipationTransferred {
            asset_id: u32,
            participation_id: u32,
            old_payer: T::AccountId,
            new_payer: T::AccountId,
        },
        #[codec(index = 24)]
        AssetPaused { asset_id: u32 },
        #[codec(index = 25)]
        AssetUnpaused { asset_id: u32 },
        /// CAT-7.2-R-S: all pending approvals were batch-rejected.
        #[codec(index = 26)]
        BatchPendingRejected { asset_id: u32, count: u32 },
    }

    // ── Errors ───────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        #[codec(index = 0)]
        AssetNotFound,
        #[codec(index = 1)]
        NotAssetOwner,
        #[codec(index = 2)]
        InvalidAssetStatus,
        #[codec(index = 3)]
        AssetAlreadyRetired,
        #[codec(index = 4)]
        ExpiryBlockInPast,
        #[codec(index = 5)]
        ExpiryNotReached,
        #[codec(index = 6)]
        SunsettingSlotsFull,
        #[codec(index = 7)]
        MaxAssetsPerOwnerReached,
        #[codec(index = 8)]
        MetadataTooLong,
        #[codec(index = 9)]
        ParticipationNotFound,
        #[codec(index = 10)]
        NotPayer,
        #[codec(index = 11)]
        NotHolder,
        #[codec(index = 12)]
        InvalidParticipationStatus,
        #[codec(index = 13)]
        ParticipationExpiredError,
        #[codec(index = 14)]
        MaxParticipantsReached,
        #[codec(index = 15)]
        HolderAlreadyExists,
        #[codec(index = 16)]
        HolderNotFound,
        #[codec(index = 17)]
        MaxGroupSizeReached,
        #[codec(index = 18)]
        AlreadyParticipating,
        #[codec(index = 19)]
        MaxParticipationsPerHolderReached,
        #[codec(index = 20)]
        SlashSharesSumInvalid,
        #[codec(index = 21)]
        AssetNotActive,
        #[codec(index = 22)]
        PendingApprovalsFull,
        #[codec(index = 23)]
        PolicyFieldImmutable,
        #[codec(index = 24)]
        MaxParticipantsBelowCurrent,
        #[codec(index = 25)]
        EmptyHoldersList,
        #[codec(index = 26)]
        SlashAmountExceedsDeposit,
        #[codec(index = 27)]
        TransferToSelf,
        #[codec(index = 28)]
        NoPendingTransfer,
        #[codec(index = 29)]
        NotPendingOwner,
        #[codec(index = 30)]
        ParticipantNotEligible,
        #[codec(index = 31)]
        AssetIdOverflow,
        #[codec(index = 32)]
        ParticipationIdOverflow,
        /// V5: policy deposit is below `MinParticipationDeposit`.
        #[codec(index = 33)]
        DepositBelowMinimum,
        /// Blocked by cross-pallet guard (e.g., active campaigns linked to this
        /// asset).
        #[codec(index = 34)]
        BlockedByLifecycleGuard,
    }

    // ── Hooks ────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        #[cfg(feature = "try-runtime")]
        fn try_state(_n: T::BlockNumber) -> Result<(), &'static str> { Self::do_try_state() }

        fn on_initialize(n: T::BlockNumber) -> Weight {
            let mut weight = T::DbWeight::get().reads(1);
            let asset_ids = SunsettingAssets::<T>::take(&n);
            if asset_ids.is_empty() {
                return weight;
            }
            for asset_id in asset_ids.iter() {
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 2));
                if let Some(mut asset) = RwaAssets::<T>::get(asset_id) {
                    if matches!(asset.status, AssetStatus::Retired) {
                        continue;
                    }
                    let deposit = asset.registration_deposit;
                    T::NativeCurrency::unreserve(&asset.owner, deposit);
                    asset.status = AssetStatus::Retired;
                    RwaAssets::<T>::insert(asset_id, &asset);
                    Self::remove_from_owner_assets(&asset.owner, *asset_id);
                    PendingOwnershipTransfer::<T>::remove(asset_id);
                    AssetSlashDistribution::<T>::remove(asset_id);
                    PendingApprovals::<T>::remove(asset_id);
                    Self::deposit_event(Event::AssetRetired {
                        asset_id: *asset_id,
                        deposit_returned: deposit,
                    });
                }
            }
            weight
        }
    }

    // ── Dispatchables ────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // ─── Asset management ────────────────────────────────────────

        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_asset())]
        pub fn register_asset(
            origin: OriginFor<T>,
            beneficiary: T::AccountId,
            policy: AssetPolicy<BalanceOf<T>, T::BlockNumber, T::AssetId>,
            metadata: Vec<u8>,
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;

            // Validate all inputs BEFORE touching any funds, so that a validation
            // failure never leaves a partially-modified state (reserved deposit
            // that should not have been reserved).

            // Use the correct error variant for metadata being too long.
            let bounded_meta: BoundedVec<u8, T::MaxMetadataLen> =
                metadata.try_into().map_err(|_| Error::<T>::MetadataTooLong)?;

            // Check per-owner asset limit with a plain read — no storage write needed.
            // Substrate is single-threaded per block, so this read is stable within
            // one extrinsic; no concurrent modification can consume capacity between
            // this check and the push below.
            let current_count = OwnerAssets::<T>::get(&owner).len() as u32;
            ensure!(
                current_count < T::MaxAssetsPerOwner::get(),
                Error::<T>::MaxAssetsPerOwnerReached
            );

            // V5 fix: enforce minimum participation deposit so that zero-deposit
            // licenses cannot be created, preventing false trust signals and
            // ensuring economic backstop for slashing.
            ensure!(
                policy.deposit >= T::MinParticipationDeposit::get(),
                Error::<T>::DepositBelowMinimum
            );

            // All validation passed — now reserve the deposit.
            let deposit = T::AssetRegistrationDeposit::get();
            T::NativeCurrency::reserve(&owner, deposit)?;

            let asset_id = NextRwaAssetId::<T>::get();
            let next_id = asset_id.checked_add(1).ok_or(Error::<T>::AssetIdOverflow)?;
            NextRwaAssetId::<T>::put(next_id);

            let now = frame_system::Pallet::<T>::block_number();
            let info = AssetInfo {
                owner: owner.clone(),
                beneficiary: beneficiary.clone(),
                status: AssetStatus::Active,
                policy,
                metadata: bounded_meta,
                participant_count: 0,
                registration_deposit: deposit,
                created_at: now,
            };
            RwaAssets::<T>::insert(asset_id, info);

            OwnerAssets::<T>::mutate(&owner, |assets| {
                let _ = assets.try_push(asset_id);
            });

            Self::deposit_event(Event::AssetRegistered { asset_id, owner, beneficiary });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::update_asset_policy())]
        pub fn update_asset_policy(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            new_policy: AssetPolicy<BalanceOf<T>, T::BlockNumber, T::AssetId>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            RwaAssets::<T>::try_mutate(rwa_asset_id, |maybe| -> DispatchResult {
                let asset = maybe.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(asset.owner == who, Error::<T>::NotAssetOwner);
                ensure!(
                    matches!(asset.status, AssetStatus::Active),
                    Error::<T>::InvalidAssetStatus
                );
                ensure!(
                    asset.policy.deposit_currency == new_policy.deposit_currency,
                    Error::<T>::PolicyFieldImmutable
                );
                ensure!(
                    asset.policy.deposit == new_policy.deposit,
                    Error::<T>::PolicyFieldImmutable
                );
                if let Some(max) = new_policy.max_participants {
                    ensure!(
                        max >= asset.participant_count,
                        Error::<T>::MaxParticipantsBelowCurrent
                    );
                }
                // V4 fix: prevent max_duration reduction when active participants
                // exist.  Reducing max_duration is a bait-and-switch that traps
                // existing licensees with shorter renewal terms than they signed up
                // for.  Increases are always allowed.
                //
                // HIGH-01 fix: also prevent entry_fee changes when participants
                // exist.  Raising entry_fee is a bait-and-switch on existing
                // participants who need to renew.  Lowering entry_fee is unfair
                // to those who already paid the higher fee.
                if asset.participant_count > 0 {
                    ensure!(
                        asset.policy.entry_fee == new_policy.entry_fee,
                        Error::<T>::PolicyFieldImmutable
                    );
                    match (asset.policy.max_duration, new_policy.max_duration) {
                        // Had a duration, new is shorter → block
                        (Some(old), Some(new)) => {
                            ensure!(new >= old, Error::<T>::PolicyFieldImmutable);
                        }
                        // Had unlimited, now adding a limit → block
                        (None, Some(_)) => {
                            return Err(Error::<T>::PolicyFieldImmutable.into());
                        }
                        // Keeping unlimited or removing limit → OK
                        _ => {}
                    }
                }
                asset.policy = new_policy;
                Ok(())
            })?;
            Self::deposit_event(Event::AssetPolicyUpdated { asset_id: rwa_asset_id });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::deactivate_asset())]
        pub fn deactivate_asset(origin: OriginFor<T>, rwa_asset_id: u32) -> DispatchResult {
            let who = ensure_signed(origin)?;
            RwaAssets::<T>::try_mutate(rwa_asset_id, |maybe| -> DispatchResult {
                let asset = maybe.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(asset.owner == who, Error::<T>::NotAssetOwner);
                ensure!(
                    matches!(asset.status, AssetStatus::Active),
                    Error::<T>::InvalidAssetStatus
                );
                asset.status = AssetStatus::Inactive;
                Ok(())
            })?;
            Self::deposit_event(Event::AssetDeactivated { asset_id: rwa_asset_id });
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::reactivate_asset())]
        pub fn reactivate_asset(origin: OriginFor<T>, rwa_asset_id: u32) -> DispatchResult {
            let who = ensure_signed(origin)?;
            RwaAssets::<T>::try_mutate(rwa_asset_id, |maybe| -> DispatchResult {
                let asset = maybe.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(asset.owner == who, Error::<T>::NotAssetOwner);
                ensure!(
                    matches!(asset.status, AssetStatus::Inactive),
                    Error::<T>::InvalidAssetStatus
                );
                asset.status = AssetStatus::Active;
                Ok(())
            })?;
            Self::deposit_event(Event::AssetReactivated { asset_id: rwa_asset_id });
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::sunset_asset())]
        pub fn sunset_asset(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            expiry_block: T::BlockNumber,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(expiry_block > now, Error::<T>::ExpiryBlockInPast);

            // Pre-flight check: verify the sunsetting slot has capacity BEFORE
            // making any state changes. Substrate block execution is single-threaded
            // so this read is stable — no interleaving can consume the slot between
            // this check and the write below.
            let current_count = SunsettingAssets::<T>::get(&expiry_block).len() as u32;
            ensure!(
                current_count < T::MaxSunsettingPerBlock::get(),
                Error::<T>::SunsettingSlotsFull
            );

            // Update asset status. If this fails (asset not found, wrong owner,
            // wrong status), neither storage item is modified — no corruption.
            RwaAssets::<T>::try_mutate(rwa_asset_id, |maybe| -> DispatchResult {
                let asset = maybe.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(asset.owner == who, Error::<T>::NotAssetOwner);
                ensure!(
                    matches!(asset.status, AssetStatus::Active | AssetStatus::Inactive),
                    Error::<T>::InvalidAssetStatus
                );
                asset.status = AssetStatus::Sunsetting { expiry_block };
                Ok(())
            })?;

            // Asset status confirmed and updated — now register in the schedule.
            // The capacity check above guarantees this push succeeds.
            SunsettingAssets::<T>::mutate(&expiry_block, |vec| {
                let _ = vec.try_push(rwa_asset_id);
            });

            Self::deposit_event(Event::AssetSunsetting { asset_id: rwa_asset_id, expiry_block });
            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::force_retire_asset())]
        pub fn force_retire_asset(origin: OriginFor<T>, rwa_asset_id: u32) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(!matches!(asset.status, AssetStatus::Retired), Error::<T>::AssetAlreadyRetired);

            // CRIT-03: check cross-pallet guard before any state changes.
            T::AssetLifecycleGuard::can_retire_asset(rwa_asset_id)
                .map_err(|_| Error::<T>::BlockedByLifecycleGuard)?;

            if let AssetStatus::Sunsetting { expiry_block } = asset.status {
                SunsettingAssets::<T>::mutate(expiry_block, |ids| {
                    if let Some(pos) = ids.iter().position(|&id| id == rwa_asset_id) {
                        ids.remove(pos);
                    }
                });
            }

            let deposit = asset.registration_deposit;
            T::NativeCurrency::unreserve(&asset.owner, deposit);
            Self::remove_from_owner_assets(&asset.owner, rwa_asset_id);
            PendingOwnershipTransfer::<T>::remove(rwa_asset_id);
            AssetSlashDistribution::<T>::remove(rwa_asset_id);
            PendingApprovals::<T>::remove(rwa_asset_id);

            RwaAssets::<T>::mutate(rwa_asset_id, |maybe| {
                if let Some(a) = maybe {
                    a.status = AssetStatus::Retired;
                }
            });

            Self::deposit_event(Event::AssetRetired {
                asset_id: rwa_asset_id,
                deposit_returned: deposit,
            });
            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::retire_asset())]
        pub fn retire_asset(origin: OriginFor<T>, rwa_asset_id: u32) -> DispatchResult {
            ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            let expiry = match asset.status {
                AssetStatus::Sunsetting { expiry_block } => expiry_block,
                _ => return Err(Error::<T>::InvalidAssetStatus.into()),
            };
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(now >= expiry, Error::<T>::ExpiryNotReached);

            let deposit = asset.registration_deposit;
            T::NativeCurrency::unreserve(&asset.owner, deposit);
            Self::remove_from_owner_assets(&asset.owner, rwa_asset_id);
            // C-1: clean up pending ownership transfer (retire_asset left this behind)
            PendingOwnershipTransfer::<T>::remove(rwa_asset_id);
            // C-2: clean up sunsetting schedule entry (expiry_block was confirmed above)
            SunsettingAssets::<T>::mutate(expiry, |ids| {
                if let Some(pos) = ids.iter().position(|&id| id == rwa_asset_id) {
                    ids.remove(pos);
                }
            });
            // M-3: clean up slash distribution config
            AssetSlashDistribution::<T>::remove(rwa_asset_id);
            // M-4: clean up pending approvals queue
            PendingApprovals::<T>::remove(rwa_asset_id);

            RwaAssets::<T>::mutate(rwa_asset_id, |maybe| {
                if let Some(a) = maybe {
                    a.status = AssetStatus::Retired;
                }
            });

            Self::deposit_event(Event::AssetRetired {
                asset_id: rwa_asset_id,
                deposit_returned: deposit,
            });
            Ok(())
        }

        // ─── Participation ───────────────────────────────────────────

        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::request_participation())]
        pub fn request_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            holders: Vec<T::AccountId>,
        ) -> DispatchResult {
            let payer = ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(matches!(asset.status, AssetStatus::Active), Error::<T>::AssetNotActive);

            ensure!(!holders.is_empty(), Error::<T>::EmptyHoldersList);
            let bounded_holders: BoundedVec<T::AccountId, T::MaxGroupSize> =
                holders.clone().try_into().map_err(|_| Error::<T>::MaxGroupSizeReached)?;

            // uniqueness
            let mut sorted = bounded_holders.clone().into_inner();
            sorted.sort();
            sorted.dedup();
            ensure!(sorted.len() == bounded_holders.len(), Error::<T>::HolderAlreadyExists);

            // max_participants
            if let Some(max) = asset.policy.max_participants {
                ensure!(
                    asset.participant_count.saturating_add(1) <= max,
                    Error::<T>::MaxParticipantsReached
                );
            }

            // holder eligibility
            for h in bounded_holders.iter() {
                ensure!(
                    !HolderIndex::<T>::contains_key(rwa_asset_id, h),
                    Error::<T>::AlreadyParticipating
                );
                let h_assets = HolderAssets::<T>::get(h);
                ensure!(
                    (h_assets.len() as u32) < T::MaxParticipationsPerHolder::get(),
                    Error::<T>::MaxParticipationsPerHolderReached
                );
            }

            // KYC / whitelist filter
            for h in bounded_holders.iter() {
                T::ParticipationFilter::ensure_eligible(rwa_asset_id, h)
                    .map_err(|_| Error::<T>::ParticipantNotEligible)?;
            }

            // Pre-flight check for PendingApprovals capacity. Do this BEFORE any
            // state mutations so that a full approval queue never causes
            // participant_count inflation (count incremented but push never lands).
            if asset.policy.requires_approval {
                let current_pending = PendingApprovals::<T>::get(rwa_asset_id).len() as u32;
                ensure!(
                    current_pending < T::MaxPendingApprovals::get(),
                    Error::<T>::PendingApprovalsFull
                );
            }

            let deposit_amount = asset.policy.deposit;
            let fee_amount = asset.policy.entry_fee;
            let currency = &asset.policy.deposit_currency;
            let pallet_acct = Self::pallet_account();

            if asset.policy.requires_approval {
                // hold deposit + fee in escrow until approval
                let total = deposit_amount.saturating_add(fee_amount);
                Self::do_transfer(currency, &payer, &pallet_acct, total)?;
            } else {
                // deposit → escrow, fee → beneficiary
                Self::do_transfer(currency, &payer, &pallet_acct, deposit_amount)?;
                if !fee_amount.is_zero() {
                    Self::do_transfer(currency, &payer, &asset.beneficiary, fee_amount)?;
                }
            }

            let pid = NextParticipationId::<T>::get(rwa_asset_id);
            let next_pid = pid.checked_add(1).ok_or(Error::<T>::ParticipationIdOverflow)?;
            NextParticipationId::<T>::insert(rwa_asset_id, next_pid);

            let now = frame_system::Pallet::<T>::block_number();
            let status = if asset.policy.requires_approval {
                ParticipationStatus::PendingApproval
            } else {
                let expires_at = asset.policy.max_duration.map(|d| now.saturating_add(d));
                ParticipationStatus::Active { started_at: now, expires_at }
            };

            let participation = Participation {
                rwa_asset_id,
                payer: payer.clone(),
                holders: bounded_holders.clone(),
                status,
                deposit_held: deposit_amount,
                entry_fee_paid: fee_amount,
            };
            Participations::<T>::insert(rwa_asset_id, pid, &participation);

            // indexes
            for h in bounded_holders.iter() {
                HolderIndex::<T>::insert(rwa_asset_id, h, pid);
                Self::push_holder_asset(h, rwa_asset_id);
            }

            // Increment participant_count. The capacity pre-flight above guarantees
            // the subsequent PendingApprovals push cannot fail, so there is no risk
            // of count being incremented without a matching pending record.
            RwaAssets::<T>::mutate(rwa_asset_id, |maybe| {
                if let Some(a) = maybe {
                    a.participant_count = a.participant_count.saturating_add(1);
                }
            });

            if asset.policy.requires_approval {
                // Capacity was confirmed above — plain mutate is safe here.
                PendingApprovals::<T>::mutate(rwa_asset_id, |vec| {
                    let _ = vec.try_push(pid);
                });
            }

            Self::deposit_event(Event::ParticipationRequested {
                asset_id: rwa_asset_id,
                participation_id: pid,
                payer,
                holders,
            });

            if !asset.policy.requires_approval {
                Self::deposit_event(Event::ParticipationApproved {
                    asset_id: rwa_asset_id,
                    participation_id: pid,
                });
            }

            Ok(())
        }

        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::approve_participation())]
        pub fn approve_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
        ) -> DispatchResult {
            Self::ensure_asset_owner_or_admin(origin, rwa_asset_id)?;

            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            // C-1 fix: block approval on non-Active assets (Paused, Inactive,
            // Sunsetting, Retired). request_participation already enforces Active;
            // approval must be equally strict to prevent circumventing pause/sunset.
            ensure!(matches!(asset.status, AssetStatus::Active), Error::<T>::AssetNotActive);

            Participations::<T>::try_mutate(
                rwa_asset_id,
                participation_id,
                |maybe| -> DispatchResult {
                    let p = maybe.as_mut().ok_or(Error::<T>::ParticipationNotFound)?;
                    ensure!(
                        matches!(p.status, ParticipationStatus::PendingApproval),
                        Error::<T>::InvalidParticipationStatus
                    );

                    // transfer entry_fee from escrow to beneficiary
                    let fee = p.entry_fee_paid;
                    if !fee.is_zero() {
                        Self::do_transfer(
                            &asset.policy.deposit_currency,
                            &Self::pallet_account(),
                            &asset.beneficiary,
                            fee,
                        )?;
                    }

                    let now = frame_system::Pallet::<T>::block_number();
                    let expires_at = asset.policy.max_duration.map(|d| now.saturating_add(d));
                    p.status = ParticipationStatus::Active { started_at: now, expires_at };
                    Ok(())
                },
            )?;

            // remove from pending list
            PendingApprovals::<T>::mutate(rwa_asset_id, |vec| {
                if let Some(pos) = vec.iter().position(|&id| id == participation_id) {
                    vec.remove(pos);
                }
            });

            Self::deposit_event(Event::ParticipationApproved {
                asset_id: rwa_asset_id,
                participation_id,
            });
            Ok(())
        }

        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::reject_participation())]
        pub fn reject_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
        ) -> DispatchResult {
            Self::ensure_asset_owner_or_admin(origin, rwa_asset_id)?;

            let p = Participations::<T>::get(rwa_asset_id, participation_id)
                .ok_or(Error::<T>::ParticipationNotFound)?;
            ensure!(
                matches!(p.status, ParticipationStatus::PendingApproval),
                Error::<T>::InvalidParticipationStatus
            );

            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            let total = p.deposit_held.saturating_add(p.entry_fee_paid);
            Self::do_transfer(
                &asset.policy.deposit_currency,
                &Self::pallet_account(),
                &p.payer,
                total,
            )?;

            // cleanup
            Self::remove_all_holder_indexes(rwa_asset_id, &p.holders);
            Participations::<T>::remove(rwa_asset_id, participation_id);

            RwaAssets::<T>::mutate(rwa_asset_id, |maybe| {
                if let Some(a) = maybe {
                    a.participant_count = a.participant_count.saturating_sub(1);
                }
            });

            PendingApprovals::<T>::mutate(rwa_asset_id, |vec| {
                if let Some(pos) = vec.iter().position(|&id| id == participation_id) {
                    vec.remove(pos);
                }
            });

            Self::deposit_event(Event::ParticipationRejected {
                asset_id: rwa_asset_id,
                participation_id,
                deposit_refunded: p.deposit_held,
                fee_refunded: p.entry_fee_paid,
            });
            Ok(())
        }

        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::exit_participation())]
        pub fn exit_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut p = Participations::<T>::get(rwa_asset_id, participation_id)
                .ok_or(Error::<T>::ParticipationNotFound)?;
            ensure!(p.payer == who, Error::<T>::NotPayer);

            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;

            // lazy expiry — if expired, settle and return Ok
            if Self::try_settle_expiry(rwa_asset_id, participation_id, &mut p, &asset)? {
                return Ok(());
            }

            ensure!(
                matches!(p.status, ParticipationStatus::Active { .. }),
                Error::<T>::InvalidParticipationStatus
            );

            let deposit = p.deposit_held;
            Self::do_transfer(
                &asset.policy.deposit_currency,
                &Self::pallet_account(),
                &p.payer,
                deposit,
            )?;

            p.status = ParticipationStatus::Exited;
            p.deposit_held = Zero::zero();
            Participations::<T>::insert(rwa_asset_id, participation_id, &p);

            Self::remove_all_holder_indexes(rwa_asset_id, &p.holders);
            Self::dec_participant_count(rwa_asset_id);

            Self::deposit_event(Event::ParticipationExited {
                asset_id: rwa_asset_id,
                participation_id,
                deposit_refunded: deposit,
            });
            Ok(())
        }

        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::renew_participation())]
        pub fn renew_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;

            ensure!(matches!(asset.status, AssetStatus::Active), Error::<T>::AssetNotActive);

            // Phase 1: lazy expiry settlement (all side effects committed here,
            // before the renewal try_mutate).
            //
            // try_settle_expiry reads the participation, and if it is logically
            // expired (Active + expires_at in the past), transfers the deposit back
            // to the payer and writes the Expired status to storage.  This is done
            // OUTSIDE the renewal try_mutate so that external storage mutations
            // (HolderIndex, HolderAssets, participant_count) are not in a half-committed
            // state if the subsequent renewal transfer fails.
            {
                let mut p = Participations::<T>::get(rwa_asset_id, participation_id)
                    .ok_or(Error::<T>::ParticipationNotFound)?;
                ensure!(p.payer == who, Error::<T>::NotPayer);
                // Settle if logically expired. Returns true if a settlement occurred.
                Self::try_settle_expiry(rwa_asset_id, participation_id, &mut p, &asset)?;
            }

            // Phase 2: renewal — re-read the (potentially updated) participation.
            Participations::<T>::try_mutate(
                rwa_asset_id,
                participation_id,
                |maybe| -> DispatchResult {
                    let p = maybe.as_mut().ok_or(Error::<T>::ParticipationNotFound)?;
                    ensure!(p.payer == who, Error::<T>::NotPayer);

                    // The participation is now in its settled state.  Accept Active
                    // (not yet expired) OR Expired (just settled or previously settled).
                    let deposit_amount = asset.policy.deposit;
                    match p.status {
                        ParticipationStatus::Active { .. } => {
                            // Deposit already in escrow — only charge entry_fee
                            // again.
                        }
                        ParticipationStatus::Expired => {
                            // Deposit was returned when expiry settled — re-collect it.
                            let pallet_acct = Self::pallet_account();
                            Self::do_transfer(
                                &asset.policy.deposit_currency,
                                &who,
                                &pallet_acct,
                                deposit_amount,
                            )?;
                            p.deposit_held = deposit_amount;
                            // Re-add holder indexes that were removed on expiry.
                            // Guard against the case where a holder has since joined a
                            // different participation for this same asset while this one
                            // was expired.  Overwriting the index would corrupt it.
                            for h in p.holders.iter() {
                                ensure!(
                                    !HolderIndex::<T>::contains_key(rwa_asset_id, h),
                                    Error::<T>::AlreadyParticipating
                                );
                            }
                            // M-6: pre-flight capacity check for HolderAssets BEFORE
                            // pushing.  push_holder_asset silently ignores push failures;
                            // we must validate capacity here to avoid silent data loss.
                            for h in p.holders.iter() {
                                let h_assets = HolderAssets::<T>::get(h);
                                ensure!(
                                    (h_assets.len() as u32) < T::MaxParticipationsPerHolder::get(),
                                    Error::<T>::MaxParticipationsPerHolderReached
                                );
                            }
                            for h in p.holders.iter() {
                                HolderIndex::<T>::insert(rwa_asset_id, h, participation_id);
                                Self::push_holder_asset(h, rwa_asset_id);
                            }
                            // Restore participant count removed on expiry.
                            Self::inc_participant_count(rwa_asset_id);
                        }
                        _ => return Err(Error::<T>::InvalidParticipationStatus.into()),
                    }

                    // KYC / whitelist filter on renewal
                    for h in p.holders.iter() {
                        T::ParticipationFilter::ensure_eligible(rwa_asset_id, h)
                            .map_err(|_| Error::<T>::ParticipantNotEligible)?;
                    }

                    // Charge entry_fee for the new term.
                    let fee = asset.policy.entry_fee;
                    if !fee.is_zero() {
                        Self::do_transfer(
                            &asset.policy.deposit_currency,
                            &who,
                            &asset.beneficiary,
                            fee,
                        )?;
                    }

                    let now = frame_system::Pallet::<T>::block_number();
                    let new_expires_at = asset.policy.max_duration.map(|d| now.saturating_add(d));
                    p.status =
                        ParticipationStatus::Active { started_at: now, expires_at: new_expires_at };

                    Self::deposit_event(Event::ParticipationRenewed {
                        asset_id: rwa_asset_id,
                        participation_id,
                        new_expires_at,
                    });
                    Ok(())
                },
            )
        }

        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::settle_expired_participation())]
        pub fn settle_expired_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            let mut p = Participations::<T>::get(rwa_asset_id, participation_id)
                .ok_or(Error::<T>::ParticipationNotFound)?;
            ensure!(
                Self::try_settle_expiry(rwa_asset_id, participation_id, &mut p, &asset)?,
                Error::<T>::InvalidParticipationStatus
            );
            Ok(())
        }

        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::claim_retired_deposit())]
        pub fn claim_retired_deposit(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(matches!(asset.status, AssetStatus::Retired), Error::<T>::InvalidAssetStatus);

            let mut p = Participations::<T>::get(rwa_asset_id, participation_id)
                .ok_or(Error::<T>::ParticipationNotFound)?;
            ensure!(
                matches!(
                    p.status,
                    ParticipationStatus::Active { .. } | ParticipationStatus::PendingApproval
                ),
                Error::<T>::InvalidParticipationStatus
            );

            let deposit = p.deposit_held;
            let fee_refund = if matches!(p.status, ParticipationStatus::PendingApproval) {
                p.entry_fee_paid
            } else {
                Zero::zero()
            };
            let total = deposit.saturating_add(fee_refund);
            Self::do_transfer(
                &asset.policy.deposit_currency,
                &Self::pallet_account(),
                &p.payer,
                total,
            )?;

            p.status = ParticipationStatus::Exited;
            p.deposit_held = Zero::zero();
            Participations::<T>::insert(rwa_asset_id, participation_id, &p);

            Self::remove_all_holder_indexes(rwa_asset_id, &p.holders);
            Self::dec_participant_count(rwa_asset_id);

            Self::deposit_event(Event::ParticipationExited {
                asset_id: rwa_asset_id,
                participation_id,
                deposit_refunded: total,
            });
            Ok(())
        }

        // ─── Group management ────────────────────────────────────────

        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::add_holder())]
        pub fn add_holder(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
            new_holder: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;

            Participations::<T>::try_mutate(
                rwa_asset_id,
                participation_id,
                |maybe| -> DispatchResult {
                    let p = maybe.as_mut().ok_or(Error::<T>::ParticipationNotFound)?;
                    ensure!(p.payer == who, Error::<T>::NotPayer);

                    if Self::try_settle_expiry_inner(rwa_asset_id, participation_id, p, &asset)? {
                        return Err(Error::<T>::ParticipationExpiredError.into());
                    }

                    ensure!(
                        matches!(p.status, ParticipationStatus::Active { .. }),
                        Error::<T>::InvalidParticipationStatus
                    );
                    ensure!(!p.holders.contains(&new_holder), Error::<T>::HolderAlreadyExists);
                    ensure!(
                        !HolderIndex::<T>::contains_key(rwa_asset_id, &new_holder),
                        Error::<T>::AlreadyParticipating
                    );
                    let h_assets = HolderAssets::<T>::get(&new_holder);
                    ensure!(
                        (h_assets.len() as u32) < T::MaxParticipationsPerHolder::get(),
                        Error::<T>::MaxParticipationsPerHolderReached
                    );

                    T::ParticipationFilter::ensure_eligible(rwa_asset_id, &new_holder)
                        .map_err(|_| Error::<T>::ParticipantNotEligible)?;

                    p.holders
                        .try_push(new_holder.clone())
                        .map_err(|_| Error::<T>::MaxGroupSizeReached)?;

                    HolderIndex::<T>::insert(rwa_asset_id, &new_holder, participation_id);
                    Self::push_holder_asset(&new_holder, rwa_asset_id);

                    Self::deposit_event(Event::HolderAdded {
                        asset_id: rwa_asset_id,
                        participation_id,
                        holder: new_holder,
                    });
                    Ok(())
                },
            )
        }

        #[pallet::call_index(15)]
        #[pallet::weight(T::WeightInfo::remove_holder())]
        pub fn remove_holder(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
            holder: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            let mut p = Participations::<T>::get(rwa_asset_id, participation_id)
                .ok_or(Error::<T>::ParticipationNotFound)?;
            ensure!(p.payer == who, Error::<T>::NotPayer);

            if Self::try_settle_expiry(rwa_asset_id, participation_id, &mut p, &asset)? {
                return Err(Error::<T>::ParticipationExpiredError.into());
            }
            ensure!(
                matches!(p.status, ParticipationStatus::Active { .. }),
                Error::<T>::InvalidParticipationStatus
            );

            let pos =
                p.holders.iter().position(|h| h == &holder).ok_or(Error::<T>::HolderNotFound)?;
            p.holders.remove(pos);

            Self::remove_single_holder_index(rwa_asset_id, &holder);

            if p.holders.is_empty() {
                // last holder removed → exit
                let deposit = p.deposit_held;
                Self::do_transfer(
                    &asset.policy.deposit_currency,
                    &Self::pallet_account(),
                    &p.payer,
                    deposit,
                )?;
                p.status = ParticipationStatus::Exited;
                p.deposit_held = Zero::zero();
                Self::dec_participant_count(rwa_asset_id);
                Participations::<T>::insert(rwa_asset_id, participation_id, &p);
                Self::deposit_event(Event::ParticipationExited {
                    asset_id: rwa_asset_id,
                    participation_id,
                    deposit_refunded: deposit,
                });
            } else {
                Participations::<T>::insert(rwa_asset_id, participation_id, &p);
                Self::deposit_event(Event::HolderRemoved {
                    asset_id: rwa_asset_id,
                    participation_id,
                    holder,
                });
            }
            Ok(())
        }

        #[pallet::call_index(16)]
        #[pallet::weight(T::WeightInfo::leave_participation())]
        pub fn leave_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            let mut p = Participations::<T>::get(rwa_asset_id, participation_id)
                .ok_or(Error::<T>::ParticipationNotFound)?;
            ensure!(p.holders.contains(&who), Error::<T>::NotHolder);

            if Self::try_settle_expiry(rwa_asset_id, participation_id, &mut p, &asset)? {
                return Err(Error::<T>::ParticipationExpiredError.into());
            }
            ensure!(
                matches!(p.status, ParticipationStatus::Active { .. }),
                Error::<T>::InvalidParticipationStatus
            );

            let pos = p.holders.iter().position(|h| h == &who).unwrap();
            p.holders.remove(pos);
            Self::remove_single_holder_index(rwa_asset_id, &who);

            if p.holders.is_empty() {
                let deposit = p.deposit_held;
                Self::do_transfer(
                    &asset.policy.deposit_currency,
                    &Self::pallet_account(),
                    &p.payer,
                    deposit,
                )?;
                p.status = ParticipationStatus::Exited;
                p.deposit_held = Zero::zero();
                Self::dec_participant_count(rwa_asset_id);
                Participations::<T>::insert(rwa_asset_id, participation_id, &p);
                Self::deposit_event(Event::ParticipationExited {
                    asset_id: rwa_asset_id,
                    participation_id,
                    deposit_refunded: deposit,
                });
            } else {
                Participations::<T>::insert(rwa_asset_id, participation_id, &p);
                Self::deposit_event(Event::HolderLeft {
                    asset_id: rwa_asset_id,
                    participation_id,
                    holder: who,
                });
            }
            Ok(())
        }

        // ─── Slash config ────────────────────────────────────────────

        #[pallet::call_index(17)]
        #[pallet::weight(T::WeightInfo::set_slash_distribution())]
        pub fn set_slash_distribution(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            distribution: BoundedVec<SlashRecipientOf<T>, T::MaxSlashRecipients>,
        ) -> DispatchResult {
            Self::ensure_asset_owner_or_admin(origin, rwa_asset_id)?;

            // HIGH-08 fix: compute the sum using raw u32 parts instead of
            // Permill::saturating_add.  Permill caps at `one()` (1_000_000),
            // so three shares of 400_000 each would saturate to 1_000_000
            // and pass the check — but distribute 1_200_000 parts worth,
            // short-changing the last recipient.
            //
            // By summing the raw parts, we detect any combination that
            // exceeds 100% or falls short.
            let total_parts: u32 = distribution
                .iter()
                .map(|r| r.share.deconstruct())
                .fold(0u32, |acc, p| acc.saturating_add(p));
            ensure!(total_parts == Permill::one().deconstruct(), Error::<T>::SlashSharesSumInvalid);

            let recipient_count = distribution.len() as u32;
            AssetSlashDistribution::<T>::insert(rwa_asset_id, distribution);
            Self::deposit_event(Event::SlashDistributionSet {
                asset_id: rwa_asset_id,
                recipient_count,
            });
            Ok(())
        }

        // ─── Enforcement ─────────────────────────────────────────────

        #[pallet::call_index(18)]
        #[pallet::weight(T::WeightInfo::slash_participation())]
        pub fn slash_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
            amount: BalanceOf<T>,
            reporter: Option<T::AccountId>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            let mut p = Participations::<T>::get(rwa_asset_id, participation_id)
                .ok_or(Error::<T>::ParticipationNotFound)?;

            if Self::try_settle_expiry(rwa_asset_id, participation_id, &mut p, &asset)? {
                return Err(Error::<T>::ParticipationExpiredError.into());
            }

            // CRIT-03: check cross-pallet guard before any state changes.
            T::AssetLifecycleGuard::can_slash_participation(rwa_asset_id, participation_id)
                .map_err(|_| Error::<T>::BlockedByLifecycleGuard)?;

            ensure!(
                matches!(p.status, ParticipationStatus::Active { .. }),
                Error::<T>::InvalidParticipationStatus
            );
            ensure!(amount <= p.deposit_held, Error::<T>::SlashAmountExceedsDeposit);

            // distribute slashed amount
            Self::do_distribute_slash(
                &asset.policy.deposit_currency,
                amount,
                rwa_asset_id,
                &asset.beneficiary,
                reporter.as_ref(),
            )?;

            // refund remainder
            let remainder = p.deposit_held.saturating_sub(amount);
            if !remainder.is_zero() {
                Self::do_transfer(
                    &asset.policy.deposit_currency,
                    &Self::pallet_account(),
                    &p.payer,
                    remainder,
                )?;
            }

            p.status = ParticipationStatus::Slashed;
            p.deposit_held = Zero::zero();
            Participations::<T>::insert(rwa_asset_id, participation_id, &p);
            Self::remove_all_holder_indexes(rwa_asset_id, &p.holders);
            Self::dec_participant_count(rwa_asset_id);

            Self::deposit_event(Event::ParticipationSlashed {
                asset_id: rwa_asset_id,
                participation_id,
                amount,
                reporter,
            });
            Ok(())
        }

        #[pallet::call_index(19)]
        #[pallet::weight(T::WeightInfo::revoke_participation())]
        pub fn revoke_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            let mut p = Participations::<T>::get(rwa_asset_id, participation_id)
                .ok_or(Error::<T>::ParticipationNotFound)?;

            if Self::try_settle_expiry(rwa_asset_id, participation_id, &mut p, &asset)? {
                return Err(Error::<T>::ParticipationExpiredError.into());
            }
            ensure!(
                matches!(p.status, ParticipationStatus::Active { .. }),
                Error::<T>::InvalidParticipationStatus
            );

            let deposit = p.deposit_held;
            Self::do_transfer(
                &asset.policy.deposit_currency,
                &Self::pallet_account(),
                &p.payer,
                deposit,
            )?;

            p.status = ParticipationStatus::Revoked;
            p.deposit_held = Zero::zero();
            Participations::<T>::insert(rwa_asset_id, participation_id, &p);
            Self::remove_all_holder_indexes(rwa_asset_id, &p.holders);
            Self::dec_participant_count(rwa_asset_id);

            Self::deposit_event(Event::ParticipationRevoked {
                asset_id: rwa_asset_id,
                participation_id,
                deposit_refunded: deposit,
            });
            Ok(())
        }

        // ─── Ownership transfer ───────────────────────────────────────

        #[pallet::call_index(20)]
        #[pallet::weight(T::WeightInfo::transfer_ownership())]
        pub fn transfer_ownership(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            new_owner: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(asset.owner == who, Error::<T>::NotAssetOwner);
            ensure!(!matches!(asset.status, AssetStatus::Retired), Error::<T>::AssetAlreadyRetired);
            ensure!(new_owner != who, Error::<T>::TransferToSelf);

            PendingOwnershipTransfer::<T>::insert(rwa_asset_id, &new_owner);

            Self::deposit_event(Event::OwnershipTransferProposed {
                asset_id: rwa_asset_id,
                from: who,
                to: new_owner,
            });
            Ok(())
        }

        #[pallet::call_index(21)]
        #[pallet::weight(T::WeightInfo::accept_ownership())]
        pub fn accept_ownership(origin: OriginFor<T>, rwa_asset_id: u32) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let pending = PendingOwnershipTransfer::<T>::get(rwa_asset_id)
                .ok_or(Error::<T>::NoPendingTransfer)?;
            ensure!(pending == who, Error::<T>::NotPendingOwner);

            let mut asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;

            // C-3: guard against accepting ownership of a Retired asset (possible when
            // retire_asset previously left PendingOwnershipTransfer behind — now fixed,
            // but this check is a belt-and-suspenders safety guard).
            //
            // HIGH-02: also block Paused assets. An admin-paused asset signals a
            // regulatory or compliance concern — allowing ownership transfer during
            // pause circumvents the pause intent. The admin should unpause first.
            ensure!(
                !matches!(asset.status, AssetStatus::Retired | AssetStatus::Paused),
                Error::<T>::InvalidAssetStatus
            );

            // Verify new_owner has capacity using a plain read — no storage write needed.
            let current_count = OwnerAssets::<T>::get(&who).len() as u32;
            ensure!(
                current_count < T::MaxAssetsPerOwner::get(),
                Error::<T>::MaxAssetsPerOwnerReached
            );

            // Transfer registration_deposit: unreserve from old owner, reserve on new owner
            let deposit = asset.registration_deposit;
            T::NativeCurrency::unreserve(&asset.owner, deposit);
            T::NativeCurrency::reserve(&who, deposit)?;

            let old_owner = asset.owner.clone();
            asset.owner = who.clone();
            RwaAssets::<T>::insert(rwa_asset_id, &asset);

            // Update OwnerAssets for both
            Self::remove_from_owner_assets(&old_owner, rwa_asset_id);
            OwnerAssets::<T>::mutate(&who, |assets| {
                let _ = assets.try_push(rwa_asset_id);
            });

            PendingOwnershipTransfer::<T>::remove(rwa_asset_id);

            Self::deposit_event(Event::OwnershipTransferred {
                asset_id: rwa_asset_id,
                old_owner,
                new_owner: who,
            });
            Ok(())
        }

        #[pallet::call_index(22)]
        #[pallet::weight(T::WeightInfo::cancel_ownership_transfer())]
        pub fn cancel_ownership_transfer(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(asset.owner == who, Error::<T>::NotAssetOwner);
            ensure!(
                PendingOwnershipTransfer::<T>::contains_key(rwa_asset_id),
                Error::<T>::NoPendingTransfer
            );

            PendingOwnershipTransfer::<T>::remove(rwa_asset_id);

            Self::deposit_event(Event::OwnershipTransferCancelled { asset_id: rwa_asset_id });
            Ok(())
        }

        // ─── Beneficiary & metadata updates ───────────────────────────

        #[pallet::call_index(23)]
        #[pallet::weight(T::WeightInfo::update_beneficiary())]
        pub fn update_beneficiary(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            new_beneficiary: T::AccountId,
        ) -> DispatchResult {
            Self::ensure_asset_owner_or_admin(origin, rwa_asset_id)?;
            RwaAssets::<T>::try_mutate(rwa_asset_id, |maybe| -> DispatchResult {
                let asset = maybe.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(
                    !matches!(asset.status, AssetStatus::Retired),
                    Error::<T>::AssetAlreadyRetired
                );
                let old_beneficiary = asset.beneficiary.clone();
                asset.beneficiary = new_beneficiary.clone();
                Self::deposit_event(Event::BeneficiaryUpdated {
                    asset_id: rwa_asset_id,
                    old_beneficiary,
                    new_beneficiary,
                });
                Ok(())
            })
        }

        #[pallet::call_index(24)]
        #[pallet::weight(T::WeightInfo::update_metadata())]
        pub fn update_metadata(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            new_metadata: Vec<u8>,
        ) -> DispatchResult {
            Self::ensure_asset_owner_or_admin(origin, rwa_asset_id)?;
            let bounded_meta: BoundedVec<u8, T::MaxMetadataLen> =
                new_metadata.try_into().map_err(|_| Error::<T>::MetadataTooLong)?;
            RwaAssets::<T>::try_mutate(rwa_asset_id, |maybe| -> DispatchResult {
                let asset = maybe.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(
                    !matches!(asset.status, AssetStatus::Retired),
                    Error::<T>::AssetAlreadyRetired
                );
                asset.metadata = bounded_meta;
                Self::deposit_event(Event::MetadataUpdated { asset_id: rwa_asset_id });
                Ok(())
            })
        }

        // ─── Participation transfer ───────────────────────────────────

        /// Transfer a participation position to a new payer.
        ///
        /// # Deposit semantics (intentional design)
        ///
        /// The participation deposit is held by the pallet account, not by
        /// either payer directly. When the current payer calls this
        /// extrinsic they are consenting to transfer the entire
        /// participation position — including the escrowed deposit — to
        /// `new_payer`. Any future refund (on exit, expiry, or revoke) will be
        /// sent to `new_payer`. The old payer effectively gifts their
        /// deposit position to the new payer. No on-chain deposit
        /// movement occurs at transfer time.
        #[pallet::call_index(25)]
        #[pallet::weight(T::WeightInfo::transfer_participation())]
        pub fn transfer_participation(
            origin: OriginFor<T>,
            rwa_asset_id: u32,
            participation_id: u32,
            new_payer: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
            ensure!(matches!(asset.status, AssetStatus::Active), Error::<T>::AssetNotActive);

            Participations::<T>::try_mutate(
                rwa_asset_id,
                participation_id,
                |maybe| -> DispatchResult {
                    let p = maybe.as_mut().ok_or(Error::<T>::ParticipationNotFound)?;
                    ensure!(p.payer == who, Error::<T>::NotPayer);
                    ensure!(new_payer != who, Error::<T>::TransferToSelf);

                    // Lazy expiry check
                    if Self::try_settle_expiry_inner(rwa_asset_id, participation_id, p, &asset)? {
                        return Err(Error::<T>::ParticipationExpiredError.into());
                    }

                    ensure!(
                        matches!(p.status, ParticipationStatus::Active { .. }),
                        Error::<T>::InvalidParticipationStatus
                    );

                    let old_payer = p.payer.clone();
                    p.payer = new_payer.clone();

                    Self::deposit_event(Event::ParticipationTransferred {
                        asset_id: rwa_asset_id,
                        participation_id,
                        old_payer,
                        new_payer,
                    });
                    Ok(())
                },
            )
        }

        // ─── Pause / unpause ──────────────────────────────────────────

        #[pallet::call_index(26)]
        #[pallet::weight(T::WeightInfo::pause_asset())]
        pub fn pause_asset(origin: OriginFor<T>, rwa_asset_id: u32) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            RwaAssets::<T>::try_mutate(rwa_asset_id, |maybe| -> DispatchResult {
                let asset = maybe.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(
                    matches!(asset.status, AssetStatus::Active | AssetStatus::Inactive),
                    Error::<T>::InvalidAssetStatus
                );
                asset.status = AssetStatus::Paused;
                Ok(())
            })?;
            Self::deposit_event(Event::AssetPaused { asset_id: rwa_asset_id });
            Ok(())
        }

        #[pallet::call_index(27)]
        #[pallet::weight(T::WeightInfo::unpause_asset())]
        pub fn unpause_asset(origin: OriginFor<T>, rwa_asset_id: u32) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            RwaAssets::<T>::try_mutate(rwa_asset_id, |maybe| -> DispatchResult {
                let asset = maybe.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(
                    matches!(asset.status, AssetStatus::Paused),
                    Error::<T>::InvalidAssetStatus
                );
                asset.status = AssetStatus::Active;
                Ok(())
            })?;
            Self::deposit_event(Event::AssetUnpaused { asset_id: rwa_asset_id });
            Ok(())
        }

        /// CAT-7.2-R-S: Batch-reject all pending approvals for an asset.
        ///
        /// This is an administrative safety valve: when a Sybil attack fills
        /// the `PendingApprovals` queue, the owner/admin can clear it
        /// in a single extrinsic instead of issuing O(n) individual
        /// `reject_participation` calls (which may exceed block weight
        /// limits).
        #[pallet::call_index(28)]
        #[pallet::weight(T::WeightInfo::batch_reject_pending())]
        pub fn batch_reject_pending(origin: OriginFor<T>, rwa_asset_id: u32) -> DispatchResult {
            Self::ensure_asset_owner_or_admin(origin, rwa_asset_id)?;

            let asset = RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;

            // HIGH-05 fix: do NOT use `take()` eagerly.  If `do_transfer` fails
            // mid-loop (e.g., pallet account drained), the remaining entries would
            // be lost because `take` already removed them from storage.
            //
            // Instead, read the list, process each entry, and only remove from
            // storage the entries that were successfully processed.  Entries that
            // fail the refund transfer are kept in both PendingApprovals and
            // Participations so they can be retried later.
            let pending = PendingApprovals::<T>::get(rwa_asset_id);
            let mut rejected: u32 = 0;
            let mut retained = sp_std::vec::Vec::new();

            for &pid in pending.iter() {
                if let Some(p) = Participations::<T>::get(rwa_asset_id, pid) {
                    if !matches!(p.status, ParticipationStatus::PendingApproval) {
                        // Not actually pending — keep it in the pending list
                        // defensively (will be cleaned up naturally).
                        retained.push(pid);
                        continue;
                    }
                    // refund deposit + entry_fee
                    let total = p.deposit_held.saturating_add(p.entry_fee_paid);
                    match Self::do_transfer(
                        &asset.policy.deposit_currency,
                        &Self::pallet_account(),
                        &p.payer,
                        total,
                    ) {
                        Ok(()) => {
                            // Transfer succeeded — clean up storage.
                            Participations::<T>::remove(rwa_asset_id, pid);
                            Self::remove_all_holder_indexes(rwa_asset_id, &p.holders);
                            Self::dec_participant_count(rwa_asset_id);
                            rejected += 1;
                        }
                        Err(e) => {
                            // Transfer failed — retain this entry so it can be
                            // retried. Log the failure for diagnostics.
                            log::warn!(
                                target: "pallet-rwa",
                                "batch_reject_pending: refund transfer failed for \
                                 asset={}, participation={}: {:?}",
                                rwa_asset_id, pid, e,
                            );
                            retained.push(pid);
                        }
                    }
                }
                // If participation doesn't exist, it was already cleaned up —
                // skip.
            }

            // Write back the retained entries (or clear if empty).
            if retained.is_empty() {
                PendingApprovals::<T>::remove(rwa_asset_id);
            } else {
                // Truncation is safe: retained.len() <= pending.len() <= MaxPendingApprovals.
                let bounded: BoundedVec<u32, T::MaxPendingApprovals> = retained
                    .try_into()
                    .expect("retained is a subset of PendingApprovals; cannot exceed bound");
                PendingApprovals::<T>::insert(rwa_asset_id, bounded);
            }

            Self::deposit_event(Event::BatchPendingRejected {
                asset_id: rwa_asset_id,
                count: rejected,
            });
            Ok(())
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        pub fn pallet_account() -> T::AccountId { T::PalletId::get().into_account_truncating() }

        fn do_transfer(
            currency: &PaymentCurrency<T::AssetId>,
            from: &T::AccountId,
            to: &T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            if amount.is_zero() {
                return Ok(());
            }
            match currency {
                PaymentCurrency::Native => {
                    T::NativeCurrency::transfer(from, to, amount, ExistenceRequirement::KeepAlive)?;
                }
                PaymentCurrency::Asset(asset_id) => {
                    <T::Fungibles as fungibles::Transfer<T::AccountId>>::transfer(
                        *asset_id, from, to, amount, true,
                    )?;
                }
            }
            Ok(())
        }

        fn do_burn(
            currency: &PaymentCurrency<T::AssetId>,
            who: &T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            if amount.is_zero() {
                return Ok(());
            }
            match currency {
                PaymentCurrency::Native => {
                    let imbalance = T::NativeCurrency::withdraw(
                        who,
                        amount,
                        WithdrawReasons::TRANSFER,
                        ExistenceRequirement::KeepAlive,
                    )?;
                    drop(imbalance);
                }
                PaymentCurrency::Asset(asset_id) => {
                    <T::Fungibles as fungibles::Mutate<T::AccountId>>::burn_from(
                        *asset_id, who, amount,
                    )?;
                }
            }
            Ok(())
        }

        /// Check lazy expiry and settle if expired. Returns true if settled.
        /// Writes to storage directly.
        fn try_settle_expiry(
            asset_id: u32,
            participation_id: u32,
            p: &mut ParticipationOf<T>,
            asset: &AssetInfoOf<T>,
        ) -> Result<bool, DispatchError> {
            let settled = Self::try_settle_expiry_inner(asset_id, participation_id, p, asset)?;
            if settled {
                Participations::<T>::insert(asset_id, participation_id, &*p);
            }
            Ok(settled)
        }

        /// Inner expiry check — mutates p in-place but does NOT write to
        /// Participations storage.
        fn try_settle_expiry_inner(
            asset_id: u32,
            participation_id: u32,
            p: &mut ParticipationOf<T>,
            asset: &AssetInfoOf<T>,
        ) -> Result<bool, DispatchError> {
            if let ParticipationStatus::Active { expires_at: Some(expiry), .. } = p.status {
                let now = frame_system::Pallet::<T>::block_number();
                if now >= expiry {
                    let deposit = p.deposit_held;
                    Self::do_transfer(
                        &asset.policy.deposit_currency,
                        &Self::pallet_account(),
                        &p.payer,
                        deposit,
                    )?;
                    p.status = ParticipationStatus::Expired;
                    p.deposit_held = Zero::zero();
                    Self::remove_all_holder_indexes(asset_id, &p.holders);
                    Self::dec_participant_count(asset_id);
                    Self::deposit_event(Event::ParticipationExpired {
                        asset_id,
                        participation_id,
                        deposit_refunded: deposit,
                    });
                    return Ok(true);
                }
            }
            Ok(false)
        }

        /// Distribute a slashed amount according to the asset's slash
        /// distribution config.
        ///
        /// # Correctness requirement
        ///
        /// This function performs multiple sequential transfers without an
        /// explicit `#[transactional]` attribute.  Its correctness
        /// depends on being called from within a transactional
        /// dispatchable (e.g., `slash_participation`) so that any
        /// mid-distribution transfer failure causes the entire extrinsic —
        /// including all prior transfers in this function — to be
        /// rolled back by the outer transaction. Do NOT call this
        /// function outside of a transactional context.
        ///
        /// # Safety (HIGH-04)
        ///
        /// In Substrate v0.9.40+, all dispatchable extrinsics are wrapped in
        /// `frame_support::storage::with_transaction` by default (see
        /// `frame_support::dispatch::Dispatchable` impl for `Call`).  This
        /// means any `Err` returned by `slash_participation` (or any
        /// other dispatchable caller) automatically rolls back ALL
        /// storage mutations made during that extrinsic, including
        /// partial transfers made here.
        ///
        /// Therefore, no explicit `#[transactional]` attribute is needed on
        /// this private helper — the outer dispatchable already
        /// provides the rollback guarantee.  This function must NOT be
        /// called from non-transactional contexts such as
        /// `on_initialize` or `offchain_worker`.
        fn do_distribute_slash(
            currency: &PaymentCurrency<T::AssetId>,
            total_amount: BalanceOf<T>,
            rwa_asset_id: u32,
            beneficiary: &T::AccountId,
            reporter: Option<&T::AccountId>,
        ) -> DispatchResult {
            let pallet_acct = Self::pallet_account();

            let distribution = AssetSlashDistribution::<T>::get(rwa_asset_id);
            match distribution {
                Some(dist) if !dist.is_empty() => {
                    let mut distributed = BalanceOf::<T>::zero();
                    let len = dist.len();
                    for (i, recipient) in dist.iter().enumerate() {
                        let is_last = i == len - 1;
                        let share_amount = if is_last {
                            total_amount.saturating_sub(distributed)
                        } else {
                            recipient.share * total_amount
                        };
                        if share_amount.is_zero() {
                            continue;
                        }
                        match &recipient.kind {
                            SlashRecipientKind::Beneficiary => {
                                Self::do_transfer(
                                    currency,
                                    &pallet_acct,
                                    beneficiary,
                                    share_amount,
                                )?;
                            }
                            SlashRecipientKind::Reporter => {
                                let dest = reporter.unwrap_or(beneficiary);
                                Self::do_transfer(currency, &pallet_acct, dest, share_amount)?;
                            }
                            SlashRecipientKind::Account(acct) => {
                                Self::do_transfer(currency, &pallet_acct, acct, share_amount)?;
                            }
                            SlashRecipientKind::Burn => {
                                Self::do_burn(currency, &pallet_acct, share_amount)?;
                            }
                        }
                        distributed = distributed.saturating_add(share_amount);
                    }
                }
                _ => {
                    // default: 100% to beneficiary
                    Self::do_transfer(currency, &pallet_acct, beneficiary, total_amount)?;
                }
            }
            Ok(())
        }

        fn ensure_asset_owner_or_admin(origin: OriginFor<T>, rwa_asset_id: u32) -> DispatchResult {
            match T::AdminOrigin::try_origin(origin) {
                Ok(_) => {
                    ensure!(RwaAssets::<T>::contains_key(rwa_asset_id), Error::<T>::AssetNotFound);
                    Ok(())
                }
                Err(origin) => {
                    let who = ensure_signed(origin)?;
                    let asset =
                        RwaAssets::<T>::get(rwa_asset_id).ok_or(Error::<T>::AssetNotFound)?;
                    ensure!(asset.owner == who, Error::<T>::NotAssetOwner);
                    Ok(())
                }
            }
        }

        fn remove_from_owner_assets(owner: &T::AccountId, asset_id: u32) {
            OwnerAssets::<T>::mutate(owner, |assets| {
                if let Some(pos) = assets.iter().position(|&id| id == asset_id) {
                    assets.remove(pos);
                }
            });
        }

        /// Push `asset_id` into the holder's `HolderAssets` list.
        ///
        /// All callers MUST pre-flight the `MaxParticipationsPerHolder`
        /// capacity check BEFORE calling this function. The `try_push`
        /// failure path here is strictly defensive — it should never be
        /// reached in practice.
        fn push_holder_asset(holder: &T::AccountId, asset_id: u32) {
            HolderAssets::<T>::mutate(holder, |assets| {
                if !assets.contains(&asset_id) {
                    if assets.try_push(asset_id).is_err() {
                        // HIGH-03: Log a defensive error instead of silently ignoring.
                        // If this fires, a caller is missing its pre-flight capacity check.
                        log::error!(
                            target: "pallet-rwa",
                            "push_holder_asset: BUG — capacity exceeded for asset_id={}, \
                             MaxParticipationsPerHolder pre-flight check was not performed by caller",
                            asset_id,
                        );
                        debug_assert!(
                            false,
                            "push_holder_asset: capacity should have been pre-checked"
                        );
                    }
                }
            });
        }

        fn remove_single_holder_index(asset_id: u32, holder: &T::AccountId) {
            HolderIndex::<T>::remove(asset_id, holder);
            HolderAssets::<T>::mutate(holder, |assets| {
                if let Some(pos) = assets.iter().position(|&id| id == asset_id) {
                    assets.remove(pos);
                }
            });
        }

        fn remove_all_holder_indexes(
            asset_id: u32,
            holders: &BoundedVec<T::AccountId, T::MaxGroupSize>,
        ) {
            for h in holders.iter() {
                Self::remove_single_holder_index(asset_id, h);
            }
        }

        fn dec_participant_count(asset_id: u32) {
            RwaAssets::<T>::mutate(asset_id, |maybe| {
                if let Some(a) = maybe {
                    a.participant_count = a.participant_count.saturating_sub(1);
                }
            });
        }

        fn inc_participant_count(asset_id: u32) {
            RwaAssets::<T>::mutate(asset_id, |maybe| {
                if let Some(a) = maybe {
                    a.participant_count = a.participant_count.saturating_add(1);
                }
            });
        }

        #[cfg(feature = "try-runtime")]
        fn do_try_state() -> Result<(), &'static str> {
            let next_id = NextRwaAssetId::<T>::get();

            // 1. All asset_ids < NextRwaAssetId
            for (asset_id, _) in RwaAssets::<T>::iter() {
                if asset_id >= next_id {
                    return Err("asset_id >= NextRwaAssetId");
                }
            }

            // 2. OwnerAssets ↔ RwaAssets bidirectional consistency
            for (owner, asset_ids) in OwnerAssets::<T>::iter() {
                for &aid in asset_ids.iter() {
                    let asset = RwaAssets::<T>::get(aid)
                        .ok_or("OwnerAssets references non-existent asset")?;
                    if asset.owner != owner {
                        return Err("OwnerAssets owner mismatch");
                    }
                    if matches!(asset.status, AssetStatus::Retired) {
                        return Err("Retired asset still in OwnerAssets");
                    }
                }
            }

            // 3. participant_count consistency
            for (asset_id, asset) in RwaAssets::<T>::iter() {
                let actual_count = Participations::<T>::iter_prefix(asset_id)
                    .filter(|(_, p)| {
                        matches!(
                            p.status,
                            ParticipationStatus::Active { .. }
                                | ParticipationStatus::PendingApproval
                        )
                    })
                    .count() as u32;
                if actual_count != asset.participant_count {
                    return Err("participant_count mismatch");
                }
            }

            // 4. HolderIndex validity
            for (asset_id, account, pid) in HolderIndex::<T>::iter() {
                let p = Participations::<T>::get(asset_id, pid)
                    .ok_or("HolderIndex references non-existent participation")?;
                if !matches!(
                    p.status,
                    ParticipationStatus::Active { .. } | ParticipationStatus::PendingApproval
                ) {
                    return Err("HolderIndex references non-active participation");
                }
                if !p.holders.contains(&account) {
                    return Err("HolderIndex account not in participation holders");
                }
            }

            // 5. HolderAssets validity
            for (account, asset_ids) in HolderAssets::<T>::iter() {
                for &aid in asset_ids.iter() {
                    if !HolderIndex::<T>::contains_key(aid, &account) {
                        return Err("HolderAssets references missing HolderIndex");
                    }
                }
            }

            // 6. PendingApprovals consistency
            for (asset_id, pending_ids) in PendingApprovals::<T>::iter() {
                for &pid in pending_ids.iter() {
                    let p = Participations::<T>::get(asset_id, pid)
                        .ok_or("PendingApprovals references non-existent participation")?;
                    if !matches!(p.status, ParticipationStatus::PendingApproval) {
                        return Err("PendingApprovals entry is not PendingApproval status");
                    }
                }
            }

            // 7. SlashDistribution shares sum
            for (_, dist) in AssetSlashDistribution::<T>::iter() {
                let total: Permill =
                    dist.iter().fold(Permill::zero(), |acc, r| acc.saturating_add(r.share));
                if total != Permill::one() {
                    return Err("SlashDistribution shares do not sum to 100%");
                }
            }

            // 8. SunsettingAssets consistency
            for (block, asset_ids) in SunsettingAssets::<T>::iter() {
                for &aid in asset_ids.iter() {
                    let asset = RwaAssets::<T>::get(aid)
                        .ok_or("SunsettingAssets references non-existent asset")?;
                    match asset.status {
                        AssetStatus::Sunsetting { expiry_block } => {
                            if expiry_block != block {
                                return Err("SunsettingAssets block mismatch");
                            }
                        }
                        _ => return Err("SunsettingAssets references non-sunsetting asset"),
                    }
                }
            }

            // 9. PendingOwnershipTransfer consistency
            for (asset_id, pending_owner) in PendingOwnershipTransfer::<T>::iter() {
                let asset = RwaAssets::<T>::get(asset_id)
                    .ok_or("PendingOwnershipTransfer references non-existent asset")?;
                if asset.owner == pending_owner {
                    return Err("PendingOwnershipTransfer owner matches current owner");
                }
                if matches!(asset.status, AssetStatus::Retired) {
                    return Err("PendingOwnershipTransfer on Retired asset");
                }
            }

            Ok(())
        }
    }
}
