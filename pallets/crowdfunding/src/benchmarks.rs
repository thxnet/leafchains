#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{
    traits::{Currency, Get},
    BoundedVec,
};
use frame_system::RawOrigin;
use sp_runtime::{traits::Bounded, Saturating};
use sp_std::vec;

use super::*;
use crate::pallet::{
    BalanceOf, CampaignConfigOf, CampaignWhitelist, Campaigns, EligibilityRuleOf, Investments,
    MilestoneStatuses, NextCampaignId,
};

/// Investment/goal amount large enough to exceed existential deposit on any
/// runtime. CampaignCreationDeposit is 50 DOLLARS = 500_000_000_000 in runtime,
/// so we use 1_000_000_000_000 (100 DOLLARS) as the goal/investment amount.
fn benchmark_amount<T: Config>() -> BalanceOf<T> {
    T::CampaignCreationDeposit::get().saturating_mul(2u32.into())
}

// Helper to create a valid campaign config
fn campaign_config<T: Config>(deadline: T::BlockNumber) -> CampaignConfigOf<T> {
    CampaignConfig {
        funding_model: FundingModel::AllOrNothing { goal: benchmark_amount::<T>() },
        funding_currency: PaymentCurrency::Native,
        deadline,
        hard_cap: None,
        min_investment: None,
        max_investment_per_investor: None,
        metadata_hash: [0u8; 32],
        early_withdrawal_penalty_bps: None,
    }
}

// Helper to create a milestone config
fn milestone_campaign_config<T: Config>(deadline: T::BlockNumber) -> CampaignConfigOf<T> {
    let milestones = vec![
        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
    ];
    CampaignConfig {
        funding_model: FundingModel::MilestoneBased {
            goal: benchmark_amount::<T>(),
            milestones: milestones.try_into().expect("within max milestones"),
        },
        funding_currency: PaymentCurrency::Native,
        deadline,
        hard_cap: None,
        min_investment: None,
        max_investment_per_investor: None,
        metadata_hash: [0u8; 32],
        early_withdrawal_penalty_bps: None,
    }
}

// Setup a campaign in Funding status and return (campaign_id, creator)
fn setup_funded_campaign<T: Config>() -> (u32, T::AccountId) {
    let caller: T::AccountId = whitelisted_caller();
    T::NativeCurrency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
    let now = frame_system::Pallet::<T>::block_number();
    let deadline = now + T::MinCampaignDuration::get() + 1u32.into();
    let config = campaign_config::<T>(deadline);
    Pallet::<T>::create_campaign(RawOrigin::Signed(caller.clone()).into(), config, None, None)
        .expect("create_campaign failed");
    let id = NextCampaignId::<T>::get() - 1;
    (id, caller)
}

// Setup a campaign with an investor
fn setup_invested_campaign<T: Config>() -> (u32, T::AccountId, T::AccountId) {
    let (id, creator) = setup_funded_campaign::<T>();
    let investor: T::AccountId = account("investor", 0, 0);
    T::NativeCurrency::make_free_balance_be(&investor, BalanceOf::<T>::max_value() / 2u32.into());
    let invest_amount = benchmark_amount::<T>();
    Pallet::<T>::invest(RawOrigin::Signed(investor.clone()).into(), id, invest_amount)
        .expect("invest failed");
    (id, creator, investor)
}

benchmarks! {
    create_campaign {
        let caller: T::AccountId = whitelisted_caller();
        T::NativeCurrency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        let now = frame_system::Pallet::<T>::block_number();
        let deadline = now + T::MinCampaignDuration::get() + 1u32.into();
        let config = campaign_config::<T>(deadline);
    }: _(RawOrigin::Signed(caller), config, None, None)
    verify {
        assert!(Campaigns::<T>::contains_key(0u32));
    }

    cancel_campaign {
        let (id, _creator) = setup_funded_campaign::<T>();
    }: _(RawOrigin::Root, id)
    verify {
        let c = Campaigns::<T>::get(id).unwrap();
        assert!(matches!(c.status, CampaignStatus::Cancelled));
    }

    set_default_eligibility {
        let rules: BoundedVec<EligibilityRuleOf<T>, T::MaxEligibilityRules> = Default::default();
    }: _(RawOrigin::Root, rules)

    invest {
        let (id, _creator) = setup_funded_campaign::<T>();
        let investor: T::AccountId = account("investor", 0, 0);
        T::NativeCurrency::make_free_balance_be(&investor, BalanceOf::<T>::max_value() / 2u32.into());
        let invest_amount = benchmark_amount::<T>() / 2u32.into();
    }: _(RawOrigin::Signed(investor), id, invest_amount)
    verify {
        assert!(Investments::<T>::contains_key(id, account::<T::AccountId>("investor", 0, 0)));
    }

    withdraw_investment {
        let (id, _creator, investor) = setup_invested_campaign::<T>();
        let withdraw_amount = benchmark_amount::<T>() / 10u32.into();
    }: _(RawOrigin::Signed(investor), id, withdraw_amount)

    claim_refund {
        let (id, _creator, investor) = setup_invested_campaign::<T>();
        // Cancel the campaign so refund is possible
        Pallet::<T>::cancel_campaign(RawOrigin::Root.into(), id).expect("cancel failed");
    }: _(RawOrigin::Signed(investor), id)

    finalize_campaign {
        let (id, creator, _investor) = setup_invested_campaign::<T>();
        // Advance past deadline
        let campaign = Campaigns::<T>::get(id).unwrap();
        frame_system::Pallet::<T>::set_block_number(campaign.config.deadline + 1u32.into());
    }: _(RawOrigin::Signed(creator), id)

    claim_funds {
        let (id, creator, _investor) = setup_invested_campaign::<T>();
        let campaign = Campaigns::<T>::get(id).unwrap();
        frame_system::Pallet::<T>::set_block_number(campaign.config.deadline + 1u32.into());
        // Mutate status directly for benchmark simplicity
        Campaigns::<T>::mutate(id, |c| {
            if let Some(c) = c {
                c.status = CampaignStatus::Succeeded;
            }
        });
    }: _(RawOrigin::Signed(creator), id)

    claim_creation_deposit {
        let (id, creator) = setup_funded_campaign::<T>();
        Pallet::<T>::cancel_campaign(RawOrigin::Root.into(), id).expect("cancel failed");
    }: _(RawOrigin::Signed(creator), id)

    submit_milestone {
        let caller: T::AccountId = whitelisted_caller();
        T::NativeCurrency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        let now = frame_system::Pallet::<T>::block_number();
        let deadline = now + T::MinCampaignDuration::get() + 1u32.into();
        let config = milestone_campaign_config::<T>(deadline);
        Pallet::<T>::create_campaign(
            RawOrigin::Signed(caller.clone()).into(),
            config,
            None,
            None,
        ).expect("create failed");
        let id = NextCampaignId::<T>::get() - 1;
        let investor: T::AccountId = account("investor", 0, 0);
        T::NativeCurrency::make_free_balance_be(&investor, BalanceOf::<T>::max_value() / 2u32.into());
        Pallet::<T>::invest(
            RawOrigin::Signed(investor).into(),
            id,
            benchmark_amount::<T>(),
        ).expect("invest failed");
        let campaign = Campaigns::<T>::get(id).unwrap();
        frame_system::Pallet::<T>::set_block_number(campaign.config.deadline + 1u32.into());
        Pallet::<T>::finalize_campaign(
            RawOrigin::Signed(caller.clone()).into(),
            id,
        ).expect("finalize failed");
    }: _(RawOrigin::Signed(caller), id, 0u8)
    verify {
        assert_eq!(
            MilestoneStatuses::<T>::get(id, 0u8),
            Some(MilestoneStatus::Submitted)
        );
    }

    approve_milestone {
        let caller: T::AccountId = whitelisted_caller();
        T::NativeCurrency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        let now = frame_system::Pallet::<T>::block_number();
        let deadline = now + T::MinCampaignDuration::get() + 1u32.into();
        let config = milestone_campaign_config::<T>(deadline);
        Pallet::<T>::create_campaign(
            RawOrigin::Signed(caller.clone()).into(),
            config,
            None,
            None,
        ).expect("create failed");
        let id = NextCampaignId::<T>::get() - 1;
        let investor: T::AccountId = account("investor", 0, 0);
        T::NativeCurrency::make_free_balance_be(&investor, BalanceOf::<T>::max_value() / 2u32.into());
        Pallet::<T>::invest(
            RawOrigin::Signed(investor).into(),
            id,
            benchmark_amount::<T>(),
        ).expect("invest failed");
        let campaign = Campaigns::<T>::get(id).unwrap();
        frame_system::Pallet::<T>::set_block_number(campaign.config.deadline + 1u32.into());
        Pallet::<T>::finalize_campaign(
            RawOrigin::Signed(caller.clone()).into(),
            id,
        ).expect("finalize failed");
        Pallet::<T>::submit_milestone(
            RawOrigin::Signed(caller).into(),
            id,
            0u8,
        ).expect("submit failed");
    }: _(RawOrigin::Root, id, 0u8)
    verify {
        assert_eq!(
            MilestoneStatuses::<T>::get(id, 0u8),
            Some(MilestoneStatus::Approved)
        );
    }

    reject_milestone {
        let caller: T::AccountId = whitelisted_caller();
        T::NativeCurrency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        let now = frame_system::Pallet::<T>::block_number();
        let deadline = now + T::MinCampaignDuration::get() + 1u32.into();
        let config = milestone_campaign_config::<T>(deadline);
        Pallet::<T>::create_campaign(
            RawOrigin::Signed(caller.clone()).into(),
            config,
            None,
            None,
        ).expect("create failed");
        let id = NextCampaignId::<T>::get() - 1;
        let investor: T::AccountId = account("investor", 0, 0);
        T::NativeCurrency::make_free_balance_be(&investor, BalanceOf::<T>::max_value() / 2u32.into());
        Pallet::<T>::invest(
            RawOrigin::Signed(investor).into(),
            id,
            benchmark_amount::<T>(),
        ).expect("invest failed");
        let campaign = Campaigns::<T>::get(id).unwrap();
        frame_system::Pallet::<T>::set_block_number(campaign.config.deadline + 1u32.into());
        Pallet::<T>::finalize_campaign(
            RawOrigin::Signed(caller.clone()).into(),
            id,
        ).expect("finalize failed");
        Pallet::<T>::submit_milestone(
            RawOrigin::Signed(caller).into(),
            id,
            0u8,
        ).expect("submit failed");
    }: _(RawOrigin::Root, id, 0u8)
    verify {
        assert_eq!(
            MilestoneStatuses::<T>::get(id, 0u8),
            Some(MilestoneStatus::Rejected)
        );
    }

    claim_milestone_funds {
        let caller: T::AccountId = whitelisted_caller();
        T::NativeCurrency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 2u32.into());
        let now = frame_system::Pallet::<T>::block_number();
        let deadline = now + T::MinCampaignDuration::get() + 1u32.into();
        let config = milestone_campaign_config::<T>(deadline);
        Pallet::<T>::create_campaign(
            RawOrigin::Signed(caller.clone()).into(),
            config,
            None,
            None,
        ).expect("create failed");
        let id = NextCampaignId::<T>::get() - 1;
        let investor: T::AccountId = account("investor", 0, 0);
        T::NativeCurrency::make_free_balance_be(&investor, BalanceOf::<T>::max_value() / 2u32.into());
        Pallet::<T>::invest(
            RawOrigin::Signed(investor).into(),
            id,
            benchmark_amount::<T>(),
        ).expect("invest failed");
        let campaign = Campaigns::<T>::get(id).unwrap();
        frame_system::Pallet::<T>::set_block_number(campaign.config.deadline + 1u32.into());
        Pallet::<T>::finalize_campaign(
            RawOrigin::Signed(caller.clone()).into(),
            id,
        ).expect("finalize failed");
        Pallet::<T>::submit_milestone(
            RawOrigin::Signed(caller.clone()).into(),
            id,
            0u8,
        ).expect("submit failed");
        Pallet::<T>::approve_milestone(
            RawOrigin::Root.into(),
            id,
            0u8,
        ).expect("approve failed");
    }: _(RawOrigin::Signed(caller), id, 0u8)
    verify {
        assert_eq!(
            MilestoneStatuses::<T>::get(id, 0u8),
            Some(MilestoneStatus::Claimed)
        );
    }

    pause_campaign {
        let (id, _creator) = setup_funded_campaign::<T>();
    }: _(RawOrigin::Root, id)
    verify {
        let c = Campaigns::<T>::get(id).unwrap();
        assert!(matches!(c.status, CampaignStatus::Paused));
    }

    resume_campaign {
        let (id, _creator) = setup_funded_campaign::<T>();
        Pallet::<T>::pause_campaign(RawOrigin::Root.into(), id).expect("pause failed");
    }: _(RawOrigin::Root, id)
    verify {
        let c = Campaigns::<T>::get(id).unwrap();
        assert!(matches!(c.status, CampaignStatus::Funding));
    }

    add_to_whitelist {
        let (id, creator) = setup_funded_campaign::<T>();
        let target: T::AccountId = account("target", 0, 0);
    }: _(RawOrigin::Signed(creator), id, target.clone())
    verify {
        assert!(CampaignWhitelist::<T>::get(id, &target));
    }

    remove_from_whitelist {
        let (id, creator) = setup_funded_campaign::<T>();
        let target: T::AccountId = account("target", 0, 0);
        Pallet::<T>::add_to_whitelist(
            RawOrigin::Signed(creator.clone()).into(),
            id,
            target.clone(),
        ).expect("add failed");
    }: _(RawOrigin::Signed(creator), id, target.clone())
    verify {
        assert!(!CampaignWhitelist::<T>::get(id, &target));
    }

    set_protocol_config {
        let recipient: T::AccountId = account("recipient", 0, 0);
    }: _(RawOrigin::Root, 500u16, recipient)

    report_license_revoked {
        // Setup: create a campaign, then mutate it to have a linked license
        // and mark the license as revoked so the extrinsic succeeds.
        let (id, creator) = setup_funded_campaign::<T>();
        // Mutate the campaign to add a linked license (rwa_asset_id=0, participation_id=0)
        Campaigns::<T>::mutate(id, |c| {
            if let Some(c) = c {
                c.rwa_asset_id = Some(0);
                c.participation_id = Some(0);
            }
        });
        // The LicenseVerifier for benchmarks is `()` which returns
        // `is_license_active() = true`. For the benchmark to succeed we need
        // `is_license_active()` to return false. Since benchmarks run against
        // the runtime's Config (not the mock), we manipulate state:
        // the `()` impl always returns true for is_license_active, so we
        // need to work around this.
        //
        // For runtime benchmarks, the LicenseVerifier is the real RwaLicenseVerifier.
        // Since no RWA asset exists with id=0, is_license_active will return false.
        // This is the correct worst-case scenario.
        let reporter: T::AccountId = account("reporter", 0, 0);
    }: _(RawOrigin::Signed(reporter), id)
    verify {
        let c = Campaigns::<T>::get(id).unwrap();
        assert!(matches!(c.status, CampaignStatus::Cancelled));
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::ExtBuilder::default().build(), crate::mock::Test);
}
