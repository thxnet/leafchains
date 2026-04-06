#![cfg_attr(not(feature = "std"), no_std)]

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
mod benchmarks;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{
            tokens::{fungibles, nonfungibles_v2},
            Currency, ExistenceRequirement, WithdrawReasons,
        },
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{
        traits::{AccountIdConversion, Saturating, Zero},
        Permill,
    };

    use super::*;

    pub type BalanceOf<T> = <<T as Config>::NativeCurrency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    pub type CampaignOf<T> = Campaign<
        <T as frame_system::Config>::AccountId,
        BalanceOf<T>,
        <T as frame_system::Config>::BlockNumber,
        <T as Config>::AssetId,
        <T as Config>::CollectionId,
        <T as Config>::ItemId,
        <T as Config>::MaxMilestones,
        <T as Config>::MaxEligibilityRules,
        <T as Config>::MaxNftSets,
        <T as Config>::MaxNftsPerSet,
    >;

    pub type CampaignConfigOf<T> = CampaignConfig<
        BalanceOf<T>,
        <T as frame_system::Config>::BlockNumber,
        <T as Config>::AssetId,
        <T as Config>::MaxMilestones,
    >;

    pub type EligibilityRuleOf<T> = EligibilityRule<
        <T as Config>::AssetId,
        BalanceOf<T>,
        <T as Config>::CollectionId,
        <T as Config>::ItemId,
        <T as Config>::MaxNftSets,
        <T as Config>::MaxNftsPerSet,
    >;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type AssetId: Member + Parameter + Copy + MaxEncodedLen;
        type CollectionId: Member + Parameter + Copy + MaxEncodedLen;
        type ItemId: Member + Parameter + Copy + MaxEncodedLen;
        type NativeCurrency: Currency<Self::AccountId>;
        type Fungibles: fungibles::Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = BalanceOf<Self>>
            + fungibles::Mutate<Self::AccountId>
            + fungibles::Transfer<Self::AccountId>;
        type NftInspect: nonfungibles_v2::Inspect<
            Self::AccountId,
            CollectionId = Self::CollectionId,
            ItemId = Self::ItemId,
        >;
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        type ForceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        type MilestoneApprover: EnsureOrigin<Self::RuntimeOrigin>;
        #[pallet::constant]
        type PalletId: Get<PalletId>;
        #[pallet::constant]
        type CampaignCreationDeposit: Get<BalanceOf<Self>>;
        #[pallet::constant]
        type MaxCampaignsPerCreator: Get<u32>;
        #[pallet::constant]
        type MinCampaignDuration: Get<Self::BlockNumber>;
        #[pallet::constant]
        type MaxCampaignDuration: Get<Self::BlockNumber>;
        #[pallet::constant]
        type EarlyWithdrawalPenaltyBps: Get<u16>;
        #[pallet::constant]
        type MaxMilestones: Get<u32>;
        #[pallet::constant]
        type MaxEligibilityRules: Get<u32>;
        #[pallet::constant]
        type MaxNftSets: Get<u32>;
        #[pallet::constant]
        type MaxNftsPerSet: Get<u32>;
        #[pallet::constant]
        type MaxInvestmentsPerInvestor: Get<u32>;
        #[pallet::constant]
        type ProtocolFeeBps: Get<u16>;
        type ProtocolFeeRecipient: Get<Self::AccountId>;
        /// Maximum number of accounts that can be whitelisted per campaign.
        /// CAT-7.1-C-G fix: bounds the `CampaignWhitelist` storage to prevent
        /// unbounded growth from a malicious campaign creator.
        #[pallet::constant]
        type MaxWhitelistSize: Get<u32>;
        /// Optional license/RWA participation verifier.
        /// Set to `()` if no license requirement is needed.
        type LicenseVerifier: LicenseVerifier<Self::AccountId, Self::BlockNumber>;
        type WeightInfo: WeightInfo;
    }

    // ── Hooks ───────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        #[cfg(feature = "try-runtime")]
        fn try_state(_n: T::BlockNumber) -> Result<(), &'static str> { Self::do_try_state() }
    }

    // ── Storage ──────────────────────────────────────────────────────

    /// All campaign records, keyed by campaign ID.
    ///
    /// Campaign records are intentionally retained in storage after a campaign
    /// reaches a terminal state (Completed, Failed, Cancelled).  Deletion would
    /// break on-chain auditability: historical records must remain accessible
    /// to investors, indexers, and on-chain governance tooling.
    #[pallet::storage]
    pub type Campaigns<T: Config> = StorageMap<_, Blake2_128Concat, u32, CampaignOf<T>>;

    #[pallet::storage]
    pub type NextCampaignId<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    pub type Investments<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32,
        Blake2_128Concat,
        T::AccountId,
        Investment<BalanceOf<T>>,
    >;

    #[pallet::storage]
    pub type MilestoneStatuses<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, u8, MilestoneStatus>;

    #[pallet::storage]
    pub type DefaultEligibilityRules<T: Config> =
        StorageValue<_, BoundedVec<EligibilityRuleOf<T>, T::MaxEligibilityRules>, ValueQuery>;

    #[pallet::storage]
    pub type CreatorCampaigns<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<u32, T::MaxCampaignsPerCreator>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type InvestorCampaigns<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<u32, T::MaxInvestmentsPerInvestor>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type CampaignWhitelist<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    /// CAT-7.1-C-G: counter for whitelist entries per campaign, bounded by
    /// `MaxWhitelistSize`. Stored separately so we don't need to iterate
    /// the double-map to check the count.
    #[pallet::storage]
    pub type CampaignWhitelistCount<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, u32, ValueQuery>;

    /// Override for the compile-time `ProtocolFeeBps` constant, set via
    /// `set_protocol_config`.  When `None`, the Config constant is used.
    #[pallet::storage]
    pub type ProtocolFeeBpsOverride<T: Config> = StorageValue<_, u16>;

    /// Override for the compile-time `ProtocolFeeRecipient` constant, set via
    /// `set_protocol_config`.  When `None`, the Config constant is used.
    #[pallet::storage]
    pub type ProtocolFeeRecipientOverride<T: Config> = StorageValue<_, T::AccountId>;

    // ── Events ───────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        #[codec(index = 0)]
        CampaignCreated { campaign_id: u32, creator: T::AccountId, deadline: T::BlockNumber },
        #[codec(index = 1)]
        CampaignFinalized { campaign_id: u32, status: CampaignStatus },
        #[codec(index = 2)]
        CampaignCancelled { campaign_id: u32 },
        #[codec(index = 3)]
        CreationDepositClaimed {
            campaign_id: u32,
            creator: T::AccountId,
            deposit_returned: BalanceOf<T>,
        },
        #[codec(index = 4)]
        Invested { campaign_id: u32, investor: T::AccountId, amount: BalanceOf<T> },
        #[codec(index = 5)]
        InvestmentWithdrawn {
            campaign_id: u32,
            investor: T::AccountId,
            amount: BalanceOf<T>,
            penalty: BalanceOf<T>,
        },
        #[codec(index = 6)]
        RefundClaimed { campaign_id: u32, investor: T::AccountId, amount: BalanceOf<T> },
        #[codec(index = 7)]
        FundsClaimed { campaign_id: u32, creator: T::AccountId, amount: BalanceOf<T> },
        #[codec(index = 8)]
        MilestoneSubmitted { campaign_id: u32, index: u8 },
        #[codec(index = 9)]
        MilestoneApproved { campaign_id: u32, index: u8 },
        #[codec(index = 10)]
        MilestoneRejected { campaign_id: u32, index: u8 },
        #[codec(index = 11)]
        MilestoneFundsClaimed { campaign_id: u32, index: u8, amount: BalanceOf<T> },
        #[codec(index = 12)]
        DefaultEligibilitySet,
        #[codec(index = 13)]
        ProtocolFeeCollected { campaign_id: u32, amount: BalanceOf<T> },
        #[codec(index = 14)]
        CampaignPaused { campaign_id: u32 },
        #[codec(index = 15)]
        CampaignResumed { campaign_id: u32 },
        #[codec(index = 16)]
        HardCapReached { campaign_id: u32 },
        #[codec(index = 17)]
        ProtocolConfigUpdated { fee_bps: u16, recipient: T::AccountId },
        /// A campaign was cancelled because its linked RWA license was revoked.
        #[codec(index = 18)]
        CampaignLicenseReported { campaign_id: u32 },
        /// A campaign was force-finalized by sudo, bypassing the deadline
        /// check.
        #[codec(index = 19)]
        CampaignForceFinalized { campaign_id: u32, status: CampaignStatus },
    }

    // ── Errors ───────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        #[codec(index = 0)]
        CampaignNotFound,
        #[codec(index = 1)]
        NotCampaignCreator,
        #[codec(index = 2)]
        InvalidCampaignStatus,
        #[codec(index = 3)]
        CampaignStillFunding,
        #[codec(index = 4)]
        DeadlinePassed,
        #[codec(index = 5)]
        DeadlineInPast,
        #[codec(index = 6)]
        DurationTooShort,
        #[codec(index = 7)]
        DurationTooLong,
        #[codec(index = 8)]
        GoalNotMet,
        #[codec(index = 9)]
        HardCapExceeded,
        #[codec(index = 10)]
        InvestmentBelowMinimum,
        #[codec(index = 11)]
        InvestmentExceedsMaxPerInvestor,
        #[codec(index = 12)]
        InsufficientInvestment,
        #[codec(index = 13)]
        NoInvestmentFound,
        #[codec(index = 14)]
        NothingToRefund,
        #[codec(index = 15)]
        AlreadyClaimed,
        #[codec(index = 16)]
        EligibilityCheckFailed,
        #[codec(index = 17)]
        InvalidMilestoneIndex,
        #[codec(index = 18)]
        InvalidMilestoneStatus,
        #[codec(index = 19)]
        MilestoneBpsSumInvalid,
        #[codec(index = 20)]
        MaxCampaignsPerCreatorReached,
        #[codec(index = 21)]
        MaxInvestmentsPerInvestorReached,
        #[codec(index = 22)]
        InsufficientBalance,
        #[codec(index = 23)]
        InvalidFundingModel,
        #[codec(index = 24)]
        NothingToClaim,
        #[codec(index = 25)]
        CampaignNotFunding,
        #[codec(index = 26)]
        CampaignNotPaused,
        #[codec(index = 27)]
        NotCampaignCreatorOrAdmin,
        #[codec(index = 28)]
        CampaignIdOverflow,
        /// early_withdrawal_penalty_bps exceeds 10 000 (100 %).
        #[codec(index = 29)]
        InvalidPenaltyBps,
        /// protocol_fee_bps exceeds 10 000 (100 %).
        #[codec(index = 30)]
        InvalidFeeBps,
        /// License verification failed: caller is not an active participant.
        #[codec(index = 31)]
        LicenseNotActive,
        /// The campaign's linked license has been revoked/slashed.
        #[codec(index = 32)]
        CampaignLicenseRevoked,
        /// The campaign has no linked license (cannot report revocation).
        #[codec(index = 33)]
        NoLinkedLicense,
        /// The campaign whitelist is full (`MaxWhitelistSize` reached).
        #[codec(index = 34)]
        WhitelistFull,
        /// Campaign deadline exceeds the license expiry block (V2 fix).
        #[codec(index = 35)]
        CampaignExceedsLicenseExpiry,
    }

    // ── Dispatchables ────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_campaign())]
        pub fn create_campaign(
            origin: OriginFor<T>,
            config: CampaignConfigOf<T>,
            custom_rules: Option<BoundedVec<EligibilityRuleOf<T>, T::MaxEligibilityRules>>,
            license: Option<(u32, u32)>,
        ) -> DispatchResult {
            let creator = ensure_signed(origin)?;

            // License verification: if provided, verify the creator holds an
            // active participation for the given RWA asset.
            let (rwa_asset_id, participation_id) = if let Some((asset_id, part_id)) = license {
                T::LicenseVerifier::ensure_active_license(asset_id, part_id, &creator)
                    .map_err(|_| Error::<T>::LicenseNotActive)?;
                // V2 fix: ensure the campaign deadline does not exceed the license expiry.
                // If the license has no expiry (unlimited duration), this check is skipped.
                if let Some(expiry) = T::LicenseVerifier::license_expiry(asset_id, part_id) {
                    // CRIT-07: strict less-than ensures the license is still active at
                    // the deadline block.  `is_license_active` uses `now < expiry`,
                    // so deadline == expiry would allow a front-runner to call
                    // `report_license_revoked` before `finalize_campaign`.
                    ensure!(config.deadline < expiry, Error::<T>::CampaignExceedsLicenseExpiry);
                }
                (Some(asset_id), Some(part_id))
            } else {
                (None, None)
            };

            let now = frame_system::Pallet::<T>::block_number();

            ensure!(config.deadline > now, Error::<T>::DeadlineInPast);
            let duration = config.deadline.saturating_sub(now);
            ensure!(duration >= T::MinCampaignDuration::get(), Error::<T>::DurationTooShort);
            ensure!(duration <= T::MaxCampaignDuration::get(), Error::<T>::DurationTooLong);

            // P0-01: validate early_withdrawal_penalty_bps <= 10_000 (100%)
            if let Some(penalty_bps) = config.early_withdrawal_penalty_bps {
                ensure!(penalty_bps <= 10_000, Error::<T>::InvalidPenaltyBps);
            }

            // M-4: validate funding model — goal must be non-zero; milestone bps must sum
            // to 10_000
            match &config.funding_model {
                FundingModel::AllOrNothing { goal } => {
                    ensure!(!goal.is_zero(), Error::<T>::InvalidFundingModel);
                }
                FundingModel::MilestoneBased { goal, milestones } => {
                    ensure!(!goal.is_zero(), Error::<T>::InvalidFundingModel);
                    let total_bps: u32 = milestones.iter().map(|m| m.release_bps as u32).sum();
                    ensure!(total_bps == 10_000, Error::<T>::MilestoneBpsSumInvalid);
                }
                FundingModel::KeepWhatYouRaise { .. } => {}
            }

            // M-5: hard_cap must be >= goal when both are set
            if let Some(cap) = &config.hard_cap {
                match &config.funding_model {
                    FundingModel::AllOrNothing { goal }
                    | FundingModel::MilestoneBased { goal, .. } => {
                        ensure!(*cap >= *goal, Error::<T>::InvalidFundingModel);
                    }
                    _ => {}
                }
            }

            // M-6: min_investment must be <= max_investment_per_investor when both are set
            if let (Some(min), Some(max)) =
                (&config.min_investment, &config.max_investment_per_investor)
            {
                ensure!(*min <= *max, Error::<T>::InvalidFundingModel);
            }

            // creator campaign limit
            let current_campaigns = CreatorCampaigns::<T>::get(&creator);
            ensure!(
                (current_campaigns.len() as u32) < T::MaxCampaignsPerCreator::get(),
                Error::<T>::MaxCampaignsPerCreatorReached
            );

            // Charge the full creation deposit (native). The deposit is transferred
            // to the per-campaign sub-account where it sits alongside any raised
            // funds.  This covers the sub-account existential deposit as well:
            // CampaignCreationDeposit must be configured to be >= ExistentialDeposit.
            // The deposit is returned via claim_creation_deposit once the campaign
            // reaches a terminal state (Completed / Failed / Cancelled).
            let deposit = T::CampaignCreationDeposit::get();
            ensure!(
                deposit >= T::NativeCurrency::minimum_balance(),
                Error::<T>::InsufficientBalance
            );

            let campaign_id = NextCampaignId::<T>::get();
            let next_id = campaign_id.checked_add(1).ok_or(Error::<T>::CampaignIdOverflow)?;
            NextCampaignId::<T>::put(next_id);

            let sub_account = Self::campaign_account(campaign_id);

            // Transfer the full deposit (not just ED) to the sub-account.
            T::NativeCurrency::transfer(
                &creator,
                &sub_account,
                deposit,
                ExistenceRequirement::KeepAlive,
            )?;

            let rules = custom_rules.unwrap_or_else(|| DefaultEligibilityRules::<T>::get());

            let campaign = Campaign {
                creator: creator.clone(),
                status: CampaignStatus::Funding,
                config,
                eligibility_rules: rules,
                total_raised: Zero::zero(),
                total_disbursed: Zero::zero(),
                investor_count: 0,
                creation_deposit: deposit,
                created_at: now,
                paused_at: None,
                rwa_asset_id,
                participation_id,
                // CAT-3.9-C-I: lock protocol fee at creation time
                protocol_fee_bps: Self::effective_protocol_fee_bps(),
            };

            Campaigns::<T>::insert(campaign_id, &campaign);
            CreatorCampaigns::<T>::try_mutate(&creator, |ids| -> DispatchResult {
                ids.try_push(campaign_id).map_err(|_| Error::<T>::MaxCampaignsPerCreatorReached)?;
                Ok(())
            })?;

            Self::deposit_event(Event::CampaignCreated {
                campaign_id,
                creator,
                deadline: campaign.config.deadline,
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::cancel_campaign())]
        pub fn cancel_campaign(origin: OriginFor<T>, campaign_id: u32) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;
            Campaigns::<T>::try_mutate(campaign_id, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                // CRIT-02: also block Succeeded — once a campaign has finalized to
                // Succeeded the creator has a committed economic relationship with
                // investors and must be guaranteed a window to claim funds.
                // MilestonePhase remains cancellable as an emergency valve if the
                // creator disappears.
                ensure!(
                    !matches!(
                        c.status,
                        CampaignStatus::Cancelled
                            | CampaignStatus::Completed
                            | CampaignStatus::Succeeded
                    ),
                    Error::<T>::InvalidCampaignStatus
                );
                // P2-08: clean up milestone statuses (bounded by MaxMilestones ≤ 5)
                if let FundingModel::MilestoneBased { milestones, .. } = &c.config.funding_model {
                    for i in 0..milestones.len() as u8 {
                        MilestoneStatuses::<T>::remove(campaign_id, i);
                    }
                }
                c.status = CampaignStatus::Cancelled;
                Ok(())
            })?;
            // H-2: clean up whitelist entries — they have no use once a campaign is
            // cancelled, and leaving them wastes storage indefinitely.
            let _ = CampaignWhitelist::<T>::clear_prefix(campaign_id, u32::MAX, None);
            CampaignWhitelistCount::<T>::remove(campaign_id);
            Self::deposit_event(Event::CampaignCancelled { campaign_id });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_default_eligibility())]
        pub fn set_default_eligibility(
            origin: OriginFor<T>,
            rules: BoundedVec<EligibilityRuleOf<T>, T::MaxEligibilityRules>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            DefaultEligibilityRules::<T>::put(rules);
            Self::deposit_event(Event::DefaultEligibilitySet);
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::invest())]
        pub fn invest(
            origin: OriginFor<T>,
            campaign_id: u32,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let investor = ensure_signed(origin)?;
            let campaign = Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(
                matches!(campaign.status, CampaignStatus::Funding),
                Error::<T>::InvalidCampaignStatus
            );
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(now <= campaign.config.deadline, Error::<T>::DeadlinePassed);

            // P2-01: reject zero-amount investments before any other checks
            ensure!(!amount.is_zero(), Error::<T>::InvestmentBelowMinimum);

            // eligibility
            Self::check_eligibility_inner(&investor, &campaign, campaign_id)?;

            // min investment
            if let Some(min) = campaign.config.min_investment {
                ensure!(amount >= min, Error::<T>::InvestmentBelowMinimum);
            }

            // hard cap
            if let Some(cap) = campaign.config.hard_cap {
                ensure!(
                    campaign.total_raised.saturating_add(amount) <= cap,
                    Error::<T>::HardCapExceeded
                );
            }

            // per-investor max
            let mut inv = Investments::<T>::get(campaign_id, &investor).unwrap_or_default();
            let current = inv.total_invested.saturating_sub(inv.total_withdrawn);
            if let Some(max_per) = campaign.config.max_investment_per_investor {
                ensure!(
                    current.saturating_add(amount) <= max_per,
                    Error::<T>::InvestmentExceedsMaxPerInvestor
                );
            }

            // H-1: use InvestorCampaigns membership as the authoritative "new investor"
            // signal. The Investment record persists after a full withdrawal
            // (total_invested and total_withdrawn are both non-zero), so
            // checking those fields produces a false negative on re-invest.
            // InvestorCampaigns is removed on full withdrawal (and on refund),
            // so its absence correctly identifies a new participant.
            let is_new = !InvestorCampaigns::<T>::get(&investor).contains(&campaign_id);

            if is_new {
                let current = InvestorCampaigns::<T>::get(&investor);
                ensure!(
                    (current.len() as u32) < T::MaxInvestmentsPerInvestor::get(),
                    Error::<T>::MaxInvestmentsPerInvestorReached
                );
            }

            let sub_account = Self::campaign_account(campaign_id);
            Self::do_transfer(
                &campaign.config.funding_currency,
                &investor,
                &sub_account,
                amount,
                ExistenceRequirement::KeepAlive,
            )?;

            inv.total_invested = inv.total_invested.saturating_add(amount);
            Investments::<T>::insert(campaign_id, &investor, inv);

            let mut hard_cap_reached = false;
            Campaigns::<T>::mutate(campaign_id, |maybe| {
                if let Some(c) = maybe {
                    c.total_raised = c.total_raised.saturating_add(amount);
                    if is_new {
                        c.investor_count = c.investor_count.saturating_add(1);
                    }
                    // P3-01: check hard cap using the updated total_raised
                    if let Some(cap) = c.config.hard_cap {
                        if c.total_raised == cap {
                            hard_cap_reached = true;
                        }
                    }
                }
            });

            if is_new {
                InvestorCampaigns::<T>::try_mutate(&investor, |ids| -> DispatchResult {
                    ids.try_push(campaign_id)
                        .map_err(|_| Error::<T>::MaxInvestmentsPerInvestorReached)?;
                    Ok(())
                })?;
            }

            if hard_cap_reached {
                Self::deposit_event(Event::HardCapReached { campaign_id });
            }

            Self::deposit_event(Event::Invested { campaign_id, investor, amount });
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::withdraw_investment())]
        pub fn withdraw_investment(
            origin: OriginFor<T>,
            campaign_id: u32,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let investor = ensure_signed(origin)?;
            let campaign = Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(
                matches!(campaign.status, CampaignStatus::Funding | CampaignStatus::Paused),
                Error::<T>::InvalidCampaignStatus
            );

            // P2-02: reject zero-amount withdrawals
            ensure!(!amount.is_zero(), Error::<T>::InsufficientInvestment);

            let mut inv = Investments::<T>::get(campaign_id, &investor)
                .ok_or(Error::<T>::NoInvestmentFound)?;
            let current = inv.total_invested.saturating_sub(inv.total_withdrawn);
            ensure!(amount <= current, Error::<T>::InsufficientInvestment);

            let penalty_bps = campaign
                .config
                .early_withdrawal_penalty_bps
                .unwrap_or_else(|| T::EarlyWithdrawalPenaltyBps::get());
            // P2-10: compute_penalty merged into bps_of (with clamping)
            let penalty = Self::bps_of(amount, penalty_bps);
            let net = amount.saturating_sub(penalty);

            let sub_account = Self::campaign_account(campaign_id);
            // transfer net to investor
            Self::do_transfer(
                &campaign.config.funding_currency,
                &sub_account,
                &investor,
                net,
                ExistenceRequirement::AllowDeath,
            )?;
            // burn penalty
            if !penalty.is_zero() {
                Self::do_burn(
                    &campaign.config.funding_currency,
                    &sub_account,
                    penalty,
                    ExistenceRequirement::AllowDeath,
                )?;
            }

            inv.total_withdrawn = inv.total_withdrawn.saturating_add(amount);
            let fully_withdrawn = inv.total_invested == inv.total_withdrawn;
            Investments::<T>::insert(campaign_id, &investor, inv);

            if fully_withdrawn {
                InvestorCampaigns::<T>::mutate(&investor, |ids| {
                    if let Some(pos) = ids.iter().position(|&id| id == campaign_id) {
                        ids.remove(pos);
                    }
                });
            }

            Campaigns::<T>::mutate(campaign_id, |maybe| {
                if let Some(c) = maybe {
                    c.total_raised = c.total_raised.saturating_sub(amount);
                    // P2-12: decrement investor_count when fully withdrawn
                    if fully_withdrawn {
                        c.investor_count = c.investor_count.saturating_sub(1);
                    }
                }
            });

            Self::deposit_event(Event::InvestmentWithdrawn {
                campaign_id,
                investor,
                amount: net,
                penalty,
            });
            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::claim_refund())]
        pub fn claim_refund(origin: OriginFor<T>, campaign_id: u32) -> DispatchResult {
            let investor = ensure_signed(origin)?;
            let campaign = Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(
                matches!(campaign.status, CampaignStatus::Failed | CampaignStatus::Cancelled),
                Error::<T>::InvalidCampaignStatus
            );

            let inv = Investments::<T>::get(campaign_id, &investor)
                .ok_or(Error::<T>::NoInvestmentFound)?;
            let raw_refund = inv.total_invested.saturating_sub(inv.total_withdrawn);
            ensure!(!raw_refund.is_zero(), Error::<T>::NothingToRefund);

            // When milestones have been partially disbursed before cancellation,
            // reduce the refund proportionally so it doesn't exceed the sub-account
            // balance.
            let refund = if !campaign.total_disbursed.is_zero() && !campaign.total_raised.is_zero()
            {
                let remaining_ratio = Permill::from_rational(
                    campaign.total_raised.saturating_sub(campaign.total_disbursed),
                    campaign.total_raised,
                );
                remaining_ratio * raw_refund
            } else {
                raw_refund
            };
            ensure!(!refund.is_zero(), Error::<T>::NothingToRefund);

            let sub_account = Self::campaign_account(campaign_id);
            Self::do_transfer(
                &campaign.config.funding_currency,
                &sub_account,
                &investor,
                refund,
                ExistenceRequirement::AllowDeath,
            )?;

            Investments::<T>::remove(campaign_id, &investor);

            InvestorCampaigns::<T>::mutate(&investor, |ids| {
                if let Some(pos) = ids.iter().position(|&id| id == campaign_id) {
                    ids.remove(pos);
                }
            });

            // P2-12: decrement investor_count on refund
            Campaigns::<T>::mutate(campaign_id, |maybe| {
                if let Some(c) = maybe {
                    c.investor_count = c.investor_count.saturating_sub(1);
                }
            });

            Self::deposit_event(Event::RefundClaimed { campaign_id, investor, amount: refund });
            Ok(())
        }

        /// Transition a Funding campaign to its terminal status after the
        /// deadline passes.
        ///
        /// This extrinsic is **permissionless by design**: any signed account
        /// can trigger finalization once the campaign deadline has
        /// elapsed.  This avoids a liveness dependency on a single
        /// privileged actor (e.g., the creator going offline) while
        /// imposing no economic risk — the outcome is fully determined
        /// by on-chain state (total_raised vs. goal).
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::finalize_campaign())]
        pub fn finalize_campaign(origin: OriginFor<T>, campaign_id: u32) -> DispatchResult {
            ensure_signed(origin)?;
            Campaigns::<T>::try_mutate(campaign_id, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                ensure!(
                    matches!(c.status, CampaignStatus::Funding),
                    Error::<T>::InvalidCampaignStatus
                );
                let now = frame_system::Pallet::<T>::block_number();
                ensure!(now > c.config.deadline, Error::<T>::CampaignStillFunding);

                let new_status = Self::do_finalize(campaign_id, c);
                c.status = new_status;
                Self::deposit_event(Event::CampaignFinalized { campaign_id, status: new_status });
                Ok(())
            })
        }

        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::claim_funds())]
        pub fn claim_funds(origin: OriginFor<T>, campaign_id: u32) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut campaign =
                Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(campaign.creator == who, Error::<T>::NotCampaignCreator);
            ensure!(
                matches!(campaign.status, CampaignStatus::Succeeded),
                Error::<T>::InvalidCampaignStatus
            );

            // DEADLOCK-FIX: License check removed from claim_funds.
            //
            // Previous V3 fix checked ensure_active_license here, but combined
            // with CRIT-01/CRIT-02 fixes (Succeeded campaigns cannot be
            // cancelled), this created a deadlock: if the license is revoked
            // after finalization, the creator cannot claim (license inactive)
            // and nobody can cancel (Succeeded is blocked).  Funds would be
            // permanently locked.
            //
            // Rationale for removal:
            // 1. The license was validated at campaign creation time.
            // 2. The campaign already met its goal and was finalized to
            //    Succeeded — the economic outcome is decided.
            // 3. `report_license_revoked` protects investors during Funding /
            //    Paused / MilestonePhase.  Once Succeeded, the creator has
            //    earned the right to claim.
            // 4. The V3 concern ("transferred participation blocks original
            //    creator") conflated RWA payer with campaign creator — these
            //    are independent concepts.

            let claimable = campaign.total_raised.saturating_sub(campaign.total_disbursed);
            ensure!(!claimable.is_zero(), Error::<T>::NothingToClaim);

            // CAT-3.9-C-I: use the fee locked at creation time, not the current global rate
            let fee_bps = campaign.protocol_fee_bps;
            let fee = Self::bps_of(claimable, fee_bps);
            let creator_amount = claimable.saturating_sub(fee);

            let sub_account = Self::campaign_account(campaign_id);
            if !fee.is_zero() {
                let fee_recipient = Self::effective_protocol_fee_recipient();
                Self::do_transfer(
                    &campaign.config.funding_currency,
                    &sub_account,
                    &fee_recipient,
                    fee,
                    ExistenceRequirement::AllowDeath,
                )?;
                Self::deposit_event(Event::ProtocolFeeCollected { campaign_id, amount: fee });
            }
            Self::do_transfer(
                &campaign.config.funding_currency,
                &sub_account,
                &who,
                creator_amount,
                ExistenceRequirement::AllowDeath,
            )?;

            campaign.total_disbursed = campaign.total_raised;
            campaign.status = CampaignStatus::Completed;
            Campaigns::<T>::insert(campaign_id, &campaign);

            // P1-02: emit net amount (after fee), not gross
            Self::deposit_event(Event::FundsClaimed {
                campaign_id,
                creator: who.clone(),
                amount: creator_amount,
            });
            // P2-09: CampaignCompleted renamed to CreationDepositClaimed; the
            // "completed" signal here is implicit from the status transition.
            // We keep a separate CreationDepositClaimed event for the deposit
            // claim step. No additional event needed here beyond FundsClaimed.
            Ok(())
        }

        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::claim_creation_deposit())]
        pub fn claim_creation_deposit(origin: OriginFor<T>, campaign_id: u32) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut campaign =
                Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(campaign.creator == who, Error::<T>::NotCampaignCreator);
            ensure!(
                matches!(
                    campaign.status,
                    CampaignStatus::Completed | CampaignStatus::Failed | CampaignStatus::Cancelled
                ),
                Error::<T>::InvalidCampaignStatus
            );

            // Guard against double-claim: the creation_deposit field is zeroed after
            // the first successful claim.  A second call will see zero and error out.
            ensure!(!campaign.creation_deposit.is_zero(), Error::<T>::AlreadyClaimed);

            // Transfer exactly the creation deposit back to the creator — not the
            // full sub-account balance — so that investor funds waiting to be
            // refunded on Failed/Cancelled campaigns cannot be drained.
            let deposit = campaign.creation_deposit;
            let sub_account = Self::campaign_account(campaign_id);

            // CRIT-06: For campaigns funded with a fungible asset (not native),
            // the sub-account also holds asset token balances for investor
            // refunds.  If the native balance drops to zero the sub-account is
            // reaped, destroying those asset token balances.  Use KeepAlive
            // when there are still investors who need to claim refunds.
            // For native-funded campaigns, or asset-funded campaigns where all
            // investors have already refunded (investor_count == 0), AllowDeath
            // is safe.
            let existence_req = match &campaign.config.funding_currency {
                PaymentCurrency::Asset(_) if campaign.investor_count > 0 => {
                    ExistenceRequirement::KeepAlive
                }
                _ => ExistenceRequirement::AllowDeath,
            };
            T::NativeCurrency::transfer(&sub_account, &who, deposit, existence_req)?;

            // Zero out the recorded deposit to prevent future double-claims.
            campaign.creation_deposit = Zero::zero();
            Campaigns::<T>::insert(campaign_id, &campaign);

            CreatorCampaigns::<T>::mutate(&who, |ids| {
                if let Some(pos) = ids.iter().position(|&id| id == campaign_id) {
                    ids.remove(pos);
                }
            });

            // H-2: clean up any remaining whitelist entries now that the campaign
            // has fully concluded and the deposit is being reclaimed.
            let _ = CampaignWhitelist::<T>::clear_prefix(campaign_id, u32::MAX, None);
            CampaignWhitelistCount::<T>::remove(campaign_id);

            // P2-09: renamed from CampaignCompleted to CreationDepositClaimed
            Self::deposit_event(Event::CreationDepositClaimed {
                campaign_id,
                creator: who,
                deposit_returned: deposit,
            });
            Ok(())
        }

        // ─── Milestone ───────────────────────────────────────────────

        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::submit_milestone())]
        pub fn submit_milestone(
            origin: OriginFor<T>,
            campaign_id: u32,
            index: u8,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let campaign = Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(campaign.creator == who, Error::<T>::NotCampaignCreator);
            ensure!(
                matches!(campaign.status, CampaignStatus::MilestonePhase),
                Error::<T>::InvalidCampaignStatus
            );

            MilestoneStatuses::<T>::try_mutate(campaign_id, index, |maybe| -> DispatchResult {
                let status = maybe.as_mut().ok_or(Error::<T>::InvalidMilestoneIndex)?;
                ensure!(
                    matches!(status, MilestoneStatus::Pending | MilestoneStatus::Rejected),
                    Error::<T>::InvalidMilestoneStatus
                );
                *status = MilestoneStatus::Submitted;
                Ok(())
            })?;

            Self::deposit_event(Event::MilestoneSubmitted { campaign_id, index });
            Ok(())
        }

        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::approve_milestone())]
        pub fn approve_milestone(
            origin: OriginFor<T>,
            campaign_id: u32,
            index: u8,
        ) -> DispatchResult {
            T::MilestoneApprover::ensure_origin(origin)?;
            // P1-01: check campaign exists AND is in MilestonePhase
            let campaign = Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(
                matches!(campaign.status, CampaignStatus::MilestonePhase),
                Error::<T>::InvalidCampaignStatus
            );

            MilestoneStatuses::<T>::try_mutate(campaign_id, index, |maybe| -> DispatchResult {
                let status = maybe.as_mut().ok_or(Error::<T>::InvalidMilestoneIndex)?;
                ensure!(
                    matches!(status, MilestoneStatus::Submitted),
                    Error::<T>::InvalidMilestoneStatus
                );
                *status = MilestoneStatus::Approved;
                Ok(())
            })?;

            Self::deposit_event(Event::MilestoneApproved { campaign_id, index });
            Ok(())
        }

        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::reject_milestone())]
        pub fn reject_milestone(
            origin: OriginFor<T>,
            campaign_id: u32,
            index: u8,
        ) -> DispatchResult {
            T::MilestoneApprover::ensure_origin(origin)?;
            // P1-01: check campaign exists AND is in MilestonePhase
            let campaign = Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(
                matches!(campaign.status, CampaignStatus::MilestonePhase),
                Error::<T>::InvalidCampaignStatus
            );

            MilestoneStatuses::<T>::try_mutate(campaign_id, index, |maybe| -> DispatchResult {
                let status = maybe.as_mut().ok_or(Error::<T>::InvalidMilestoneIndex)?;
                ensure!(
                    matches!(status, MilestoneStatus::Submitted),
                    Error::<T>::InvalidMilestoneStatus
                );
                *status = MilestoneStatus::Rejected;
                Ok(())
            })?;

            Self::deposit_event(Event::MilestoneRejected { campaign_id, index });
            Ok(())
        }

        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::claim_milestone_funds())]
        pub fn claim_milestone_funds(
            origin: OriginFor<T>,
            campaign_id: u32,
            index: u8,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let mut campaign =
                Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(campaign.creator == who, Error::<T>::NotCampaignCreator);
            ensure!(
                matches!(campaign.status, CampaignStatus::MilestonePhase),
                Error::<T>::InvalidCampaignStatus
            );

            // DEADLOCK-FIX: License check removed from claim_milestone_funds.
            //
            // Same rationale as claim_funds: the milestone was already approved
            // by MilestoneApprover (a governance-controlled origin).  Blocking
            // the creator from claiming an approved milestone due to license
            // revocation would strand funds.  The MilestonePhase is still
            // cancellable via `report_license_revoked`, which protects
            // investors for unapproved milestones.

            // check milestone is approved
            let ms_status = MilestoneStatuses::<T>::get(campaign_id, index)
                .ok_or(Error::<T>::InvalidMilestoneIndex)?;
            ensure!(
                matches!(ms_status, MilestoneStatus::Approved),
                Error::<T>::InvalidMilestoneStatus
            );

            // get milestone definition
            let milestones = match &campaign.config.funding_model {
                FundingModel::MilestoneBased { milestones, .. } => milestones.clone(),
                _ => return Err(Error::<T>::InvalidFundingModel.into()),
            };
            let milestone =
                milestones.get(index as usize).ok_or(Error::<T>::InvalidMilestoneIndex)?;

            // compute release amount, capped at remaining funds to prevent
            // ceiling-rounding overshoot from consuming the creation deposit.
            let raw_release = Self::bps_of(campaign.total_raised, milestone.release_bps);
            let remaining = campaign.total_raised.saturating_sub(campaign.total_disbursed);
            let release_amount = raw_release.min(remaining);

            // CAT-3.9-C-I: use the fee locked at creation time
            let fee_bps = campaign.protocol_fee_bps;
            let fee = Self::bps_of(release_amount, fee_bps);
            let creator_amount = release_amount.saturating_sub(fee);

            let sub_account = Self::campaign_account(campaign_id);
            if !fee.is_zero() {
                let fee_recipient = Self::effective_protocol_fee_recipient();
                Self::do_transfer(
                    &campaign.config.funding_currency,
                    &sub_account,
                    &fee_recipient,
                    fee,
                    ExistenceRequirement::AllowDeath,
                )?;
                Self::deposit_event(Event::ProtocolFeeCollected { campaign_id, amount: fee });
            }
            Self::do_transfer(
                &campaign.config.funding_currency,
                &sub_account,
                &who,
                creator_amount,
                ExistenceRequirement::AllowDeath,
            )?;

            campaign.total_disbursed = campaign.total_disbursed.saturating_add(release_amount);
            MilestoneStatuses::<T>::insert(campaign_id, index, MilestoneStatus::Claimed);

            // check if all milestones are claimed
            let all_claimed = (0..milestones.len() as u8).all(|i| {
                MilestoneStatuses::<T>::get(campaign_id, i)
                    .map_or(false, |s| matches!(s, MilestoneStatus::Claimed))
            });
            if all_claimed {
                campaign.status = CampaignStatus::Completed;
            }

            Campaigns::<T>::insert(campaign_id, &campaign);

            // P1-02: emit net amount (after fee), not gross release_amount
            Self::deposit_event(Event::MilestoneFundsClaimed {
                campaign_id,
                index,
                amount: creator_amount,
            });

            // P2-09: CampaignCompleted event removed; completion is signalled
            // implicitly by the status transition to Completed. The deposit is
            // returned (with a CreationDepositClaimed event) via a separate
            // claim_creation_deposit call.
            Ok(())
        }

        // ─── Pause / Resume ─────────────────────────────────────────

        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::pause_campaign())]
        pub fn pause_campaign(origin: OriginFor<T>, campaign_id: u32) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            Campaigns::<T>::try_mutate(campaign_id, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                ensure!(
                    matches!(c.status, CampaignStatus::Funding),
                    Error::<T>::CampaignNotFunding
                );
                let now = frame_system::Pallet::<T>::block_number();
                // M-2: record the block at which we paused so resume can extend deadline
                c.paused_at = Some(now);
                c.status = CampaignStatus::Paused;
                Ok(())
            })?;
            Self::deposit_event(Event::CampaignPaused { campaign_id });
            Ok(())
        }

        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::resume_campaign())]
        pub fn resume_campaign(origin: OriginFor<T>, campaign_id: u32) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            Campaigns::<T>::try_mutate(campaign_id, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                ensure!(matches!(c.status, CampaignStatus::Paused), Error::<T>::CampaignNotPaused);
                let now = frame_system::Pallet::<T>::block_number();
                // M-2: extend the deadline by the duration of the pause so the effective
                // funding window for investors is not shortened by the pause.
                if let Some(paused_at) = c.paused_at {
                    let pause_duration = now.saturating_sub(paused_at);
                    c.config.deadline = c.config.deadline.saturating_add(pause_duration);
                }
                // CRIT-04: if the campaign has a linked license, re-validate that the
                // extended deadline still falls before the license expiry.  Without
                // this check, an admin can pause/resume to push the deadline past the
                // license expiry, creating a window for `report_license_revoked` to
                // cancel the campaign.
                if let (Some(asset_id), Some(part_id)) = (c.rwa_asset_id, c.participation_id) {
                    if let Some(expiry) = T::LicenseVerifier::license_expiry(asset_id, part_id) {
                        ensure!(
                            c.config.deadline < expiry,
                            Error::<T>::CampaignExceedsLicenseExpiry
                        );
                    }
                }
                c.paused_at = None;
                c.status = CampaignStatus::Funding;
                Ok(())
            })?;
            Self::deposit_event(Event::CampaignResumed { campaign_id });
            Ok(())
        }

        // ─── Whitelist ──────────────────────────────────────────────

        #[pallet::call_index(15)]
        #[pallet::weight(T::WeightInfo::add_to_whitelist())]
        pub fn add_to_whitelist(
            origin: OriginFor<T>,
            campaign_id: u32,
            account: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let campaign = Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(campaign.creator == who, Error::<T>::NotCampaignCreator);
            // P2-03: only allow whitelist modification on active campaigns
            ensure!(
                matches!(campaign.status, CampaignStatus::Funding | CampaignStatus::Paused),
                Error::<T>::InvalidCampaignStatus
            );
            // CAT-7.1-C-G: idempotent — only increment counter for new entries
            if !CampaignWhitelist::<T>::get(campaign_id, &account) {
                let count = CampaignWhitelistCount::<T>::get(campaign_id);
                ensure!(count < T::MaxWhitelistSize::get(), Error::<T>::WhitelistFull);
                CampaignWhitelistCount::<T>::insert(campaign_id, count + 1);
            }
            CampaignWhitelist::<T>::insert(campaign_id, &account, true);
            Ok(())
        }

        #[pallet::call_index(16)]
        #[pallet::weight(T::WeightInfo::remove_from_whitelist())]
        pub fn remove_from_whitelist(
            origin: OriginFor<T>,
            campaign_id: u32,
            account: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let campaign = Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(campaign.creator == who, Error::<T>::NotCampaignCreator);
            // P2-03: only allow whitelist modification on active campaigns
            ensure!(
                matches!(campaign.status, CampaignStatus::Funding | CampaignStatus::Paused),
                Error::<T>::InvalidCampaignStatus
            );
            // CAT-7.1-C-G: only decrement counter if entry actually existed
            if CampaignWhitelist::<T>::get(campaign_id, &account) {
                CampaignWhitelistCount::<T>::mutate(campaign_id, |c| *c = c.saturating_sub(1));
            }
            CampaignWhitelist::<T>::remove(campaign_id, &account);
            Ok(())
        }

        // ─── Protocol Config ────────────────────────────────────────

        #[pallet::call_index(17)]
        #[pallet::weight(T::WeightInfo::set_protocol_config())]
        pub fn set_protocol_config(
            origin: OriginFor<T>,
            new_fee_bps: u16,
            new_recipient: T::AccountId,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ensure!(new_fee_bps <= 10_000, Error::<T>::InvalidFeeBps);
            ProtocolFeeBpsOverride::<T>::put(new_fee_bps);
            ProtocolFeeRecipientOverride::<T>::put(&new_recipient);
            Self::deposit_event(Event::ProtocolConfigUpdated {
                fee_bps: new_fee_bps,
                recipient: new_recipient,
            });
            Ok(())
        }

        // ─── License Revocation ──────────────────────────────────────

        /// Permissionless: anyone can cancel a campaign whose linked RWA
        /// license has been revoked/slashed.  Investors can then claim
        /// refunds.
        #[pallet::call_index(18)]
        #[pallet::weight(T::WeightInfo::report_license_revoked())]
        pub fn report_license_revoked(origin: OriginFor<T>, campaign_id: u32) -> DispatchResult {
            ensure_signed(origin)?;
            Campaigns::<T>::try_mutate(campaign_id, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                // Only applies to campaigns with a linked license.
                let (asset_id, part_id) = match (c.rwa_asset_id, c.participation_id) {
                    (Some(a), Some(p)) => (a, p),
                    _ => return Err(Error::<T>::NoLinkedLicense.into()),
                };
                // CRIT-01: only affects campaigns still in a pre-finalization or
                // active-disbursement state.  Succeeded is intentionally excluded:
                // once a campaign has finalized to Succeeded the creator has a
                // guaranteed window to claim funds and must not be rug-pulled by a
                // permissionless license-revocation report.
                ensure!(
                    matches!(
                        c.status,
                        CampaignStatus::Funding
                            | CampaignStatus::Paused
                            | CampaignStatus::MilestonePhase
                    ),
                    Error::<T>::InvalidCampaignStatus
                );
                // Verify the license is indeed no longer active.
                ensure!(
                    !T::LicenseVerifier::is_license_active(asset_id, part_id),
                    Error::<T>::LicenseNotActive
                );
                // Clean up milestone statuses if applicable.
                if let FundingModel::MilestoneBased { milestones, .. } = &c.config.funding_model {
                    for i in 0..milestones.len() as u8 {
                        MilestoneStatuses::<T>::remove(campaign_id, i);
                    }
                }
                c.status = CampaignStatus::Cancelled;
                Ok(())
            })?;
            let _ = CampaignWhitelist::<T>::clear_prefix(campaign_id, u32::MAX, None);
            CampaignWhitelistCount::<T>::remove(campaign_id);
            Self::deposit_event(Event::CampaignLicenseReported { campaign_id });
            Self::deposit_event(Event::CampaignCancelled { campaign_id });
            Ok(())
        }

        // ─── Force Finalize ─────────────────────────────────────────────

        /// Force-finalize a campaign, bypassing the deadline check.
        ///
        /// Requires `ForceOrigin` (sudo).  The finalization outcome is still
        /// determined by on-chain state (total_raised vs. goal), but the
        /// deadline constraint is skipped so governance can resolve stuck
        /// campaigns or accelerate finalization when appropriate.
        #[pallet::call_index(19)]
        #[pallet::weight(T::WeightInfo::force_finalize_campaign())]
        pub fn force_finalize_campaign(origin: OriginFor<T>, campaign_id: u32) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;
            Campaigns::<T>::try_mutate(campaign_id, |maybe| -> DispatchResult {
                let c = maybe.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                ensure!(
                    matches!(c.status, CampaignStatus::Funding),
                    Error::<T>::InvalidCampaignStatus
                );
                // NOTE: deadline check intentionally skipped.

                let new_status = Self::do_finalize(campaign_id, c);
                c.status = new_status;
                Self::deposit_event(Event::CampaignForceFinalized {
                    campaign_id,
                    status: new_status,
                });
                Ok(())
            })
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Determine the finalization outcome for a Funding campaign and
        /// initialize milestone statuses if applicable.  Returns the new
        /// `CampaignStatus`.  Caller is responsible for writing it back
        /// and emitting the appropriate event.
        fn do_finalize(campaign_id: u32, c: &CampaignOf<T>) -> CampaignStatus {
            match &c.config.funding_model {
                FundingModel::AllOrNothing { goal } => {
                    if c.total_raised >= *goal {
                        CampaignStatus::Succeeded
                    } else {
                        CampaignStatus::Failed
                    }
                }
                FundingModel::KeepWhatYouRaise { soft_cap } => match soft_cap {
                    Some(cap) if c.total_raised < *cap => CampaignStatus::Failed,
                    _ => CampaignStatus::Succeeded,
                },
                FundingModel::MilestoneBased { goal, milestones } => {
                    if c.total_raised >= *goal {
                        for i in 0..milestones.len() {
                            MilestoneStatuses::<T>::insert(
                                campaign_id,
                                i as u8,
                                MilestoneStatus::Pending,
                            );
                        }
                        CampaignStatus::MilestonePhase
                    } else {
                        CampaignStatus::Failed
                    }
                }
            }
        }

        pub fn campaign_account(campaign_id: u32) -> T::AccountId {
            T::PalletId::get().into_sub_account_truncating(campaign_id)
        }

        /// Return the effective protocol fee in basis points.
        ///
        /// Uses the runtime override if set, otherwise falls back to the
        /// compile-time `Config::ProtocolFeeBps` constant.
        pub fn effective_protocol_fee_bps() -> u16 {
            ProtocolFeeBpsOverride::<T>::get().unwrap_or_else(T::ProtocolFeeBps::get)
        }

        /// Return the effective protocol fee recipient.
        ///
        /// Uses the runtime override if set, otherwise falls back to the
        /// compile-time `Config::ProtocolFeeRecipient` constant.
        pub fn effective_protocol_fee_recipient() -> T::AccountId {
            ProtocolFeeRecipientOverride::<T>::get().unwrap_or_else(T::ProtocolFeeRecipient::get)
        }

        fn do_transfer(
            currency: &PaymentCurrency<T::AssetId>,
            from: &T::AccountId,
            to: &T::AccountId,
            amount: BalanceOf<T>,
            existence_req: ExistenceRequirement,
        ) -> DispatchResult {
            if amount.is_zero() {
                return Ok(());
            }
            match currency {
                PaymentCurrency::Native => {
                    T::NativeCurrency::transfer(from, to, amount, existence_req)?;
                }
                PaymentCurrency::Asset(asset_id) => {
                    let keep_alive = matches!(existence_req, ExistenceRequirement::KeepAlive);
                    <T::Fungibles as fungibles::Transfer<T::AccountId>>::transfer(
                        *asset_id, from, to, amount, keep_alive,
                    )?;
                }
            }
            Ok(())
        }

        fn do_burn(
            currency: &PaymentCurrency<T::AssetId>,
            who: &T::AccountId,
            amount: BalanceOf<T>,
            existence_req: ExistenceRequirement,
        ) -> DispatchResult {
            if amount.is_zero() {
                return Ok(());
            }
            match currency {
                PaymentCurrency::Native => {
                    // L-1: use FEE reason, not TRANSFER — this is a penalty burn, not a
                    // transfer to another account.  Some Currency implementations gate
                    // certain withdrawal paths on the reason flag.
                    let imbalance = T::NativeCurrency::withdraw(
                        who,
                        amount,
                        WithdrawReasons::FEE,
                        existence_req,
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

        fn check_eligibility_inner(
            who: &T::AccountId,
            campaign: &CampaignOf<T>,
            campaign_id: u32,
        ) -> DispatchResult {
            for rule in campaign.eligibility_rules.iter() {
                let passes = match rule {
                    EligibilityRule::NativeBalance { min_balance } => {
                        T::NativeCurrency::free_balance(who) >= *min_balance
                    }
                    EligibilityRule::AssetBalance { asset_id, min_balance } => {
                        <T::Fungibles as fungibles::Inspect<T::AccountId>>::balance(*asset_id, who)
                            >= *min_balance
                    }
                    EligibilityRule::NftOwnership { required_sets } => {
                        required_sets.iter().any(|set| {
                            set.iter().all(|(cid, iid)| {
                                <T::NftInspect as nonfungibles_v2::Inspect<T::AccountId>>::owner(
                                    cid, iid,
                                )
                                .map_or(false, |o| o == *who)
                            })
                        })
                    }
                    EligibilityRule::AccountWhitelist => {
                        CampaignWhitelist::<T>::get(campaign_id, who)
                    }
                };
                ensure!(passes, Error::<T>::EligibilityCheckFailed);
            }
            Ok(())
        }

        /// Compute `bps` basis-points of `amount` using **ceiling** division.
        ///
        /// P0-01 / P2-10: clamps bps to [0, 10_000] as defense-in-depth so that
        /// even a misconfigured value can never exceed 100 %. `compute_penalty`
        /// has been merged into this single function.
        ///
        /// CAT-4.3-C-B fix: uses ceiling division `ceil(amount * bps / 10_000)`
        /// instead of `Permill` floor multiplication so that even the smallest
        /// non-zero amount incurs at least 1 unit of penalty, preventing a bot
        /// from splitting withdrawals into dust-sized pieces to avoid all
        /// penalty.
        pub fn bps_of(amount: BalanceOf<T>, bps: u16) -> BalanceOf<T> {
            let clamped = bps.min(10_000);
            if clamped == 0 {
                return Zero::zero();
            }
            let bps_balance: BalanceOf<T> = (clamped as u32).into();
            let divisor: BalanceOf<T> = 10_000u32.into();
            let one: BalanceOf<T> = 1u32.into();
            let numerator = amount.saturating_mul(bps_balance);
            // ceil(a / b) = (a + b - 1) / b
            numerator.saturating_add(divisor.saturating_sub(one)) / divisor
        }

        #[cfg(feature = "try-runtime")]
        fn do_try_state() -> Result<(), &'static str> {
            // 1. NextCampaignId >= campaign count
            let next_id = NextCampaignId::<T>::get();
            let campaign_count = Campaigns::<T>::iter().count() as u32;
            if next_id < campaign_count {
                return Err("NextCampaignId is less than the number of campaigns");
            }

            // 2. total_disbursed <= total_raised for all campaigns
            for (id, campaign) in Campaigns::<T>::iter() {
                if campaign.total_disbursed > campaign.total_raised {
                    frame_support::log::error!(
                        target: "pallet-crowdfunding",
                        "Campaign {}: total_disbursed ({:?}) > total_raised ({:?})",
                        id, campaign.total_disbursed, campaign.total_raised,
                    );
                    return Err("Campaign total_disbursed exceeds total_raised");
                }
            }

            // 3. total_withdrawn <= total_invested for all investments
            for (campaign_id, investor, investment) in Investments::<T>::iter() {
                if investment.total_withdrawn > investment.total_invested {
                    frame_support::log::error!(
                        target: "pallet-crowdfunding",
                        "Investment ({}, {:?}): total_withdrawn ({:?}) > total_invested ({:?})",
                        campaign_id, investor, investment.total_withdrawn, investment.total_invested,
                    );
                    return Err("Investment total_withdrawn exceeds total_invested");
                }
            }

            // 4. CreatorCampaigns entries point to existing campaigns
            for (creator, campaign_ids) in CreatorCampaigns::<T>::iter() {
                for &id in campaign_ids.iter() {
                    if !Campaigns::<T>::contains_key(id) {
                        frame_support::log::error!(
                            target: "pallet-crowdfunding",
                            "CreatorCampaigns for {:?} references non-existent campaign {}",
                            creator, id,
                        );
                        return Err("CreatorCampaigns references non-existent campaign");
                    }
                }
            }

            // 5. InvestorCampaigns entries point to existing campaigns
            for (investor, campaign_ids) in InvestorCampaigns::<T>::iter() {
                for &id in campaign_ids.iter() {
                    if !Campaigns::<T>::contains_key(id) {
                        frame_support::log::error!(
                            target: "pallet-crowdfunding",
                            "InvestorCampaigns for {:?} references non-existent campaign {}",
                            investor, id,
                        );
                        return Err("InvestorCampaigns references non-existent campaign");
                    }
                }
            }

            // L-4 invariant 1: investor_count consistency for non-terminal campaigns.
            // Skip terminal campaigns (Failed/Cancelled/Completed) — their investor
            // records may have been removed by refund/claim operations.
            for (id, campaign) in Campaigns::<T>::iter() {
                if matches!(
                    campaign.status,
                    CampaignStatus::Failed | CampaignStatus::Cancelled | CampaignStatus::Completed
                ) {
                    continue;
                }
                // Count investors with a positive net balance (total_invested >
                // total_withdrawn).
                let counted = Investments::<T>::iter_prefix(id)
                    .filter(|(_, inv)| inv.total_invested > inv.total_withdrawn)
                    .count() as u32;
                if campaign.investor_count != counted {
                    frame_support::log::error!(
                        target: "pallet-crowdfunding",
                        "Campaign {}: investor_count ({}) does not match active investment count ({})",
                        id, campaign.investor_count, counted,
                    );
                    return Err("Campaign investor_count inconsistent with Investments");
                }
            }

            // L-4 invariant 2: CampaignWhitelist entries reference existing campaigns.
            for (id, _account, _val) in CampaignWhitelist::<T>::iter() {
                if !Campaigns::<T>::contains_key(id) {
                    frame_support::log::error!(
                        target: "pallet-crowdfunding",
                        "CampaignWhitelist references non-existent campaign {}",
                        id,
                    );
                    return Err("CampaignWhitelist references non-existent campaign");
                }
            }

            // L-4 invariant 3: MilestoneStatuses reference campaigns in MilestonePhase
            // or a terminal state that passed through it (Completed from milestone path).
            for (id, _index, _status) in MilestoneStatuses::<T>::iter() {
                match Campaigns::<T>::get(id) {
                    None => {
                        frame_support::log::error!(
                            target: "pallet-crowdfunding",
                            "MilestoneStatuses references non-existent campaign {}",
                            id,
                        );
                        return Err("MilestoneStatuses references non-existent campaign");
                    }
                    Some(c)
                        if matches!(
                            c.status,
                            CampaignStatus::MilestonePhase | CampaignStatus::Completed
                        ) => {}
                    Some(c) => {
                        frame_support::log::error!(
                            target: "pallet-crowdfunding",
                            "MilestoneStatuses exists for campaign {} with status {:?}",
                            id, c.status,
                        );
                        return Err(
                            "MilestoneStatuses exists for campaign not in MilestonePhase/Completed"
                        );
                    }
                }
            }

            Ok(())
        }
    }
}
