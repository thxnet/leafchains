/// Storage migrations for pallet-crowdfunding.
pub mod v3 {
    use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade, weights::Weight, BoundedVec};

    use crate::{
        pallet::{self, Config},
        types::*,
    };

    /// Old Campaign struct WITHOUT the `protocol_fee_bps` field (storage
    /// version 2).
    #[derive(
        frame_support::CloneNoBound,
        frame_support::PartialEqNoBound,
        frame_support::EqNoBound,
        codec::Encode,
        codec::Decode,
        frame_support::RuntimeDebugNoBound,
        scale_info::TypeInfo,
        MaxEncodedLen,
    )]
    #[scale_info(skip_type_params(MaxMilestones, MaxEligibilityRules, MaxNftSets, MaxNftsPerSet))]
    pub struct OldCampaign<
        AccountId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
        Balance: Clone + PartialEq + Eq + sp_std::fmt::Debug,
        BlockNumber: Clone + PartialEq + Eq + sp_std::fmt::Debug,
        AssetId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
        CollectionId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
        ItemId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
        MaxMilestones: Get<u32>,
        MaxEligibilityRules: Get<u32>,
        MaxNftSets: Get<u32>,
        MaxNftsPerSet: Get<u32>,
    > {
        pub creator: AccountId,
        pub status: CampaignStatus,
        pub config: CampaignConfig<Balance, BlockNumber, AssetId, MaxMilestones>,
        pub eligibility_rules: BoundedVec<
            EligibilityRule<AssetId, Balance, CollectionId, ItemId, MaxNftSets, MaxNftsPerSet>,
            MaxEligibilityRules,
        >,
        pub total_raised: Balance,
        pub total_disbursed: Balance,
        pub investor_count: u32,
        pub creation_deposit: Balance,
        pub created_at: BlockNumber,
        pub paused_at: Option<BlockNumber>,
        pub rwa_asset_id: Option<u32>,
        pub participation_id: Option<u32>,
    }

    type OldCampaignOf<T> = OldCampaign<
        <T as frame_system::Config>::AccountId,
        pallet::BalanceOf<T>,
        <T as frame_system::Config>::BlockNumber,
        <T as Config>::AssetId,
        <T as Config>::CollectionId,
        <T as Config>::ItemId,
        <T as Config>::MaxMilestones,
        <T as Config>::MaxEligibilityRules,
        <T as Config>::MaxNftSets,
        <T as Config>::MaxNftsPerSet,
    >;

    pub struct MigrateToV3<T>(sp_std::marker::PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for MigrateToV3<T> {
        fn on_runtime_upgrade() -> Weight {
            let current = pallet::Pallet::<T>::on_chain_storage_version();
            if current != 2 {
                frame_support::log::info!(
                    target: "pallet-crowdfunding",
                    "MigrateToV3: on-chain version is {:?}, skipping",
                    current
                );
                return Weight::zero();
            }

            let default_fee_bps = T::ProtocolFeeBps::get();
            let mut count: u64 = 0;

            pallet::Campaigns::<T>::translate::<OldCampaignOf<T>, _>(|_id, old| {
                count += 1;
                Some(Campaign {
                    creator: old.creator,
                    status: old.status,
                    config: old.config,
                    eligibility_rules: old.eligibility_rules,
                    total_raised: old.total_raised,
                    total_disbursed: old.total_disbursed,
                    investor_count: old.investor_count,
                    creation_deposit: old.creation_deposit,
                    created_at: old.created_at,
                    paused_at: old.paused_at,
                    rwa_asset_id: old.rwa_asset_id,
                    participation_id: old.participation_id,
                    protocol_fee_bps: default_fee_bps,
                })
            });

            StorageVersion::new(3).put::<pallet::Pallet<T>>();

            frame_support::log::info!(
                target: "pallet-crowdfunding",
                "MigrateToV3: migrated {} campaigns, set protocol_fee_bps={}",
                count, default_fee_bps,
            );

            T::DbWeight::get().reads_writes(count + 1, count + 1)
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
            let count = pallet::Campaigns::<T>::iter().count() as u32;
            Ok(count.encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
            let old_count = u32::decode(&mut &state[..]).map_err(|_| "decode failed")?;
            let new_count = pallet::Campaigns::<T>::iter().count() as u32;
            frame_support::ensure!(old_count == new_count, "campaign count mismatch");
            // verify all campaigns now have protocol_fee_bps set
            for (_id, campaign) in pallet::Campaigns::<T>::iter() {
                // non-zero check only if the config default is non-zero
                if T::ProtocolFeeBps::get() > 0 {
                    frame_support::ensure!(
                        campaign.protocol_fee_bps > 0,
                        "protocol_fee_bps should be non-zero after migration"
                    );
                }
            }
            Ok(())
        }
    }
}
