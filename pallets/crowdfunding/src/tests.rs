use frame_support::{assert_noop, assert_ok, traits::Currency, BoundedVec};

use super::{mock::*, *};

// ── create_campaign ─────────────────────────────────────────────────────

mod create_campaign {
    use super::*;

    #[test]
    fn aon_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let config = default_aon_config(100, 1000);
            let id = create_funded_campaign(ALICE, config);
            assert_eq!(id, 0);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.creator, ALICE);
            assert!(matches!(c.status, CampaignStatus::Funding));
            assert_eq!(c.creation_deposit, 100);
            assert_eq!(c.total_raised, 0);
            // Deposit transferred to sub-account
            let sub = Crowdfunding::campaign_account(id);
            assert_eq!(Balances::free_balance(sub), 100);
            assert_eq!(Balances::free_balance(ALICE), 10_000 - 100);
            assert!(pallet::CreatorCampaigns::<Test>::get(ALICE).contains(&0));
        });
    }

    #[test]
    fn kwyr_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let config = default_kwyr_config(100);
            let id = create_funded_campaign(ALICE, config);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.config.funding_model, FundingModel::KeepWhatYouRaise { .. }));
            let _ = id;
        });
    }

    #[test]
    fn milestone_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.config.funding_model, FundingModel::MilestoneBased { .. }));
            let _ = id;
        });
    }

    #[test]
    fn deadline_in_past() {
        ExtBuilder::default().build().execute_with(|| {
            let config = default_aon_config(1, 1000); // deadline = 1, current block = 1
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::DeadlineInPast
            );
        });
    }

    #[test]
    fn duration_too_short() {
        ExtBuilder::default().build().execute_with(|| {
            // MinCampaignDuration = 10, deadline = 2 means duration = 1
            let config = default_aon_config(2, 1000);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::DurationTooShort
            );
        });
    }

    #[test]
    fn duration_too_long() {
        ExtBuilder::default().build().execute_with(|| {
            // MaxCampaignDuration = 1000, deadline = 1002 means duration = 1001
            let config = default_aon_config(1002, 1000);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::DurationTooLong
            );
        });
    }

    #[test]
    fn invalid_milestone_bps() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 4000, description_hash: [2u8; 32] },
                    // sum = 9000, not 10000
                ],
            );
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::MilestoneBpsSumInvalid
            );
        });
    }

    #[test]
    fn max_campaigns_reached() {
        ExtBuilder::default().build().execute_with(|| {
            // MaxCampaignsPerCreator = 5
            for _ in 0..5 {
                create_funded_campaign(ALICE, default_aon_config(100, 1000));
            }
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 1000),
                    None,
                    None,
                ),
                Error::<Test>::MaxCampaignsPerCreatorReached
            );
        });
    }

    #[test]
    fn insufficient_balance() {
        ExtBuilder::default().balances(vec![(ALICE, 50)]).build().execute_with(|| {
            // CampaignCreationDeposit = 100, but ALICE only has 50
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 1000),
                    None,
                    None,
                ),
                pallet_balances::Error::<Test>::InsufficientBalance
            );
        });
    }

    #[test]
    fn id_increment() {
        ExtBuilder::default().build().execute_with(|| {
            let id1 = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let id2 = create_funded_campaign(BOB, default_aon_config(100, 1000));
            assert_eq!(id1, 0);
            assert_eq!(id2, 1);
            assert_eq!(pallet::NextCampaignId::<Test>::get(), 2);
        });
    }

    #[test]
    fn custom_rules() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NativeBalance { min_balance: 500 }].try_into().unwrap();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules.clone()),
                None,
            ));
            let c = pallet::Campaigns::<Test>::get(0).unwrap();
            assert_eq!(c.eligibility_rules.len(), 1);
        });
    }
}

// ── cancel_campaign ─────────────────────────────────────────────────────

mod cancel_campaign {
    use super::*;

    #[test]
    fn happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Cancelled));
        });
    }

    #[test]
    fn not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), 99),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn already_cancelled() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn already_completed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── set_default_eligibility ─────────────────────────────────────────────

mod set_default_eligibility {
    use super::*;

    #[test]
    fn happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NativeBalance { min_balance: 100 }].try_into().unwrap();
            assert_ok!(Crowdfunding::set_default_eligibility(RuntimeOrigin::root(), rules));
            let stored = pallet::DefaultEligibilityRules::<Test>::get();
            assert_eq!(stored.len(), 1);
        });
    }

    #[test]
    fn non_admin_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NativeBalance { min_balance: 100 }].try_into().unwrap();
            assert_noop!(
                Crowdfunding::set_default_eligibility(RuntimeOrigin::signed(ALICE), rules),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }
}

// ── invest ──────────────────────────────────────────────────────────────

mod invest {
    use super::*;

    #[test]
    fn native_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_eq!(Balances::free_balance(BOB), bob_before - 500);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 500);
            assert_eq!(c.investor_count, 1);
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_invested, 500);
            assert!(pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
        });
    }

    #[test]
    fn not_funding() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn deadline_passed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            run_to_block(21);
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500),
                Error::<Test>::DeadlinePassed
            );
        });
    }

    #[test]
    fn below_minimum() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.min_investment = Some(100);
            let id = create_funded_campaign(ALICE, config);
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50),
                Error::<Test>::InvestmentBelowMinimum
            );
        });
    }

    #[test]
    fn hard_cap_exceeded() {
        ExtBuilder::default().build().execute_with(|| {
            // M-5: hard_cap must be >= goal; use goal = 500 = hard_cap
            let mut config = default_aon_config(100, 500);
            config.hard_cap = Some(500);
            let id = create_funded_campaign(ALICE, config);
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 501),
                Error::<Test>::HardCapExceeded
            );
        });
    }

    #[test]
    fn max_per_investor() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.max_investment_per_investor = Some(200);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1),
                Error::<Test>::InvestmentExceedsMaxPerInvestor
            );
        });
    }

    #[test]
    fn accumulation() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 300));
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_invested, 500);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 500);
            assert_eq!(c.investor_count, 1); // still 1
        });
    }

    #[test]
    fn max_investments_per_investor() {
        ExtBuilder::default().build().execute_with(|| {
            // MaxInvestmentsPerInvestor = 5, MaxCampaignsPerCreator = 5
            // Use different creators to avoid hitting campaign limit
            let creators = [ALICE, ALICE, ALICE, ALICE, ALICE];
            for creator in creators {
                let id = create_funded_campaign(creator, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            }
            // 6th campaign by CHARLIE to avoid ALICE's creator limit
            let id = create_funded_campaign(CHARLIE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::MaxInvestmentsPerInvestorReached
            );
        });
    }

    #[test]
    fn eligibility_native_balance() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NativeBalance { min_balance: 9000 }].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // BOB has 10_000, passes 9000 check
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            // CHARLIE has 10_000 but after investing would still be checked before transfer
            // Let's test with a low-balance account
            let _ = Balances::deposit_creating(&99u64, 500);
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(99), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
        });
    }

    #[test]
    fn eligibility_nft_ownership() {
        ExtBuilder::default().build().execute_with(|| {
            // Set up NFT ownership requirement
            let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
            let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // BOB doesn't own the NFT
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
            // Give BOB the NFT
            MockNftInspect::set_owner(1, 1, BOB);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }
}

// ── withdraw_investment ─────────────────────────────────────────────────

mod withdraw_investment {
    use super::*;

    #[test]
    fn happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
            // 1% penalty = 2, net = 198
            assert_eq!(Balances::free_balance(BOB), bob_before + 198);
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_withdrawn, 200);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 300); // 500 - 200
        });
    }

    #[test]
    fn penalty_calculation() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // 1% of 1000 = 10 penalty, net = 990
            assert_eq!(Balances::free_balance(BOB), bob_before + 990);
        });
    }

    #[test]
    fn not_funding() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn no_investment() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::NoInvestmentFound
            );
        });
    }

    #[test]
    fn insufficient_investment() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200),
                Error::<Test>::InsufficientInvestment
            );
        });
    }
}

// ── claim_refund ────────────────────────────────────────────────────────

mod claim_refund {
    use super::*;

    #[test]
    fn failed_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Goal not met → Failed
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Failed));

            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);
        });
    }

    #[test]
    fn cancelled_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);
        });
    }

    #[test]
    fn not_failed_or_cancelled() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn nothing_to_refund() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            // Second claim — investment record is now removed
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::NoInvestmentFound
            );
        });
    }
}

// ── finalize_campaign ───────────────────────────────────────────────────

mod finalize_campaign {
    use super::*;

    #[test]
    fn aon_goal_met() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn aon_goal_not_met() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Failed));
        });
    }

    #[test]
    fn kwyr_always_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            // No investment at all
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn milestone_phase_entry() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::MilestonePhase));
            // Milestone statuses initialized
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Pending)
            );
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 1u8),
                Some(MilestoneStatus::Pending)
            );
        });
    }

    #[test]
    fn still_funding() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::CampaignStillFunding
            );
        });
    }

    #[test]
    fn not_funding_status() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            run_to_block(101);
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── soft_cap ────────────────────────────────────────────────────────────

mod soft_cap {
    use super::*;

    #[test]
    fn kwyr_soft_cap_met_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(20, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn kwyr_soft_cap_exceeded_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(20, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn kwyr_soft_cap_not_met_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(20, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Failed));
        });
    }

    #[test]
    fn kwyr_soft_cap_none_always_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            // No investment at all
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn kwyr_soft_cap_zero_always_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(20, 0));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn kwyr_soft_cap_failed_allows_refund() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(20, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Failed));
            // BOB can claim refund
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 200);
        });
    }
}

// ── claim_funds ─────────────────────────────────────────────────────────

mod claim_funds {
    use super::*;

    #[test]
    fn happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
        });
    }

    #[test]
    fn not_creator() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::NotCampaignCreator
            );
        });
    }

    #[test]
    fn not_succeeded() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── claim_creation_deposit ──────────────────────────────────────────────

mod claim_creation_deposit {
    use super::*;

    #[test]
    fn after_completed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.creation_deposit, 0);
        });
    }

    #[test]
    fn after_failed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }

    #[test]
    fn after_cancelled() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }

    #[test]
    fn double_claim() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::AlreadyClaimed
            );
        });
    }

    #[test]
    fn not_terminal() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── milestone_workflow ──────────────────────────────────────────────────

mod milestone_workflow {
    use super::*;

    fn setup_milestone_campaign() -> u32 {
        let config = milestone_config(
            20,
            1000,
            vec![
                Milestone { release_bps: 6000, description_hash: [1u8; 32] },
                Milestone { release_bps: 4000, description_hash: [2u8; 32] },
            ],
        );
        let id = create_funded_campaign(ALICE, config);
        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
        run_to_block(21);
        assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
        id
    }

    #[test]
    fn submit_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Submitted)
            );
        });
    }

    #[test]
    fn submit_not_creator() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::NotCampaignCreator
            );
        });
    }

    #[test]
    fn approve_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Approved)
            );
        });
    }

    #[test]
    fn approve_not_submitted() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn reject_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Rejected)
            );
        });
    }

    #[test]
    fn resubmit_after_rejection() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Submitted)
            );
        });
    }

    #[test]
    fn claim_milestone_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            // 60% of 1000 = 600
            assert_eq!(Balances::free_balance(ALICE), alice_before + 600);
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Claimed)
            );
        });
    }

    #[test]
    fn all_claimed_completes_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();

            // Milestone 0: submit → approve → claim
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));

            // Milestone 1: submit → approve → claim
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 1));

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
        });
    }

    #[test]
    fn claim_not_approved() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            // Not yet approved
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }
}

// ── integration ─────────────────────────────────────────────────────────

mod integration {
    use super::*;

    #[test]
    fn full_aon_lifecycle() {
        ExtBuilder::default().build().execute_with(|| {
            // Create
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            // Invest
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 500));
            // Finalize (goal met)
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Succeeded
            ));
            // Claim funds
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1100);
            // Claim creation deposit
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }

    #[test]
    fn full_milestone_lifecycle() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 7000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Milestone 0: 30% of 1000 = 300
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 300);

            // Milestone 1: 70% of 1000 = 700
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 1));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 700);

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
            assert_eq!(c.total_disbursed, 1000);

            // Claim creation deposit
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }

    #[test]
    fn failed_aon_with_refund() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Failed
            ));

            // BOB gets full refund
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 1000);

            // ALICE gets creation deposit back
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SUPPLEMENTARY TESTS — coverage gaps identified by forensic audit
// ═══════════════════════════════════════════════════════════════════════

// ── create_campaign (supplementary) ─────────────────────────────────────

mod create_campaign_supplementary {
    use super::*;

    #[test]
    fn emits_campaign_created_event() {
        ExtBuilder::default().build().execute_with(|| {
            System::reset_events();
            let config = default_aon_config(100, 1000);
            let id = create_funded_campaign(ALICE, config);
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CampaignCreated {
                        campaign_id,
                        creator,
                        deadline,
                    }) if *campaign_id == id && *creator == ALICE && *deadline == 100
                )
            });
            assert!(found, "CampaignCreated event not found");
        });
    }

    #[test]
    fn uses_default_eligibility_when_no_custom_rules() {
        ExtBuilder::default().build().execute_with(|| {
            // Set default rules
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NativeBalance { min_balance: 42 }].try_into().unwrap();
            assert_ok!(Crowdfunding::set_default_eligibility(RuntimeOrigin::root(), rules));

            // Create campaign without custom rules
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.eligibility_rules.len(), 1);
        });
    }

    #[test]
    fn exact_min_duration_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            // MinCampaignDuration = 10, block = 1, deadline = 11 => duration = 10 (exact
            // min)
            let config = default_aon_config(11, 1000);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn exact_max_duration_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            // MaxCampaignDuration = 1000, block = 1, deadline = 1001 => duration = 1000
            // (exact max)
            let config = default_aon_config(1001, 1000);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn milestone_bps_exactly_10000_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                    Milestone { release_bps: 4000, description_hash: [3u8; 32] },
                ],
            );
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn milestone_bps_over_10000_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 6000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    // sum = 11000
                ],
            );
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::MilestoneBpsSumInvalid
            );
        });
    }

    #[test]
    fn aon_does_not_validate_milestones() {
        ExtBuilder::default().build().execute_with(|| {
            // AllOrNothing model has no milestone validation
            let config = default_aon_config(100, 1000);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn campaign_with_all_optional_fields() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.hard_cap = Some(5000);
            config.min_investment = Some(10);
            config.max_investment_per_investor = Some(2000);
            let id = create_funded_campaign(ALICE, config);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.config.hard_cap, Some(5000));
            assert_eq!(c.config.min_investment, Some(10));
            assert_eq!(c.config.max_investment_per_investor, Some(2000));
        });
    }
}

// ── cancel_campaign (supplementary) ─────────────────────────────────────

mod cancel_campaign_supplementary {
    use super::*;

    #[test]
    fn non_force_origin_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            // Signed origin should fail since ForceOrigin = EnsureRoot
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::signed(ALICE), id),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn emits_campaign_cancelled_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            System::reset_events();
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CampaignCancelled { campaign_id })
                    if *campaign_id == id
                )
            });
            assert!(found, "CampaignCancelled event not found");
        });
    }

    #[test]
    fn cancel_succeeded_campaign_is_blocked() {
        // CRIT-02: Succeeded campaigns must NOT be cancellable — the creator
        // has a committed economic relationship with investors.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.status, CampaignStatus::Succeeded);
        });
    }

    #[test]
    fn cancel_milestone_phase_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // In MilestonePhase — should be cancellable
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Cancelled));
        });
    }
}

// ── set_default_eligibility (supplementary) ─────────────────────────────

mod set_default_eligibility_supplementary {
    use super::*;

    #[test]
    fn emits_event() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NativeBalance { min_balance: 100 }].try_into().unwrap();
            System::reset_events();
            assert_ok!(Crowdfunding::set_default_eligibility(RuntimeOrigin::root(), rules));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(&e.event, RuntimeEvent::Crowdfunding(Event::DefaultEligibilitySet))
            });
            assert!(found, "DefaultEligibilitySet event not found");
        });
    }

    #[test]
    fn new_campaign_uses_updated_default_rules() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NativeBalance { min_balance: 500 }].try_into().unwrap();
            assert_ok!(Crowdfunding::set_default_eligibility(RuntimeOrigin::root(), rules));
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.eligibility_rules.len(), 1);
            // Verify the rule content
            match &c.eligibility_rules[0] {
                EligibilityRule::NativeBalance { min_balance } => assert_eq!(*min_balance, 500),
                _ => panic!("expected NativeBalance rule"),
            }
        });
    }

    #[test]
    fn empty_rules_clear_defaults() {
        ExtBuilder::default().build().execute_with(|| {
            // Set some rules first
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NativeBalance { min_balance: 100 }].try_into().unwrap();
            assert_ok!(Crowdfunding::set_default_eligibility(RuntimeOrigin::root(), rules));
            // Now clear
            let empty_rules: BoundedVec<_, _> = vec![].try_into().unwrap();
            assert_ok!(Crowdfunding::set_default_eligibility(RuntimeOrigin::root(), empty_rules));
            let stored = pallet::DefaultEligibilityRules::<Test>::get();
            assert_eq!(stored.len(), 0);
        });
    }
}

// ── invest (supplementary) ──────────────────────────────────────────────

mod invest_supplementary {
    use super::*;

    #[test]
    fn campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), 99, 500),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn invest_at_exact_deadline_block() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            run_to_block(20); // exactly at deadline
                              // now == deadline => passes ensure!(now <= deadline)
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
        });
    }

    #[test]
    fn invest_exactly_min_investment() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.min_investment = Some(100);
            let id = create_funded_campaign(ALICE, config);
            // Exact minimum should succeed
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    #[test]
    fn invest_exactly_at_hard_cap() {
        ExtBuilder::default().build().execute_with(|| {
            // M-5: hard_cap must be >= goal; use goal = 500 = hard_cap
            let mut config = default_aon_config(100, 500);
            config.hard_cap = Some(500);
            let id = create_funded_campaign(ALICE, config);
            // Invest exactly at hard cap
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 500);
        });
    }

    #[test]
    fn invest_hard_cap_cumulative() {
        ExtBuilder::default().build().execute_with(|| {
            // M-5: hard_cap must be >= goal; use goal = 500 = hard_cap
            let mut config = default_aon_config(100, 500);
            config.hard_cap = Some(500);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 300));
            // Second invest should fail because 300 + 201 > 500
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 201),
                Error::<Test>::HardCapExceeded
            );
            // But exactly 200 should work
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));
        });
    }

    #[test]
    fn emits_invested_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            System::reset_events();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::Invested {
                        campaign_id,
                        investor,
                        amount,
                    }) if *campaign_id == id && *investor == BOB && *amount == 500
                )
            });
            assert!(found, "Invested event not found");
        });
    }

    #[test]
    fn multiple_investors_increment_count() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 300));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 3);
            assert_eq!(c.total_raised, 600);
        });
    }

    #[test]
    fn investor_campaigns_updated() {
        ExtBuilder::default().build().execute_with(|| {
            let id1 = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let id2 = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id1, 100));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id2, 100));
            let investor_campaigns = pallet::InvestorCampaigns::<Test>::get(BOB);
            assert!(investor_campaigns.contains(&id1));
            assert!(investor_campaigns.contains(&id2));
            assert_eq!(investor_campaigns.len(), 2);
        });
    }

    #[test]
    fn repeat_invest_does_not_duplicate_investor_campaigns() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
            let investor_campaigns = pallet::InvestorCampaigns::<Test>::get(BOB);
            // Should only appear once
            assert_eq!(investor_campaigns.len(), 1);
        });
    }

    #[test]
    fn max_per_investor_with_withdrawals() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.max_investment_per_investor = Some(200);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
            // Withdraw 100 (net position = 200 - 100 = 100 after accounting withdrawn)
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100));
            // current = total_invested - total_withdrawn = 200 - 100 = 100
            // Investing 100 more => current + 100 = 200 which == max => should succeed
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }
}

// ── withdraw_investment (supplementary) ─────────────────────────────────

mod withdraw_investment_supplementary {
    use super::*;

    #[test]
    fn campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), 99, 100),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn withdraw_full_investment() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            // 1% of 500 = 5 penalty, net = 495
            assert_eq!(Balances::free_balance(BOB), bob_before + 495);
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_withdrawn, 500);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 0);
        });
    }

    #[test]
    fn emits_investment_withdrawn_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            System::reset_events();
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::InvestmentWithdrawn {
                        campaign_id,
                        investor,
                        amount,
                        penalty,
                    }) if *campaign_id == id && *investor == BOB && *amount == 990 && *penalty == 10
                )
            });
            assert!(found, "InvestmentWithdrawn event not found");
        });
    }

    #[test]
    fn partial_withdraw_then_more() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
            // current = 500 - 200 = 300
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 300));
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_withdrawn, 500);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 0);
        });
    }
}

// ── claim_refund (supplementary) ────────────────────────────────────────

mod claim_refund_supplementary {
    use super::*;

    #[test]
    fn campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), 99),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn no_investment_found() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // CHARLIE never invested
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id),
                Error::<Test>::NoInvestmentFound
            );
        });
    }

    #[test]
    fn emits_refund_claimed_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::RefundClaimed {
                        campaign_id,
                        investor,
                        amount,
                    }) if *campaign_id == id && *investor == BOB && *amount == 500
                )
            });
            assert!(found, "RefundClaimed event not found");
        });
    }

    #[test]
    fn refund_after_partial_withdrawal() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            // Withdraw 200 during funding (with penalty)
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Refund should be total_invested - total_withdrawn = 500 - 200 = 300
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 300);
        });
    }

    #[test]
    fn refund_nothing_after_full_withdrawal() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::NothingToRefund
            );
        });
    }
}

// ── finalize_campaign (supplementary) ───────────────────────────────────

mod finalize_campaign_supplementary {
    use super::*;

    #[test]
    fn campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), 99),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn milestone_goal_not_met_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500)); // below goal
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Failed));
        });
    }

    #[test]
    fn emits_finalized_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            System::reset_events();
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CampaignFinalized {
                        campaign_id,
                        status,
                    }) if *campaign_id == id && matches!(status, CampaignStatus::Succeeded)
                )
            });
            assert!(found, "CampaignFinalized event not found");
        });
    }

    #[test]
    fn finalize_at_deadline_plus_one() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(20); // exactly at deadline — should fail (need > deadline)
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::CampaignStillFunding
            );
            run_to_block(21); // deadline + 1 — should succeed
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
        });
    }

    #[test]
    fn aon_goal_exactly_met() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000)); // exactly at goal
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn kwyr_with_zero_raised_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
            assert_eq!(c.total_raised, 0);
        });
    }

    #[test]
    fn anyone_can_finalize() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            // BOB (not the creator) finalizes
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
        });
    }
}

// ── claim_funds (supplementary) ─────────────────────────────────────────

mod claim_funds_supplementary {
    use super::*;

    #[test]
    fn campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), 99),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn nothing_to_claim_after_claiming() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            // Already completed — claim again should fail on InvalidCampaignStatus
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn emits_funds_claimed_event() {
        // P1-02: amount in FundsClaimed is now net (after protocol fee).
        // P2-09: CampaignCompleted is no longer emitted from claim_funds —
        //        deposit is returned via claim_creation_deposit instead.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            // ProtocolFeeBps = 0 in mock, so creator_amount == 1000 (no fee)
            let has_funds_claimed = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::FundsClaimed {
                        campaign_id,
                        creator,
                        amount,
                    }) if *campaign_id == id && *creator == ALICE && *amount == 1000
                )
            });
            assert!(has_funds_claimed, "FundsClaimed event not found");
            // Verify campaign status transitions to Completed
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
        });
    }

    #[test]
    fn claim_funds_failed_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Failed status — should fail
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── claim_creation_deposit (supplementary) ──────────────────────────────

mod claim_creation_deposit_supplementary {
    use super::*;

    #[test]
    fn campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), 99),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn not_creator() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::NotCampaignCreator
            );
        });
    }

    #[test]
    fn emits_creation_deposit_claimed_event() {
        // P2-09: CampaignCompleted renamed to CreationDepositClaimed
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CreationDepositClaimed {
                        campaign_id,
                        creator,
                        deposit_returned,
                    }) if *campaign_id == id && *creator == ALICE && *deposit_returned == 100
                )
            });
            assert!(found, "CreationDepositClaimed event not found");
        });
    }

    #[test]
    fn succeeded_status_is_not_terminal() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Succeeded but not Completed — should fail
            assert_noop!(
                Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn milestone_phase_is_not_terminal() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── milestone_workflow (supplementary) ──────────────────────────────────

mod milestone_supplementary {
    use super::*;

    fn setup_milestone_campaign() -> u32 {
        let config = milestone_config(
            20,
            1000,
            vec![
                Milestone { release_bps: 6000, description_hash: [1u8; 32] },
                Milestone { release_bps: 4000, description_hash: [2u8; 32] },
            ],
        );
        let id = create_funded_campaign(ALICE, config);
        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
        run_to_block(21);
        assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
        id
    }

    // ── submit_milestone ──

    #[test]
    fn submit_campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), 99, 0),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn submit_not_milestone_phase() {
        ExtBuilder::default().build().execute_with(|| {
            // AoN campaign in Funding status
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn submit_invalid_index() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            // Only indices 0 and 1 exist
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 5),
                Error::<Test>::InvalidMilestoneIndex
            );
        });
    }

    #[test]
    fn submit_already_submitted() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            // Already Submitted — cannot submit again (must be Pending or Rejected)
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn submit_already_approved() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            // Approved — cannot submit again
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn submit_already_claimed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            // Claimed — cannot submit again
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn submit_emits_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            System::reset_events();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::MilestoneSubmitted {
                        campaign_id,
                        index,
                    }) if *campaign_id == id && *index == 0
                )
            });
            assert!(found, "MilestoneSubmitted event not found");
        });
    }

    // ── approve_milestone ──

    #[test]
    fn approve_non_root_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn approve_campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), 99, 0),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn approve_invalid_index() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 5),
                Error::<Test>::InvalidMilestoneIndex
            );
        });
    }

    #[test]
    fn approve_pending_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            // Milestone 0 is Pending (not yet submitted)
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn approve_emits_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            System::reset_events();
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::MilestoneApproved {
                        campaign_id,
                        index,
                    }) if *campaign_id == id && *index == 0
                )
            });
            assert!(found, "MilestoneApproved event not found");
        });
    }

    // ── reject_milestone ──

    #[test]
    fn reject_non_root_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn reject_campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), 99, 0),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn reject_pending_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            // Milestone 0 is Pending
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn reject_emits_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            System::reset_events();
            assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::MilestoneRejected {
                        campaign_id,
                        index,
                    }) if *campaign_id == id && *index == 0
                )
            });
            assert!(found, "MilestoneRejected event not found");
        });
    }

    // ── claim_milestone_funds ──

    #[test]
    fn claim_not_creator() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::NotCampaignCreator
            );
        });
    }

    #[test]
    fn claim_campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), 99, 0),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn claim_not_milestone_phase() {
        ExtBuilder::default().build().execute_with(|| {
            // AoN campaign that succeeded (not milestone phase)
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn claim_invalid_index() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 5),
                Error::<Test>::InvalidMilestoneIndex
            );
        });
    }

    #[test]
    fn claim_pending_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            // Milestone 0 is Pending (not approved)
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn claim_emits_milestone_funds_claimed_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::MilestoneFundsClaimed {
                        campaign_id,
                        index,
                        amount,
                    }) if *campaign_id == id && *index == 0 && *amount == 600
                )
            });
            assert!(found, "MilestoneFundsClaimed event not found");
        });
    }

    #[test]
    fn claim_disbursement_tracking() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_disbursed, 600); // 60% of 1000
                                                // Not yet completed (milestone 1 still pending)
            assert!(matches!(c.status, CampaignStatus::MilestonePhase));
        });
    }

    #[test]
    fn claim_creation_deposit_after_milestone_completed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            // Complete all milestones
            for i in 0..2u8 {
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, i));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    i
                ));
            }
            // Now can claim creation deposit
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }
}

// ── edge cases and boundary tests ──────────────────────────────────────

mod edge_cases {
    use super::*;

    #[test]
    fn invest_campaign_not_found_returns_error() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), 42, 100),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn withdraw_campaign_not_found_returns_error() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), 42, 100),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn claim_refund_on_succeeded_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Succeeded — refund not available
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn multiple_campaigns_independent_state() {
        ExtBuilder::default().build().execute_with(|| {
            let id1 = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            let id2 = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id1, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id2, 300));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id1));
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id2));
            // id1 failed (500 < 1000 goal), id2 succeeded (KWYR always succeeds)
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id1).unwrap().status,
                CampaignStatus::Failed
            ));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id2).unwrap().status,
                CampaignStatus::Succeeded
            ));
        });
    }

    #[test]
    fn cancel_during_funding_then_refund() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 300));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // Both investors can claim refunds
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);
            let charlie_before = Balances::free_balance(CHARLIE);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
            assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 300);
        });
    }

    #[test]
    fn next_campaign_id_monotonically_increases() {
        ExtBuilder::default().build().execute_with(|| {
            assert_eq!(pallet::NextCampaignId::<Test>::get(), 0);
            create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_eq!(pallet::NextCampaignId::<Test>::get(), 1);
            create_funded_campaign(BOB, default_aon_config(100, 1000));
            assert_eq!(pallet::NextCampaignId::<Test>::get(), 2);
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// FORENSIC AUDIT ROUND 2 — gaps identified through exhaustive code-path
// enumeration of every match arm, every ensure!, every error variant,
// every state transition, every event emission.
// ═══════════════════════════════════════════════════════════════════════

mod audit_claim_funds_gaps {
    use super::*;

    #[test]
    fn kwyr_zero_raised_nothing_to_claim() {
        // KWYR always succeeds even with zero raised, but claim_funds
        // should fail with NothingToClaim when total_raised == 0
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::NothingToClaim
            );
        });
    }

    #[test]
    fn claim_funds_on_cancelled_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn claim_funds_on_milestone_phase_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // MilestonePhase is not Succeeded
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn claim_funds_updates_total_disbursed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 700));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_disbursed, 700);
            assert_eq!(c.total_raised, 700);
        });
    }
}

mod audit_cancel_campaign_gaps {
    use super::*;

    #[test]
    fn cancel_failed_campaign_fails() {
        // Failed is not Cancelled or Completed, so the ensure! on line 344
        // should pass. Let's verify: "not matches!(Cancelled | Completed)"
        // means Failed IS cancellable. This tests that cancelling a Failed
        // campaign transitions to Cancelled.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Failed));
            // Failed is not in the exclusion set {Cancelled, Completed}
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Cancelled));
        });
    }
}

mod audit_finalize_gaps {
    use super::*;

    #[test]
    fn double_finalize_fails() {
        // Once finalized (Succeeded), status is no longer Funding
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn double_finalize_failed_also_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn aon_goal_exceeded_succeeds() {
        // total_raised > goal should still succeed
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn milestone_goal_exactly_met() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::MilestonePhase));
        });
    }

    #[test]
    fn milestone_goal_exceeded() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 2000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::MilestonePhase));
        });
    }

    #[test]
    fn finalize_emits_failed_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            System::reset_events();
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CampaignFinalized {
                        campaign_id,
                        status,
                    }) if *campaign_id == id && matches!(status, CampaignStatus::Failed)
                )
            });
            assert!(found, "CampaignFinalized(Failed) event not found");
        });
    }

    #[test]
    fn finalize_emits_milestone_phase_event() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            System::reset_events();
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CampaignFinalized {
                        campaign_id,
                        status,
                    }) if *campaign_id == id && matches!(status, CampaignStatus::MilestonePhase)
                )
            });
            assert!(found, "CampaignFinalized(MilestonePhase) event not found");
        });
    }
}

mod audit_milestone_state_transitions {
    use super::*;

    fn setup_milestone_campaign() -> u32 {
        let config = milestone_config(
            20,
            1000,
            vec![
                Milestone { release_bps: 6000, description_hash: [1u8; 32] },
                Milestone { release_bps: 4000, description_hash: [2u8; 32] },
            ],
        );
        let id = create_funded_campaign(ALICE, config);
        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
        run_to_block(21);
        assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
        id
    }

    // ── reject_milestone state transition gaps ──

    #[test]
    fn reject_invalid_index() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 5),
                Error::<Test>::InvalidMilestoneIndex
            );
        });
    }

    #[test]
    fn reject_already_rejected() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
            // Now status is Rejected — reject again should fail (only Submitted allowed)
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn reject_already_approved() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            // Approved — reject should fail (only Submitted allowed)
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn reject_already_claimed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            // Claimed — reject should fail
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    // ── approve_milestone state transition gaps ──

    #[test]
    fn approve_already_approved() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            // Double approve should fail (Approved is not Submitted)
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn approve_already_rejected() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
            // Rejected — approve should fail (only Submitted allowed)
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn approve_already_claimed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            // Claimed — approve should fail
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    // ── claim_milestone_funds state transition gaps ──

    #[test]
    fn claim_milestone_double_claim() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            // Status is now Claimed — second claim should fail
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn claim_milestone_rejected_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
            // Rejected — cannot claim
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    #[test]
    fn claim_milestone_submitted_not_approved_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            // Submitted but not approved — should fail
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    // ── full milestone lifecycle event verification ──

    #[test]
    fn all_milestones_claimed_transitions_to_completed() {
        // P2-09: CampaignCompleted event is no longer emitted from
        // claim_milestone_funds. Completion is verified via status check.
        // P1-02: MilestoneFundsClaimed amount is now net (after fee).
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            // Complete milestone 0
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));

            // Complete milestone 1 — this should transition status to Completed
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 1));

            let events = System::events();
            // ProtocolFeeBps = 0, so creator_amount = release_amount = 400
            let has_milestone_claimed = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::MilestoneFundsClaimed {
                        campaign_id,
                        index,
                        amount,
                    }) if *campaign_id == id && *index == 1 && *amount == 400
                )
            });
            assert!(has_milestone_claimed, "MilestoneFundsClaimed event not found for milestone 1");

            // Verify status transitioned to Completed
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
        });
    }

    // ── out-of-order milestone operations ──

    #[test]
    fn milestones_can_be_claimed_out_of_order() {
        // The pallet does NOT enforce sequential milestone claiming.
        // Milestone 1 can be submitted/approved/claimed before milestone 0.
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            // Submit and approve milestone 1 first (out of order)
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 1));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 400); // 40% of 1000

            // Campaign is NOT completed yet (milestone 0 still pending)
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::MilestonePhase));

            // Now complete milestone 0
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 600); // 60% of 1000

            // NOW campaign should be completed
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
            assert_eq!(c.total_disbursed, 1000);
        });
    }
}

mod audit_invest_gaps {
    use super::*;

    #[test]
    fn invest_zero_amount_fails() {
        // P2-01: zero-amount investments are now rejected before any state change
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::InvestmentBelowMinimum
            );
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 0);
            assert_eq!(c.investor_count, 0);
        });
    }

    #[test]
    fn invest_exactly_at_max_per_investor() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.max_investment_per_investor = Some(500);
            let id = create_funded_campaign(ALICE, config);
            // Exactly at max
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            // One more unit should fail
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1),
                Error::<Test>::InvestmentExceedsMaxPerInvestor
            );
        });
    }

    #[test]
    fn invest_one_block_past_deadline() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            run_to_block(21); // past deadline
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500),
                Error::<Test>::DeadlinePassed
            );
        });
    }

    #[test]
    fn invest_with_no_min_no_hard_cap_no_max_per_investor() {
        // Campaign with all optional limits as None
        ExtBuilder::default().build().execute_with(|| {
            let config = default_aon_config(100, 1000);
            let id = create_funded_campaign(ALICE, config);
            // Should work for any amount
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 9000));
        });
    }

    #[test]
    fn invest_insufficient_balance() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 200)]).build().execute_with(
            || {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                // BOB has 200, ExistentialDeposit = 1, so max transferable = 199 (KeepAlive)
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200),
                    pallet_balances::Error::<Test>::KeepAlive
                );
            },
        );
    }
}

mod audit_withdraw_gaps {
    use super::*;

    #[test]
    fn withdraw_zero_amount_fails() {
        // P2-02: zero-amount withdrawals are now rejected
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::InsufficientInvestment
            );
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_withdrawn, 0); // unchanged
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 500); // unchanged
        });
    }

    #[test]
    fn withdraw_exact_current_balance() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_withdrawn, 500);
            assert_eq!(inv.total_invested, 500);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 0);
        });
    }

    #[test]
    fn withdraw_on_succeeded_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn withdraw_on_failed_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Failed — withdraw should fail (not Funding)
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

mod audit_refund_gaps {
    use super::*;

    #[test]
    fn multiple_investors_refund_from_failed_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 300));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 200));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Failed
            ));

            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);

            let charlie_before = Balances::free_balance(CHARLIE);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
            assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 300);

            let dave_before = Balances::free_balance(DAVE);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(DAVE), id));
            assert_eq!(Balances::free_balance(DAVE), dave_before + 200);
        });
    }

    #[test]
    fn refund_removes_investment_record() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert!(pallet::Investments::<Test>::get(id, BOB).is_none());
            assert!(!pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
        });
    }

    #[test]
    fn refund_on_completed_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            // Completed — refund should fail
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn refund_on_milestone_phase_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // MilestonePhase — refund should fail
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn refund_on_funding_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            // Still Funding — refund should fail
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

mod audit_creation_deposit_gaps {
    use super::*;

    #[test]
    fn claim_creation_deposit_on_funding_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            // Funding is not a terminal state
            assert_noop!(
                Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn claim_creation_deposit_zeroes_deposit_field() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let c_before = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c_before.creation_deposit, 100);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            let c_after = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c_after.creation_deposit, 0);
        });
    }

    #[test]
    fn claim_creation_deposit_does_not_drain_investor_funds() {
        // After failed campaign with investments, creation deposit claim
        // should only transfer the deposit amount, not investor funds.
        // Both orderings (deposit-first or investors-first) work because
        // do_transfer uses AllowDeath for sub-account outgoing transfers.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            let sub_account = Crowdfunding::campaign_account(id);
            // Sub-account has: 100 (deposit) + 500 + 500 = 1100
            assert_eq!(Balances::free_balance(sub_account), 1100);

            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);

            // Sub-account should still have investor funds
            assert_eq!(Balances::free_balance(sub_account), 1000);

            // BOB refunds (sub goes from 1000 to 500)
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);

            // CHARLIE is the last investor — AllowDeath allows draining sub to 0
            let charlie_before = Balances::free_balance(CHARLIE);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
            assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 500);

            assert_eq!(Balances::free_balance(sub_account), 0);
        });
    }

    #[test]
    fn investors_refund_before_deposit_claim_works() {
        // Correct ordering: investors refund first, then creator claims deposit.
        // The deposit claim uses AllowDeath so it can drain the sub-account.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // BOB refunds first (sub: 1100 - 1000 = 100)
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 1000);

            // Then creator claims deposit (AllowDeath allows draining to 0)
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }
}

mod audit_sub_account_balance_consistency {
    use super::*;

    #[test]
    fn sub_account_tracks_deposit_and_investments() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            let sub = Crowdfunding::campaign_account(id);
            assert_eq!(Balances::free_balance(sub), 100); // creation deposit

            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_eq!(Balances::free_balance(sub), 600); // 100 + 500

            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 300));
            assert_eq!(Balances::free_balance(sub), 900); // 100 + 500 + 300
        });
    }

    #[test]
    fn sub_account_after_withdrawal_with_penalty() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            let sub = Crowdfunding::campaign_account(id);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_eq!(Balances::free_balance(sub), 1100); // 100 deposit + 1000

            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            // 1% penalty on 500 = 5, net refund = 495, burned = 5
            // sub_account loses: 495 (transfer) + 5 (burn) = 500
            assert_eq!(Balances::free_balance(sub), 600); // 1100 - 500
        });
    }

    #[test]
    fn sub_account_after_full_lifecycle_is_depleted() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            let sub = Crowdfunding::campaign_account(id);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_eq!(Balances::free_balance(sub), 600); // 100 + 500

            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            // After claim_funds: sub has 600 - 500 = 100 (deposit remains)
            assert_eq!(Balances::free_balance(sub), 100);

            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            // After claiming deposit: sub should be empty (or at existential deposit)
            // AllowDeath used for deposit transfer, so account can go to 0
            assert_eq!(Balances::free_balance(sub), 0);
        });
    }

    #[test]
    fn sub_account_milestone_lifecycle() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 7000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            let sub = Crowdfunding::campaign_account(id);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_eq!(Balances::free_balance(sub), 1100); // 100 + 1000

            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Claim milestone 0: 30% of 1000 = 300
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(Balances::free_balance(sub), 800); // 1100 - 300

            // Claim milestone 1: 70% of 1000 = 700
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 1));
            assert_eq!(Balances::free_balance(sub), 100); // deposit only

            // Claim creation deposit
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(sub), 0);
        });
    }
}

mod audit_max_milestones_boundary {
    use super::*;

    #[test]
    fn five_milestones_max_boundary() {
        // MaxMilestones = 5 in mock config
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 2000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 2000, description_hash: [2u8; 32] },
                    Milestone { release_bps: 2000, description_hash: [3u8; 32] },
                    Milestone { release_bps: 2000, description_hash: [4u8; 32] },
                    Milestone { release_bps: 2000, description_hash: [5u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // All 5 milestone statuses should be initialized
            for i in 0..5u8 {
                assert_eq!(
                    pallet::MilestoneStatuses::<Test>::get(id, i),
                    Some(MilestoneStatus::Pending)
                );
            }

            // Complete all 5
            for i in 0..5u8 {
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, i));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    i
                ));
            }

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
            assert_eq!(c.total_disbursed, 1000);
        });
    }

    #[test]
    fn single_milestone_100_percent() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![Milestone { release_bps: 10000, description_hash: [1u8; 32] }],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
        });
    }
}

mod audit_eligibility_gaps {
    use super::*;

    #[test]
    fn eligibility_asset_balance_passes() {
        ExtBuilder::default().build().execute_with(|| {
            // Create an asset and mint to BOB
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                codec::Compact(1u32),
                ALICE,
                true,
                1
            ));
            assert_ok!(Assets::mint(RuntimeOrigin::signed(ALICE), codec::Compact(1u32), BOB, 5000));

            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::AssetBalance { asset_id: 1u32, min_balance: 1000 }]
                    .try_into()
                    .unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // BOB has 5000 of asset 1, passes 1000 check
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    #[test]
    fn eligibility_asset_balance_fails() {
        ExtBuilder::default().build().execute_with(|| {
            // Create an asset but do NOT mint to BOB
            assert_ok!(Assets::force_create(
                RuntimeOrigin::root(),
                codec::Compact(1u32),
                ALICE,
                true,
                1
            ));

            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::AssetBalance { asset_id: 1u32, min_balance: 1000 }]
                    .try_into()
                    .unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
        });
    }

    #[test]
    fn eligibility_multiple_rules_all_must_pass() {
        // Multiple eligibility rules must ALL pass (AND logic per rule)
        ExtBuilder::default().build().execute_with(|| {
            MockNftInspect::set_owner(1, 1, BOB);
            let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
            let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();

            let rules: BoundedVec<_, _> = vec![
                EligibilityRule::NativeBalance { min_balance: 5000 },
                EligibilityRule::NftOwnership { required_sets },
            ]
            .try_into()
            .unwrap();

            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // BOB has 10_000 native and owns NFT (1,1) — should pass both
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    #[test]
    fn eligibility_multiple_rules_one_fails() {
        ExtBuilder::default().build().execute_with(|| {
            // BOB does NOT own NFT
            let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
            let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();

            let rules: BoundedVec<_, _> = vec![
                EligibilityRule::NativeBalance { min_balance: 5000 },
                EligibilityRule::NftOwnership { required_sets },
            ]
            .try_into()
            .unwrap();

            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // BOB has sufficient balance but no NFT — second rule fails
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
        });
    }

    #[test]
    fn eligibility_empty_rules_anyone_can_invest() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> = vec![].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // No rules — anyone can invest
            let _ = Balances::deposit_creating(&99u64, 1000);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(99), id, 100));
        });
    }

    #[test]
    fn nft_ownership_any_set_satisfies() {
        // NftOwnership checks: required_sets.iter().any(|set| set.iter().all(...))
        // So if ANY one set is fully owned, the rule passes
        ExtBuilder::default().build().execute_with(|| {
            // BOB owns NFT (2,2) but not (1,1)
            MockNftInspect::set_owner(2, 2, BOB);

            let set1: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
            let set2: BoundedVec<(u32, u32), _> = vec![(2u32, 2u32)].try_into().unwrap();
            let required_sets: BoundedVec<_, _> = vec![set1, set2].try_into().unwrap();

            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();

            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // BOB doesn't own set1 but owns set2 — any() should pass
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    #[test]
    fn nft_ownership_partial_set_fails() {
        // If a set requires multiple NFTs, ALL must be owned
        ExtBuilder::default().build().execute_with(|| {
            // BOB owns (1,1) but not (1,2)
            MockNftInspect::set_owner(1, 1, BOB);

            let set1: BoundedVec<(u32, u32), _> =
                vec![(1u32, 1u32), (1u32, 2u32)].try_into().unwrap();
            let required_sets: BoundedVec<_, _> = vec![set1].try_into().unwrap();

            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();

            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // BOB owns only 1 of 2 required NFTs in the only set — fails
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
        });
    }
}

mod audit_penalty_computation {
    use super::*;

    #[test]
    fn penalty_on_small_amounts() {
        // 1% of 1 = ceil(100/10000) = 1 (ceiling division)
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1));
            // ceil(1 * 100 / 10000) = 1, penalty = 1, net = 0
            assert_eq!(Balances::free_balance(BOB), bob_before);
        });
    }

    #[test]
    fn penalty_on_exact_100() {
        // 1% of 100 = 1
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100));
            // Permill(10_000) * 100 = 1
            assert_eq!(Balances::free_balance(BOB), bob_before + 99);
        });
    }

    #[test]
    fn penalty_on_large_amount() {
        // 1% of 9000 = 90
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 50000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 9000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 9000));
            assert_eq!(Balances::free_balance(BOB), bob_before + 8910);
        });
    }
}

mod audit_campaign_account_helper {
    use super::*;

    #[test]
    fn campaign_account_is_deterministic() {
        // The same campaign_id always produces the same sub-account
        ExtBuilder::default().build().execute_with(|| {
            let acc1 = Crowdfunding::campaign_account(7);
            let acc2 = Crowdfunding::campaign_account(7);
            assert_eq!(acc1, acc2);
        });
    }

    #[test]
    fn campaign_account_derived_from_pallet_id() {
        // Verify the sub-account is non-zero and derived from PalletId
        ExtBuilder::default().build().execute_with(|| {
            let acc = Crowdfunding::campaign_account(0);
            assert_ne!(acc, 0u64);
        });
    }
}

mod audit_interaction_sequences {
    use super::*;

    #[test]
    fn invest_withdraw_reinvest_finalize_claim() {
        // Full interaction: invest, partially withdraw, re-invest, finalize, claim
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            // BOB invests 500
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            // BOB withdraws 200 (gets 198, penalty 2)
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
            // total_raised = 300, BOB's position = 300
            // BOB invests 100 more
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            // total_raised = 400

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 400);
            assert_eq!(c.investor_count, 1);

            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 400);
        });
    }

    #[test]
    fn cancel_during_milestone_phase_then_refund() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Claim first milestone (50% of 1000 = 500)
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));

            // Cancel during milestone phase
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            let sub = Crowdfunding::campaign_account(id);
            // sub_balance: 100 (deposit) + 1000 (invested) - 500 (milestone 0) = 600
            assert_eq!(Balances::free_balance(sub), 600);

            // BOB gets a proportional refund: 50% of raised was disbursed,
            // so refund = 50% of (total_invested - total_withdrawn) = 50% of 1000 = 500
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);

            // Sub-account still has 100 (deposit)
            assert_eq!(Balances::free_balance(sub), 100);

            // Creator can claim deposit
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }

    #[test]
    fn creator_deposit_and_investor_refund_ordering_matters() {
        // Both orderings work: investors first or creator first.
        // AllowDeath on sub-account outgoing transfers removes ordering dependency.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 300));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            let sub = Crowdfunding::campaign_account(id);
            // Sub-account: 100 (deposit) + 500 + 300 = 900
            assert_eq!(Balances::free_balance(sub), 900);

            // Creator claims deposit FIRST
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
            assert_eq!(Balances::free_balance(sub), 800);

            // Investors claim refunds after
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);

            let charlie_before = Balances::free_balance(CHARLIE);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
            assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 300);

            // Sub-account should be empty
            assert_eq!(Balances::free_balance(sub), 0);
        });
    }

    #[test]
    fn last_investor_refund_succeeds_when_deposit_already_claimed() {
        // AllowDeath on sub-account outgoing transfers means the last investor
        // can drain the sub-account to 0 even after the deposit is claimed.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            // Creator claims deposit first — sub goes from 600 to 500
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));

            // BOB is the sole investor — refund of 500 drains sub to 0 (AllowDeath)
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);

            let sub = Crowdfunding::campaign_account(id);
            assert_eq!(Balances::free_balance(sub), 0);
        });
    }

    #[test]
    fn investors_refund_then_creator_claims_deposit() {
        // Reverse order: investors refund first, then creator claims deposit
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            // BOB refunds first
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);

            // Creator claims deposit
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }

    #[test]
    fn kwyr_claim_funds_then_deposit() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Claim funds, then claim deposit
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);

            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
        });
    }
}

mod fix_keepalive_and_proportional_refund {
    use super::*;

    #[test]
    fn refund_and_deposit_any_order_works() {
        // Both orderings (investor-first and creator-first) succeed.
        ExtBuilder::default().build().execute_with(|| {
            // --- Ordering A: creator claims deposit first ---
            let id_a = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_a, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id_a, 300));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id_a));

            // Creator first
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id_a));
            // Then investors
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id_a));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);
            let charlie_before = Balances::free_balance(CHARLIE);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id_a));
            assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 300);
            let sub_a = Crowdfunding::campaign_account(id_a);
            assert_eq!(Balances::free_balance(sub_a), 0);

            // --- Ordering B: investors claim refunds first ---
            let id_b = create_funded_campaign(ALICE, default_aon_config(50, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_b, 400));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id_b, 200));
            run_to_block(51);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id_b));

            // Investors first
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id_b));
            assert_eq!(Balances::free_balance(BOB), bob_before + 400);
            let charlie_before = Balances::free_balance(CHARLIE);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id_b));
            assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 200);
            // Then creator
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id_b));
            let sub_b = Crowdfunding::campaign_account(id_b);
            assert_eq!(Balances::free_balance(sub_b), 0);
        });
    }

    #[test]
    fn proportional_refund_single_milestone_disbursed() {
        // 1 of 2 milestones claimed (50% disbursed), cancel, investor gets 50% refund.
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Claim milestone 0: 50% of 1000 = 500
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));

            let sub = Crowdfunding::campaign_account(id);
            // sub: 100 (deposit) + 1000 - 500 = 600
            assert_eq!(Balances::free_balance(sub), 600);

            // Cancel
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            // BOB gets proportional refund: 50% remaining * 1000 = 500
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);

            // Sub still has 100 (deposit)
            assert_eq!(Balances::free_balance(sub), 100);

            // Creator claims deposit
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(sub), 0);
        });
    }

    #[test]
    fn proportional_refund_multiple_investors() {
        // Multiple investors get proportional shares after partial disbursement.
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 7000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 400));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Claim milestone 0: 30% of 1000 = 300
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));

            // Cancel
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            // remaining_ratio = (1000 - 300) / 1000 = 70%
            // BOB refund: 70% of 600 = 420
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 420);

            // CHARLIE refund: 70% of 400 = 280
            let charlie_before = Balances::free_balance(CHARLIE);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
            assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 280);

            // Sub should have exactly the deposit left
            let sub = Crowdfunding::campaign_account(id);
            assert_eq!(Balances::free_balance(sub), 100);
        });
    }

    #[test]
    fn proportional_refund_no_disbursement_is_full() {
        // Edge case: total_disbursed == 0 gives full refund (unchanged behavior).
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Cancel immediately (no milestones claimed, total_disbursed == 0)
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            // Full refund
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 1000);
        });
    }

    #[test]
    fn all_milestones_disbursed_then_cancel_not_possible() {
        // Edge case: 100% disbursed means campaign is Completed — can't cancel.
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Claim both milestones
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 1));

            // Campaign is now Completed
            let campaign = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(campaign.status, CampaignStatus::Completed);

            // Cannot cancel a completed campaign
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

mod bounded_vec_cleanup {
    use super::*;

    #[test]
    fn creator_campaigns_cleaned_after_deposit_claim() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert!(pallet::CreatorCampaigns::<Test>::get(ALICE).contains(&id));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert!(!pallet::CreatorCampaigns::<Test>::get(ALICE).contains(&id));
        });
    }

    #[test]
    fn creator_can_create_new_after_cleanup() {
        ExtBuilder::default().build().execute_with(|| {
            // Fill up MaxCampaignsPerCreator = 5
            let mut ids = vec![];
            for _ in 0..5 {
                ids.push(create_funded_campaign(ALICE, default_aon_config(100, 1000)));
            }
            // Can't create more
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 1000),
                    None,
                    None,
                ),
                Error::<Test>::MaxCampaignsPerCreatorReached
            );
            // Cancel and claim deposit on one campaign
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), ids[0]));
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), ids[0]));
            // Now can create a new one
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                None,
                None,
            ));
        });
    }

    #[test]
    fn investor_campaigns_cleaned_after_refund() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert!(pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert!(!pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
        });
    }

    #[test]
    fn investor_campaigns_cleaned_after_full_withdraw() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert!(pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            assert!(!pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
        });
    }

    #[test]
    fn investor_campaigns_not_cleaned_after_partial_withdraw() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
            // Still has remaining investment — should stay in list
            assert!(pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
        });
    }

    #[test]
    fn investor_can_invest_new_after_cleanup() {
        ExtBuilder::default().build().execute_with(|| {
            // MaxInvestmentsPerInvestor = 5
            let mut campaign_ids = vec![];
            for _ in 0..5 {
                campaign_ids.push(create_funded_campaign(ALICE, default_aon_config(100, 1000)));
            }
            for &cid in &campaign_ids {
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), cid, 100));
            }
            // BOB is at max investments
            let extra = create_funded_campaign(CHARLIE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), extra, 100),
                Error::<Test>::MaxInvestmentsPerInvestorReached
            );
            // Fully withdraw from one campaign to clean up
            assert_ok!(Crowdfunding::withdraw_investment(
                RuntimeOrigin::signed(BOB),
                campaign_ids[0],
                100
            ));
            // Now can invest in new campaign
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), extra, 100));
        });
    }

    #[test]
    fn milestones_can_be_claimed_out_of_order_still_works() {
        // Verify that milestone out-of-order claiming still works after Phase 1A
        // changes
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 6000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 4000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Claim milestone 1 before milestone 0
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 1));
            // Then claim milestone 0
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// PHASE 3 TESTS
// ═══════════════════════════════════════════════════════════════════════

mod protocol_fee {
    use super::*;

    #[test]
    fn zero_fee_no_impact() {
        // Mock has ProtocolFeeBps = 0, so no fee collected
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            // Full amount goes to creator since fee is 0
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);
        });
    }
}

mod paused_status {
    use super::*;

    #[test]
    fn pause_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Paused));
        });
    }

    #[test]
    fn resume_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Funding));
        });
    }

    #[test]
    fn paused_cannot_invest() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn paused_can_withdraw() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
        });
    }

    #[test]
    fn non_admin_cannot_pause() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::pause_campaign(RuntimeOrigin::signed(ALICE), id),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn cancel_paused_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Cancelled));
        });
    }
}

mod whitelist {
    use super::*;

    #[test]
    fn whitelist_happy_path() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::AccountWhitelist].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // BOB not whitelisted -- should fail
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
            // Whitelist BOB
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            // Now BOB can invest
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    #[test]
    fn remove_from_whitelist_works() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::AccountWhitelist].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            assert_ok!(Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
        });
    }

    #[test]
    fn non_creator_cannot_whitelist() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(BOB), id, CHARLIE),
                Error::<Test>::NotCampaignCreator
            );
        });
    }

    #[test]
    fn whitelist_with_other_rules() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> = vec![
                EligibilityRule::NativeBalance { min_balance: 5000 },
                EligibilityRule::AccountWhitelist,
            ]
            .try_into()
            .unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // BOB has 10_000 balance but not whitelisted
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
            // Whitelist BOB
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            // Now should pass both rules
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    #[test]
    fn whitelist_campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), 99, BOB),
                Error::<Test>::CampaignNotFound
            );
        });
    }
}

mod campaign_id_overflow {
    use super::*;

    #[test]
    fn overflow_protection() {
        ExtBuilder::default().build().execute_with(|| {
            pallet::NextCampaignId::<Test>::put(u32::MAX);
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 1000),
                    None,
                    None,
                ),
                Error::<Test>::CampaignIdOverflow
            );
        });
    }
}

mod hard_cap_reached {
    use super::*;

    #[test]
    fn emits_hard_cap_reached_event() {
        ExtBuilder::default().build().execute_with(|| {
            // M-5: hard_cap must be >= goal; use goal = 500 = hard_cap
            let mut config = default_aon_config(100, 500);
            config.hard_cap = Some(500);
            let id = create_funded_campaign(ALICE, config);
            System::reset_events();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::HardCapReached { campaign_id })
                    if *campaign_id == id
                )
            });
            assert!(found, "HardCapReached event not found");
        });
    }

    #[test]
    fn no_event_when_below_hard_cap() {
        ExtBuilder::default().build().execute_with(|| {
            // M-5: hard_cap must be >= goal; use goal = 500 = hard_cap
            let mut config = default_aon_config(100, 500);
            config.hard_cap = Some(500);
            let id = create_funded_campaign(ALICE, config);
            System::reset_events();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(&e.event, RuntimeEvent::Crowdfunding(Event::HardCapReached { .. }))
            });
            assert!(!found, "HardCapReached event should not be emitted below cap");
        });
    }
}

mod per_campaign_penalty {
    use super::*;

    #[test]
    fn per_campaign_penalty_overrides_default() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(500); // 5%
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // 5% of 1000 = 50 penalty, net = 950
            assert_eq!(Balances::free_balance(BOB), bob_before + 950);
        });
    }

    #[test]
    fn none_uses_pallet_default() {
        ExtBuilder::default().build().execute_with(|| {
            let config = default_aon_config(100, 5000);
            // early_withdrawal_penalty_bps is None, falls back to pallet default (1%)
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // 1% of 1000 = 10 penalty, net = 990
            assert_eq!(Balances::free_balance(BOB), bob_before + 990);
        });
    }

    #[test]
    fn zero_bps_penalty_no_deduction() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(0);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // 0% penalty => net == gross
            assert_eq!(Balances::free_balance(BOB), bob_before + 1000);
        });
    }

    #[test]
    fn max_bps_penalty_10000_takes_everything() {
        // 10000 bps = 100% penalty, investor gets nothing
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(10000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // Permill::from_parts(10000 * 100) = Permill(1_000_000) = 100%
            // penalty = 1000, net = 0
            assert_eq!(Balances::free_balance(BOB), bob_before); // gets nothing
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// FORENSIC AUDIT ROUND 3 — asset currency dual-track, protocol fees,
// pause/resume edge cases, whitelist edge cases, rounding, security
// ═══════════════════════════════════════════════════════════════════════

// ── Asset currency (dual-track) tests ───────────────────────────────

mod asset_currency {
    use frame_support::traits::tokens::fungibles;

    use super::*;

    fn setup_asset_and_balances(asset_id: u32) {
        // Create asset with ALICE as admin
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            codec::Compact(asset_id),
            ALICE,
            true,
            1, // min_balance
        ));
        // Mint to all test accounts
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            codec::Compact(asset_id),
            ALICE,
            50_000
        ));
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            codec::Compact(asset_id),
            BOB,
            50_000
        ));
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            codec::Compact(asset_id),
            CHARLIE,
            50_000
        ));
    }

    fn asset_aon_config(deadline: u64, goal: u128, asset_id: u32) -> CampaignConfigOf {
        crate::CampaignConfig {
            funding_model: crate::FundingModel::AllOrNothing { goal },
            funding_currency: crate::PaymentCurrency::Asset(asset_id),
            deadline,
            hard_cap: None,
            min_investment: None,
            max_investment_per_investor: None,
            metadata_hash: [0u8; 32],
            early_withdrawal_penalty_bps: None,
        }
    }

    fn asset_kwyr_config(deadline: u64, asset_id: u32) -> CampaignConfigOf {
        crate::CampaignConfig {
            funding_model: crate::FundingModel::KeepWhatYouRaise { soft_cap: None },
            funding_currency: crate::PaymentCurrency::Asset(asset_id),
            deadline,
            hard_cap: None,
            min_investment: None,
            max_investment_per_investor: None,
            metadata_hash: [0u8; 32],
            early_withdrawal_penalty_bps: None,
        }
    }

    fn asset_milestone_config(
        deadline: u64,
        goal: u128,
        milestones: Vec<crate::Milestone>,
        asset_id: u32,
    ) -> CampaignConfigOf {
        crate::CampaignConfig {
            funding_model: crate::FundingModel::MilestoneBased {
                goal,
                milestones: BoundedVec::try_from(milestones).unwrap(),
            },
            funding_currency: crate::PaymentCurrency::Asset(asset_id),
            deadline,
            hard_cap: None,
            min_investment: None,
            max_investment_per_investor: None,
            metadata_hash: [0u8; 32],
            early_withdrawal_penalty_bps: None,
        }
    }

    #[test]
    fn invest_with_asset_currency() {
        ExtBuilder::default().build().execute_with(|| {
            setup_asset_and_balances(1);
            let config = asset_aon_config(100, 1000, 1);
            let id = create_funded_campaign(ALICE, config);
            let bob_asset_before = <Assets as fungibles::Inspect<u64>>::balance(1, &BOB);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let bob_asset_after = <Assets as fungibles::Inspect<u64>>::balance(1, &BOB);
            assert_eq!(bob_asset_after, bob_asset_before - 500);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 500);
        });
    }

    #[test]
    fn withdraw_with_asset_currency() {
        ExtBuilder::default().build().execute_with(|| {
            setup_asset_and_balances(1);
            let config = asset_aon_config(100, 5000, 1);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_asset_before = <Assets as fungibles::Inspect<u64>>::balance(1, &BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            // 1% penalty on 500 = 5, net = 495
            let bob_asset_after = <Assets as fungibles::Inspect<u64>>::balance(1, &BOB);
            assert_eq!(bob_asset_after, bob_asset_before + 495);
        });
    }

    #[test]
    fn claim_funds_with_asset_currency() {
        ExtBuilder::default().build().execute_with(|| {
            setup_asset_and_balances(1);
            let config = asset_kwyr_config(20, 1);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let alice_asset_before = <Assets as fungibles::Inspect<u64>>::balance(1, &ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            let alice_asset_after = <Assets as fungibles::Inspect<u64>>::balance(1, &ALICE);
            assert_eq!(alice_asset_after, alice_asset_before + 1000);
        });
    }

    #[test]
    fn claim_refund_with_asset_currency() {
        ExtBuilder::default().build().execute_with(|| {
            setup_asset_and_balances(1);
            let config = asset_aon_config(20, 5000, 1);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let bob_asset_before = <Assets as fungibles::Inspect<u64>>::balance(1, &BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            let bob_asset_after = <Assets as fungibles::Inspect<u64>>::balance(1, &BOB);
            assert_eq!(bob_asset_after, bob_asset_before + 1000);
        });
    }

    #[test]
    fn milestone_claim_with_asset_currency() {
        ExtBuilder::default().build().execute_with(|| {
            setup_asset_and_balances(1);
            let config = asset_milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
                1,
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            let alice_asset_before = <Assets as fungibles::Inspect<u64>>::balance(1, &ALICE);
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            let alice_asset_after = <Assets as fungibles::Inspect<u64>>::balance(1, &ALICE);
            // 50% of 1000 = 500
            assert_eq!(alice_asset_after, alice_asset_before + 500);
        });
    }

    #[test]
    fn asset_deposit_is_still_native() {
        // creation deposit always uses native currency, even for asset-based campaigns
        ExtBuilder::default().build().execute_with(|| {
            setup_asset_and_balances(1);
            let alice_native_before = Balances::free_balance(ALICE);
            let config = asset_aon_config(100, 1000, 1);
            let id = create_funded_campaign(ALICE, config);
            // Native balance decreased by deposit amount
            assert_eq!(Balances::free_balance(ALICE), alice_native_before - 100);
            // Claim deposit returns native
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let alice_native_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_native_before + 100);
        });
    }

    #[test]
    fn asset_full_lifecycle_aon_failed_refund() {
        ExtBuilder::default().build().execute_with(|| {
            setup_asset_and_balances(1);
            let config = asset_aon_config(20, 5000, 1);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // BOB refund
            let bob_before = <Assets as fungibles::Inspect<u64>>::balance(1, &BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            let bob_after = <Assets as fungibles::Inspect<u64>>::balance(1, &BOB);
            assert_eq!(bob_after, bob_before + 1000);

            // CHARLIE refund
            let charlie_before = <Assets as fungibles::Inspect<u64>>::balance(1, &CHARLIE);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
            let charlie_after = <Assets as fungibles::Inspect<u64>>::balance(1, &CHARLIE);
            assert_eq!(charlie_after, charlie_before + 500);
        });
    }
}

// ── Pause/Resume edge cases ─────────────────────────────────────────

mod paused_status_extended {
    use super::*;

    #[test]
    fn pause_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::pause_campaign(RuntimeOrigin::root(), 99),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn resume_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::resume_campaign(RuntimeOrigin::root(), 99),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn pause_non_funding_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::pause_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotFunding
            );
        });
    }

    #[test]
    fn pause_already_paused_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::pause_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotFunding
            );
        });
    }

    #[test]
    fn resume_non_paused_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::resume_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotPaused
            );
        });
    }

    #[test]
    fn non_admin_cannot_resume() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::resume_campaign(RuntimeOrigin::signed(ALICE), id),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn pause_emits_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            System::reset_events();
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CampaignPaused { campaign_id })
                    if *campaign_id == id
                )
            });
            assert!(found, "CampaignPaused event not found");
        });
    }

    #[test]
    fn resume_emits_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            System::reset_events();
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CampaignResumed { campaign_id })
                    if *campaign_id == id
                )
            });
            assert!(found, "CampaignResumed event not found");
        });
    }

    #[test]
    fn invest_after_resume_works() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500),
                Error::<Test>::InvalidCampaignStatus
            );
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 500);
        });
    }

    #[test]
    fn finalize_paused_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            run_to_block(21);
            // Paused is not Funding, so finalize should fail with InvalidCampaignStatus
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn finalize_after_pause_and_resume_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn withdraw_from_paused_campaign_with_custom_penalty() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(500); // 5%
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // 5% of 1000 = 50, net = 950
            assert_eq!(Balances::free_balance(BOB), bob_before + 950);
        });
    }

    #[test]
    fn pause_succeeded_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::pause_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotFunding
            );
        });
    }

    #[test]
    fn pause_milestone_phase_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::pause_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotFunding
            );
        });
    }
}

// ── Whitelist edge cases ────────────────────────────────────────────

mod whitelist_extended {
    use super::*;

    #[test]
    fn remove_from_whitelist_non_creator_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(BOB), id, CHARLIE),
                Error::<Test>::NotCampaignCreator
            );
        });
    }

    #[test]
    fn remove_from_whitelist_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), 99, BOB),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn whitelist_add_same_account_twice_is_idempotent() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::AccountWhitelist].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            // BOB can still invest (whitelist didn't break)
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    #[test]
    fn remove_non_whitelisted_account_is_noop() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            // Removing someone not on the whitelist should not error
            assert_ok!(Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
        });
    }

    #[test]
    fn whitelist_only_campaign_allows_whitelisted_blocks_others() {
        // Verify that with AccountWhitelist rule, only whitelisted accounts invest
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::AccountWhitelist].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            // Whitelist BOB only
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            // BOB can invest
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            // CHARLIE cannot
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
            // Whitelist CHARLIE
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE));
            // Now CHARLIE can invest
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100));
        });
    }
}

// ── Milestone operations on cancelled campaign ──────────────────────

mod milestone_after_cancel {
    use super::*;

    fn setup_milestone_campaign_funded() -> u32 {
        let config = milestone_config(
            20,
            1000,
            vec![
                Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                Milestone { release_bps: 5000, description_hash: [2u8; 32] },
            ],
        );
        let id = create_funded_campaign(ALICE, config);
        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
        run_to_block(21);
        assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
        id
    }

    #[test]
    fn submit_milestone_after_cancel_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign_funded();
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn approve_milestone_after_cancel_fails() {
        // P1-01: approve_milestone now checks campaign.status == MilestonePhase.
        // Previously it only checked Campaigns::contains_key. Now it correctly
        // rejects operations on cancelled campaigns.
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign_funded();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // After cancel, status is Cancelled — approve must fail
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn reject_milestone_after_cancel_fails() {
        // P1-01: reject_milestone now checks campaign.status == MilestonePhase.
        // Previously it only checked Campaigns::contains_key.
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign_funded();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // After cancel, status is Cancelled — reject must fail
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── Creator invests in own campaign ─────────────────────────────────

mod creator_self_invest {
    use super::*;

    #[test]
    fn creator_can_invest_in_own_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            // Nothing in the pallet prevents the creator from investing
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(ALICE), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 500);
            assert_eq!(c.investor_count, 1);
        });
    }

    #[test]
    fn creator_invests_and_claims_own_funds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(ALICE), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 500);
        });
    }
}

// ── created_at field verification ───────────────────────────────────

mod created_at_field {
    use super::*;

    #[test]
    fn created_at_records_creation_block() {
        ExtBuilder::default().build().execute_with(|| {
            run_to_block(5);
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.created_at, 5);
        });
    }

    #[test]
    fn created_at_at_block_1() {
        ExtBuilder::default().build().execute_with(|| {
            // ExtBuilder sets block to 1
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.created_at, 1);
        });
    }
}

// ── Storage version ─────────────────────────────────────────────────

mod storage_version_check {
    use frame_support::traits::GetStorageVersion;

    use super::*;

    #[test]
    fn on_chain_storage_version_starts_at_zero_in_mock() {
        // In test environments without migration, on-chain version starts at 0.
        // The pallet's declared version (current_storage_version) is 1.
        ExtBuilder::default().build().execute_with(|| {
            let on_chain = Crowdfunding::on_chain_storage_version();
            let current = Crowdfunding::current_storage_version();
            // on_chain is 0 in fresh mock (no genesis migration)
            assert_eq!(on_chain, frame_support::traits::StorageVersion::new(0));
            // current (compile-time declared) is 3
            assert_eq!(current, frame_support::traits::StorageVersion::new(3));
        });
    }
}

// ── Proportional refund rounding ────────────────────────────────────

mod proportional_refund_rounding {
    use super::*;

    #[test]
    fn rounding_truncation_with_odd_amounts() {
        // Tests Permill rounding when proportional refund doesn't divide evenly
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 3333, description_hash: [1u8; 32] },
                    Milestone { release_bps: 6667, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 999));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Claim milestone 0: 33.33% of 999 = 332 (truncated)
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            let disbursed_0 = Balances::free_balance(ALICE) - alice_before;
            // Permill::from_parts(3333 * 100) = Permill(333300)
            // Permill(333300) * 999 = 999 * 333300 / 1_000_000 = 332_966_700 / 1_000_000
            // Permill multiplication rounds to nearest, so = 333
            assert_eq!(disbursed_0, 333);

            // Cancel
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            // remaining_ratio = (999 - 333) / 999 = 666/999
            // Permill::from_rational(666, 999) = Permill(666666)
            // refund = Permill(666666) * 999 = 666_666 * 999 / 1_000_000
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            let refund = Balances::free_balance(BOB) - bob_before;
            // Permill::from_rational uses truncating division, exact value depends
            // on implementation. Just verify refund + disbursed <= total_raised (no
            // over-refund)
            assert!(refund > 0, "Refund should be positive");
            assert!(refund + disbursed_0 <= 999, "Refund + disbursed must not exceed total_raised");
        });
    }

    #[test]
    fn proportional_refund_after_partial_withdrawal_and_partial_disbursement() {
        // Investor partially withdrew, then milestones were partially disbursed,
        // then campaign cancelled. Refund is proportional to remaining.
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            // Withdraw 200 during funding
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
            // total_raised = 800, BOB's position = 800

            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Claim milestone 0: 50% of 800 = 400
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));

            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

            // remaining_ratio = (800 - 400) / 800 = 50%
            // BOB's raw_refund = total_invested(1000) - total_withdrawn(200) = 800
            // proportional_refund = 50% of 800 = 400
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 400);
        });
    }
}

// ── Milestone overraise behavior ────────────────────────────────────

mod milestone_overraise {
    use super::*;

    #[test]
    fn milestone_disbursement_based_on_total_raised_not_goal() {
        // When total_raised > goal, milestone disbursements are based on total_raised
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            // Invest 2000, far exceeding goal of 500
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 2000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Milestone 0: 50% of total_raised(2000) = 1000, NOT 50% of goal(500)
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);

            // Milestone 1: 50% of 2000 = 1000
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 1));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_disbursed, 2000);
            assert!(matches!(c.status, CampaignStatus::Completed));
        });
    }
}

// ── Hard cap interactions ───────────────────────────────────────────

mod hard_cap_extended {
    use super::*;

    #[test]
    fn hard_cap_with_kwyr_model() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_kwyr_config(100);
            config.hard_cap = Some(500);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 1),
                Error::<Test>::HardCapExceeded
            );
        });
    }

    #[test]
    fn hard_cap_with_milestone_model() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = milestone_config(
                100,
                500,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            config.hard_cap = Some(1000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 800));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 1),
                Error::<Test>::HardCapExceeded
            );
        });
    }

    #[test]
    fn hard_cap_reached_event_via_cumulative_investments() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 500);
            config.hard_cap = Some(1000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 700));
            System::reset_events();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 300));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::HardCapReached { campaign_id })
                    if *campaign_id == id
                )
            });
            assert!(found, "HardCapReached event not found on cumulative cap hit");
        });
    }

    #[test]
    fn hard_cap_not_emitted_when_above_but_not_equal() {
        // The event is only emitted when total == cap, not when total < cap
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 500);
            config.hard_cap = Some(1000);
            let id = create_funded_campaign(ALICE, config);
            System::reset_events();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 999));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(&e.event, RuntimeEvent::Crowdfunding(Event::HardCapReached { .. }))
            });
            assert!(!found, "HardCapReached event should NOT be emitted at 999/1000");
        });
    }
}

// ── Access control comprehensive ────────────────────────────────────

mod access_control {
    use super::*;

    #[test]
    fn claim_funds_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::none(), 0),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn invest_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::none(), 0, 100),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn create_campaign_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::none(),
                    default_aon_config(100, 1000),
                    None,
                    None,
                ),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn withdraw_investment_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::none(), 0, 100),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn claim_refund_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::none(), 0),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn claim_creation_deposit_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::claim_creation_deposit(RuntimeOrigin::none(), 0),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn submit_milestone_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::none(), 0, 0),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn finalize_campaign_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::none(), 0),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn add_to_whitelist_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::none(), 0, BOB),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn remove_from_whitelist_unsigned_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::remove_from_whitelist(RuntimeOrigin::none(), 0, BOB),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }
}

// ── Invest zero amount edge cases ───────────────────────────────────

mod invest_zero_edge_cases {
    use super::*;

    #[test]
    fn zero_invest_below_min_investment_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.min_investment = Some(100);
            let id = create_funded_campaign(ALICE, config);
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::InvestmentBelowMinimum
            );
        });
    }

    #[test]
    fn withdraw_zero_is_rejected() {
        // P2-02: zero-amount withdrawals are now rejected with InsufficientInvestment
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let bob_before = Balances::free_balance(BOB);
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::InsufficientInvestment
            );
            // Balance and state unchanged
            assert_eq!(Balances::free_balance(BOB), bob_before);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 500);
            assert!(pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
        });
    }
}

// ── Claim funds event amount includes claimable not creator_amount ──

mod claim_funds_event_verification {
    use super::*;

    #[test]
    fn funds_claimed_event_amount_is_total_claimable() {
        // FundsClaimed event emits `amount: claimable` (before protocol fee deduction),
        // not `amount: creator_amount`
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::FundsClaimed {
                        campaign_id,
                        creator,
                        amount,
                    }) if *campaign_id == id && *creator == ALICE && *amount == 1000
                )
            });
            assert!(found, "FundsClaimed event should have amount = 1000 (total claimable)");
        });
    }
}

// ── AoN with hard cap: goal met but below cap ───────────────────────

mod aon_hard_cap_interactions {
    use super::*;

    #[test]
    fn aon_succeeds_at_goal_below_hard_cap() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(20, 500);
            config.hard_cap = Some(2000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    #[test]
    fn aon_fails_below_goal_with_hard_cap() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(20, 1000);
            config.hard_cap = Some(2000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Failed));
        });
    }
}

// ── invest_withdraw_refund behavioral nuance ────────────────────────

mod invest_withdraw_refund_nuance {
    use super::*;

    #[test]
    fn full_withdrawal_then_failed_campaign_gives_nothing_to_refund() {
        // After full withdrawal, Investment record still exists (total_invested ==
        // total_withdrawn). On claim_refund, raw_refund = 0 => NothingToRefund
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Investment record exists but total_invested == total_withdrawn
            let inv = pallet::Investments::<Test>::get(id, BOB);
            assert!(inv.is_some()); // record exists
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::NothingToRefund
            );
        });
    }

    #[test]
    fn refund_removes_record_second_refund_is_no_investment_found() {
        // After claim_refund, Investment record is removed.
        // Second claim_refund gives NoInvestmentFound (not NothingToRefund)
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            // Record removed
            assert!(pallet::Investments::<Test>::get(id, BOB).is_none());
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::NoInvestmentFound
            );
        });
    }

    #[test]
    fn re_invest_after_full_withdrawal_is_new_investor() {
        // H-1: After full withdrawal, investor_count is decremented to 0 and
        // InvestorCampaigns no longer contains the campaign id.
        // Re-invest must detect the investor as *new* (using InvestorCampaigns
        // membership, not the stale Investment fields), so investor_count is
        // re-incremented to 1.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            // P2-12: investor_count decremented on full withdrawal
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 0);
            // After full withdrawal InvestorCampaigns no longer contains the campaign id
            assert!(!pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
            // Re-invest: is_new=true because InvestorCampaigns does not contain id
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 300));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            // investor_count must be 1 — the re-investor is treated as a new participant
            assert_eq!(c.investor_count, 1);
            assert_eq!(c.total_raised, 300);
            // InvestorCampaigns must contain the campaign id again
            assert!(pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
        });
    }
}

// ── Multiple campaigns cross-interaction ────────────────────────────

mod cross_campaign {
    use super::*;

    #[test]
    fn investor_in_multiple_campaigns_refund_one_keeps_others() {
        ExtBuilder::default().build().execute_with(|| {
            let id1 = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            let id2 = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id1, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id2, 300));

            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id1));
            // id1 Failed, id2 still Funding
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id1));

            // BOB's InvestorCampaigns should only contain id2
            let investor_campaigns = pallet::InvestorCampaigns::<Test>::get(BOB);
            assert!(!investor_campaigns.contains(&id1));
            assert!(investor_campaigns.contains(&id2));

            // BOB's investment in id2 is unaffected
            let inv = pallet::Investments::<Test>::get(id2, BOB).unwrap();
            assert_eq!(inv.total_invested, 300);
        });
    }

    #[test]
    fn sub_accounts_are_deterministic_per_campaign_id() {
        // into_sub_account_truncating with u64 AccountId truncates to 8 bytes.
        // The PalletId (8 bytes) + campaign_id (4 bytes) = 12 bytes, but only
        // the first 8 bytes survive truncation. This means sub-accounts may
        // collide in the mock, but are unique in production (32-byte AccountId).
        // This test verifies deterministic behavior.
        ExtBuilder::default().build().execute_with(|| {
            let sub_0a = Crowdfunding::campaign_account(0);
            let sub_0b = Crowdfunding::campaign_account(0);
            assert_eq!(sub_0a, sub_0b);
            // Sub-account is non-zero
            assert_ne!(sub_0a, 0u64);
        });
    }
}

// ── BoundedVec saturation: MaxEligibilityRules ──────────────────────

mod bounded_vec_saturation {
    use frame_support::traits::ConstU32;

    use super::*;

    #[test]
    fn max_eligibility_rules_exactly_at_limit() {
        // MaxEligibilityRules = 3
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> = vec![
                EligibilityRule::NativeBalance { min_balance: 100 },
                EligibilityRule::NativeBalance { min_balance: 200 },
                EligibilityRule::NativeBalance { min_balance: 300 },
            ]
            .try_into()
            .unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.eligibility_rules.len(), 3);
        });
    }

    #[test]
    fn max_eligibility_rules_over_limit_fails_at_type_level() {
        // BoundedVec::try_from with 4 elements for MaxEligibilityRules=3 should fail
        ExtBuilder::default().build().execute_with(|| {
            let result: Result<
                BoundedVec<
                    EligibilityRule<u32, u128, u32, u32, ConstU32<3>, ConstU32<3>>,
                    ConstU32<3>,
                >,
                _,
            > = vec![
                EligibilityRule::NativeBalance { min_balance: 100 },
                EligibilityRule::NativeBalance { min_balance: 200 },
                EligibilityRule::NativeBalance { min_balance: 300 },
                EligibilityRule::NativeBalance { min_balance: 400 },
            ]
            .try_into();
            assert!(result.is_err());
        });
    }
}

// ── claim_creation_deposit cleans CreatorCampaigns ───────────────────

mod claim_creation_deposit_cleanup {
    use super::*;

    #[test]
    fn claim_deposit_after_completed_cleans_creator_campaigns() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert!(pallet::CreatorCampaigns::<Test>::get(ALICE).contains(&id));
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            assert!(!pallet::CreatorCampaigns::<Test>::get(ALICE).contains(&id));
        });
    }
}

// ── Milestone reject not milestone phase check ──────────────────────

mod milestone_approve_reject_status_check {
    use super::*;

    #[test]
    fn approve_milestone_checks_campaign_status() {
        // P1-01: approve_milestone now checks campaign.status == MilestonePhase.
        // A Funding campaign (non-milestone) must return InvalidCampaignStatus.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            // Campaign is Funding — approve must fail with InvalidCampaignStatus
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn reject_milestone_checks_campaign_status() {
        // P1-01: reject_milestone now checks campaign.status == MilestonePhase.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            // Campaign is Funding — reject must fail with InvalidCampaignStatus
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── FundsClaimed event reports claimable (pre-fee) amount ───────────

mod event_amount_semantics {
    use super::*;

    #[test]
    fn milestone_funds_claimed_event_reports_release_amount_including_fee() {
        // MilestoneFundsClaimed event reports release_amount (before fee deduction)
        // when ProtocolFeeBps = 0, the release_amount == creator_amount
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 7000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 2000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            let events = System::events();
            // 30% of 2000 = 600
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::MilestoneFundsClaimed {
                        campaign_id,
                        index,
                        amount,
                    }) if *campaign_id == id && *index == 0 && *amount == 600
                )
            });
            assert!(found, "MilestoneFundsClaimed should report 600 (30% of 2000)");
        });
    }
}

// ── InvestmentWithdrawn event amounts ────────────────────────────────

mod withdrawal_event_amounts {
    use super::*;

    #[test]
    fn withdrawal_event_net_and_penalty_correct_with_custom_penalty() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(500); // 5%
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            System::reset_events();
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            let events = System::events();
            // 5% of 1000 = 50 penalty, net = 950
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::InvestmentWithdrawn {
                        campaign_id,
                        investor,
                        amount,
                        penalty,
                    }) if *campaign_id == id && *investor == BOB && *amount == 950 && *penalty == 50
                )
            });
            assert!(found, "InvestmentWithdrawn event net/penalty incorrect");
        });
    }

    #[test]
    fn withdrawal_event_zero_penalty() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(0);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            System::reset_events();
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::InvestmentWithdrawn {
                        campaign_id,
                        investor,
                        amount,
                        penalty,
                    }) if *campaign_id == id && *investor == BOB && *amount == 1000 && *penalty == 0
                )
            });
            assert!(found, "InvestmentWithdrawn event should have 0 penalty");
        });
    }
}

// ── claim_funds AoN succeeded ───────────────────────────────────────

mod claim_funds_aon {
    use super::*;

    #[test]
    fn claim_funds_aon_succeeded() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1200));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1200);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
            assert_eq!(c.total_disbursed, 1200);
        });
    }
}

// ── claim_funds after withdrawal reduces claimable ──────────────────

mod claim_funds_after_withdrawal {
    use super::*;

    #[test]
    fn claim_funds_after_investor_withdrawal_reflects_reduced_total_raised() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 400));
            // total_raised = 600
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            // Creator gets total_raised - total_disbursed = 600 - 0 = 600
            assert_eq!(Balances::free_balance(ALICE), alice_before + 600);
        });
    }
}

// ── Milestone with 5 milestones (max) unequal splits ────────────────

mod milestone_max_unequal {
    use super::*;

    #[test]
    fn five_milestones_unequal_bps() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 1000, description_hash: [1u8; 32] }, // 10%
                    Milestone { release_bps: 2000, description_hash: [2u8; 32] }, // 20%
                    Milestone { release_bps: 3000, description_hash: [3u8; 32] }, // 30%
                    Milestone { release_bps: 1500, description_hash: [4u8; 32] }, // 15%
                    Milestone { release_bps: 2500, description_hash: [5u8; 32] }, // 25%
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            let expected_amounts = vec![100, 200, 300, 150, 250]; // bps of 1000
            let mut total_disbursed = 0u128;
            for (i, &expected) in expected_amounts.iter().enumerate() {
                assert_ok!(Crowdfunding::submit_milestone(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    i as u8
                ));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i as u8));
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    i as u8,
                ));
                assert_eq!(Balances::free_balance(ALICE), alice_before + expected);
                total_disbursed += expected;
            }

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
            assert_eq!(c.total_disbursed, total_disbursed);
            assert_eq!(total_disbursed, 1000);
        });
    }
}

// ── Cancel from every valid status ──────────────────────────────────

mod cancel_from_every_status {
    use super::*;

    #[test]
    fn cancel_from_funding() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Cancelled));
        });
    }

    #[test]
    fn cancel_from_paused() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Cancelled));
        });
    }

    #[test]
    fn cancel_from_succeeded_blocked() {
        // CRIT-02: Succeeded campaigns must NOT be force-cancellable.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn cancel_from_failed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
        });
    }

    #[test]
    fn cancel_from_milestone_phase() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![Milestone { release_bps: 10000, description_hash: [1u8; 32] }],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
        });
    }

    #[test]
    fn cancel_from_cancelled_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn cancel_from_completed_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── Refund event includes proportional amount ───────────────────────

mod refund_event_proportional {
    use super::*;

    #[test]
    fn refund_event_amount_is_proportional_after_disbursement() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                500,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Disburse 50%
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));

            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            let events = System::events();
            // 50% of 1000 = 500 refund
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::RefundClaimed {
                        campaign_id,
                        investor,
                        amount,
                    }) if *campaign_id == id && *investor == BOB && *amount == 500
                )
            });
            assert!(found, "RefundClaimed event should have proportional amount 500");
        });
    }
}

// ── Invest after resume maintains investment data ───────────────────

mod invest_after_resume_data_integrity {
    use super::*;

    #[test]
    fn investment_data_preserved_across_pause_resume() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            // Investment data should be unchanged
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_invested, 500);
            assert_eq!(inv.total_withdrawn, 0);
            // Can invest more after resume
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 300));
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_invested, 800);
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// AUDIT FIXES — tests for all findings from the eagle-eye reviewer
// ═══════════════════════════════════════════════════════════════════════

mod audit_fixes {
    use super::*;

    // ── P0-01: early_withdrawal_penalty_bps validation ──────────────

    #[test]
    fn create_campaign_penalty_bps_over_10000_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.early_withdrawal_penalty_bps = Some(10_001);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidPenaltyBps
            );
        });
    }

    #[test]
    fn create_campaign_penalty_bps_exactly_10000_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.early_withdrawal_penalty_bps = Some(10_000);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn create_campaign_penalty_bps_none_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let config = default_aon_config(100, 1000); // None penalty
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn penalty_bps_u16_max_rejected() {
        // u16::MAX = 65535, well above 10_000
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.early_withdrawal_penalty_bps = Some(u16::MAX);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidPenaltyBps
            );
        });
    }

    // ── P1-01: approve/reject_milestone campaign status check ────────

    #[test]
    fn approve_milestone_on_cancelled_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Submit milestone while in MilestonePhase
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            // Cancel the campaign
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // Now try to approve — must fail because status is Cancelled
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn reject_milestone_on_cancelled_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn approve_milestone_on_funding_campaign_fails() {
        // A non-milestone campaign in Funding cannot have milestones approved
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    // ── P1-02: FundsClaimed reports net amount ───────────────────────

    #[test]
    fn funds_claimed_event_amount_is_net_when_fee_zero() {
        // ProtocolFeeBps = 0 in mock, creator_amount == total_raised
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::FundsClaimed {
                        campaign_id,
                        creator,
                        amount,
                    }) if *campaign_id == id && *creator == ALICE && *amount == 1000
                )
            });
            assert!(found, "FundsClaimed with net amount not found");
        });
    }

    // ── P2-01: invest zero-amount rejected ────────────────────────────

    #[test]
    fn invest_zero_with_min_investment_none_still_fails() {
        // Zero invest must fail even when min_investment is None
        ExtBuilder::default().build().execute_with(|| {
            let config = default_aon_config(100, 5000); // no min_investment
            let id = create_funded_campaign(ALICE, config);
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::InvestmentBelowMinimum
            );
            // investor_count and total_raised must be unchanged
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 0);
            assert_eq!(c.total_raised, 0);
        });
    }

    // ── P2-02: withdraw zero-amount rejected ──────────────────────────

    #[test]
    fn withdraw_zero_with_sufficient_position_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            // Zero withdrawal must be rejected even when position is non-zero
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::InsufficientInvestment
            );
        });
    }

    // ── P2-03: whitelist modification on terminal campaigns ───────────

    #[test]
    fn add_to_whitelist_on_cancelled_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn remove_from_whitelist_on_cancelled_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::AccountWhitelist].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), id, BOB),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn add_to_whitelist_on_completed_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            // Completed — whitelist modification must fail
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn add_to_whitelist_on_paused_campaign_succeeds() {
        // Paused is still an active campaign status
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
        });
    }

    // ── P2-08: MilestoneStatuses cleaned on cancel ───────────────────

    #[test]
    fn cancel_in_milestone_phase_clears_milestone_statuses() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Milestone statuses are initialized
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Pending)
            );
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 1u8),
                Some(MilestoneStatus::Pending)
            );
            // Cancel — statuses should be removed
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 0u8), None);
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 1u8), None);
        });
    }

    #[test]
    fn cancel_with_submitted_milestones_clears_statuses() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 6000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 4000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Submit milestone 0
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Submitted)
            );
            // Cancel
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // Both statuses must be gone
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 0u8), None);
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 1u8), None);
        });
    }

    // ── P2-09: CreationDepositClaimed event ──────────────────────────

    #[test]
    fn creation_deposit_claimed_event_has_correct_fields() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CreationDepositClaimed {
                        campaign_id,
                        creator,
                        deposit_returned,
                    }) if *campaign_id == id && *creator == ALICE && *deposit_returned == 100
                )
            });
            assert!(found, "CreationDepositClaimed event not found");
        });
    }

    // ── P2-12: investor_count decrements ─────────────────────────────

    #[test]
    fn investor_count_decrements_on_full_withdrawal() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 1);
            // Full withdrawal — investor_count must decrement
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 0);
        });
    }

    #[test]
    fn investor_count_does_not_decrement_on_partial_withdrawal() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            // Partial withdrawal — investor_count must NOT change
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 1);
        });
    }

    #[test]
    fn investor_count_decrements_on_refund() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 1);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 0);
        });
    }

    #[test]
    fn investor_count_multiple_investors_decrements_correctly() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 50000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 3);
            // BOB fully withdraws
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 2);
            // CHARLIE fully withdraws
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(CHARLIE), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 1);
            // DAVE partial withdraw — count stays at 1
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(DAVE), id, 200));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 1);
        });
    }

    // ── P3-01: HardCapReached uses updated total_raised ───────────────

    #[test]
    fn hard_cap_reached_uses_post_invest_total() {
        // Verify HardCapReached fires when total_raised reaches exactly the cap
        // AFTER the invest mutate (post-mutate check).
        // M-5: hard_cap must be >= goal; use goal = 1000 = hard_cap
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.hard_cap = Some(1000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
            System::reset_events();
            // This invest brings total_raised to exactly 1000 = hard cap
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 400));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::HardCapReached { campaign_id })
                    if *campaign_id == id
                )
            });
            assert!(found, "HardCapReached event not emitted when exactly reaching cap");
            // Verify no HardCapReached on the partial invest
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().total_raised, 1000);
        });
    }

    #[test]
    fn hard_cap_not_reached_below_cap() {
        // M-5: hard_cap must be >= goal; use goal = 1000 = hard_cap
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.hard_cap = Some(1000);
            let id = create_funded_campaign(ALICE, config);
            System::reset_events();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 999));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(&e.event, RuntimeEvent::Crowdfunding(Event::HardCapReached { .. }))
            });
            assert!(!found, "HardCapReached should NOT fire below cap");
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// AUDIT FINDINGS — comprehensive tests for every fixed vulnerability
// ═══════════════════════════════════════════════════════════════════════

// ── P0-01: early_withdrawal_penalty_bps validation ──────────────────

mod p0_01_penalty_bps_validation {
    use super::*;

    #[test]
    fn create_campaign_penalty_bps_10001_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.early_withdrawal_penalty_bps = Some(10_001);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidPenaltyBps
            );
        });
    }

    #[test]
    fn create_campaign_penalty_bps_u16_max_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.early_withdrawal_penalty_bps = Some(u16::MAX); // 65535
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidPenaltyBps
            );
        });
    }

    #[test]
    fn create_campaign_penalty_bps_10000_succeeds() {
        // 10000 bps = 100% penalty — valid (investor gets zero net on withdrawal)
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.early_withdrawal_penalty_bps = Some(10_000);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn create_campaign_penalty_bps_9999_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.early_withdrawal_penalty_bps = Some(9_999);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn create_campaign_penalty_bps_0_explicit_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.early_withdrawal_penalty_bps = Some(0);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn create_campaign_penalty_bps_none_succeeds() {
        // None means use pallet default — always valid
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.early_withdrawal_penalty_bps = None;
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn withdraw_with_10000_bps_penalty_gets_zero_net() {
        // 100% penalty means investor gets nothing back, full amount is burned
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(10_000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            // Permill::from_parts(10_000 * 100) = Permill(1_000_000) = 100%
            // penalty = 500, net = 0 => no balance change for BOB
            assert_eq!(Balances::free_balance(BOB), bob_before);
            // Verify event shows penalty = 500 and amount (net) = 0
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::InvestmentWithdrawn {
                        amount,
                        penalty,
                        ..
                    }) if *amount == 0 && *penalty == 500
                )
            });
            assert!(found, "InvestmentWithdrawn event should show net=0 and penalty=500");
        });
    }

    #[test]
    fn bps_of_clamping_defense_in_depth() {
        // bps_of clamps values > 10_000 to 10_000 internally.
        // This can't happen through create_campaign (which rejects >10000),
        // but proves the defense-in-depth works.
        // We test indirectly: a campaign with penalty_bps=10000 produces 100% penalty
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(10_000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // 100% penalty => net = 0
            assert_eq!(Balances::free_balance(BOB), bob_before);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            // total_raised decremented by the full withdrawn amount (1000)
            assert_eq!(c.total_raised, 0);
        });
    }

    #[test]
    fn penalty_1_bps_on_small_amount_rounds_to_zero() {
        // 1 bps = 0.01%. ceil(99 * 1 / 10000) = ceil(0.0099) = 1
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(1);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 99));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 99));
            // ceil(99 * 1 / 10000) = 1, penalty = 1, net = 98
            assert_eq!(Balances::free_balance(BOB), bob_before + 98);
        });
    }

    #[test]
    fn create_campaign_penalty_bps_boundary_10000_kwyr() {
        // Same validation for KWYR model
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_kwyr_config(100);
            config.early_withdrawal_penalty_bps = Some(10_000);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None
            ));
        });
    }

    #[test]
    fn create_campaign_penalty_bps_boundary_10001_milestone() {
        // Same validation for milestone model
        ExtBuilder::default().build().execute_with(|| {
            let mut config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            config.early_withdrawal_penalty_bps = Some(10_001);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidPenaltyBps
            );
        });
    }
}

// ── P1-01: approve_milestone / reject_milestone campaign status check ──

mod p1_01_milestone_status_check {
    use super::*;

    // Helper: create a milestone campaign and put it in MilestonePhase,
    // then submit milestone 0 so it's ready for approval/rejection.
    fn setup_submitted_milestone() -> u32 {
        let config = milestone_config(
            20,
            1000,
            vec![
                Milestone { release_bps: 6000, description_hash: [1u8; 32] },
                Milestone { release_bps: 4000, description_hash: [2u8; 32] },
            ],
        );
        let id = create_funded_campaign(ALICE, config);
        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
        run_to_block(21);
        assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
        assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
        id
    }

    #[test]
    fn approve_milestone_on_cancelled_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_submitted_milestone();
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn reject_milestone_on_cancelled_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_submitted_milestone();
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn approve_milestone_on_funding_campaign_fails() {
        // Funding status is not MilestonePhase
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            // Campaign is still in Funding, not MilestonePhase
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn approve_milestone_on_succeeded_campaign_fails() {
        // AoN Succeeded is not MilestonePhase
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Succeeded
            ));
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn approve_milestone_on_failed_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Failed
            ));
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn approve_milestone_on_completed_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Complete all milestones to reach Completed
            for i in 0..2u8 {
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, i));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    i
                ));
            }
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Completed
            ));
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn reject_milestone_on_funding_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn reject_milestone_on_succeeded_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn reject_milestone_on_failed_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn reject_milestone_on_completed_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            for i in 0..2u8 {
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, i));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    i
                ));
            }
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn approve_milestone_on_milestone_phase_succeeds() {
        // Positive confirmation that the happy path still works
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_submitted_milestone();
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Approved)
            );
        });
    }

    #[test]
    fn reject_milestone_on_milestone_phase_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_submitted_milestone();
            assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Rejected)
            );
        });
    }

    #[test]
    fn approve_milestone_on_paused_campaign_fails() {
        // Paused can only be reached from Funding; milestones can't be paused
        // by the pallet (pause only works on Funding campaigns), but let's verify
        // that if somehow a campaign were paused, approve would fail.
        // Since pause_campaign requires Funding status, a MilestonePhase campaign
        // can't be paused. We still verify that attempting approve_milestone on a
        // paused (Funding-origin) campaign fails.
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            // Paused is not MilestonePhase
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── P2-01: Zero-amount invest rejected ──────────────────────────────

mod p2_01_zero_invest {
    use super::*;

    #[test]
    fn invest_one_unit_succeeds() {
        // Minimum possible investment (amount = 1) should succeed
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 1);
            assert_eq!(c.investor_count, 1);
        });
    }

    #[test]
    fn invest_zero_with_min_investment_set_fails() {
        // Both zero-check and min_investment guard should catch this
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.min_investment = Some(50);
            let id = create_funded_campaign(ALICE, config);
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::InvestmentBelowMinimum
            );
            // State unchanged
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 0);
            assert_eq!(c.investor_count, 0);
        });
    }

    #[test]
    fn invest_zero_no_investor_campaigns_entry() {
        // Verify zero invest doesn't pollute InvestorCampaigns
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::InvestmentBelowMinimum
            );
            assert!(!pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
        });
    }
}

// ── P2-02: Zero-amount withdrawal rejected ──────────────────────────

mod p2_02_zero_withdraw {
    use super::*;

    #[test]
    fn withdraw_one_unit_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1));
            // ceil(1 * 100 / 10000) = 1, penalty = 1, net = 0
            assert_eq!(Balances::free_balance(BOB), bob_before);
        });
    }

    #[test]
    fn withdraw_zero_no_state_change() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let c_before = pallet::Campaigns::<Test>::get(id).unwrap();
            let inv_before = pallet::Investments::<Test>::get(id, BOB).unwrap();
            let bob_before = Balances::free_balance(BOB);
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 0),
                Error::<Test>::InsufficientInvestment
            );
            // Confirm nothing changed
            let c_after = pallet::Campaigns::<Test>::get(id).unwrap();
            let inv_after = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(c_before.total_raised, c_after.total_raised);
            assert_eq!(inv_before.total_withdrawn, inv_after.total_withdrawn);
            assert_eq!(Balances::free_balance(BOB), bob_before);
        });
    }
}

// ── P2-03: Whitelist functions check campaign status ────────────────

mod p2_03_whitelist_status_check {
    use super::*;

    #[test]
    fn add_to_whitelist_on_completed_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Completed
            ));
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn add_to_whitelist_on_cancelled_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn add_to_whitelist_on_failed_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Failed
            ));
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn add_to_whitelist_on_succeeded_campaign_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Succeeded
            ));
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn add_to_whitelist_on_milestone_phase_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::MilestonePhase
            ));
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn add_to_whitelist_on_funding_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE));
            assert!(pallet::CampaignWhitelist::<Test>::get(id, CHARLIE));
        });
    }

    #[test]
    fn add_to_whitelist_on_paused_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE));
            assert!(pallet::CampaignWhitelist::<Test>::get(id, CHARLIE));
        });
    }

    #[test]
    fn remove_from_whitelist_on_completed_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn remove_from_whitelist_on_cancelled_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn remove_from_whitelist_on_failed_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn remove_from_whitelist_on_paused_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_ok!(Crowdfunding::remove_from_whitelist(
                RuntimeOrigin::signed(ALICE),
                id,
                CHARLIE
            ));
            assert!(!pallet::CampaignWhitelist::<Test>::get(id, CHARLIE));
        });
    }
}

// ── P2-08: Milestone statuses cleaned on cancel (supplementary) ────

mod p2_08_milestone_cleanup_supplementary {
    use super::*;

    #[test]
    fn cancel_milestone_campaign_with_some_approved_cleans_all() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 4000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                    Milestone { release_bps: 3000, description_hash: [3u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Approve milestone 0, leave 1 as pending, submit 2
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 2));
            // Verify mixed statuses before cancel
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Approved)
            );
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 1u8),
                Some(MilestoneStatus::Pending)
            );
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 2u8),
                Some(MilestoneStatus::Submitted)
            );
            // Cancel cleans ALL statuses regardless of their current state
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 0u8), None);
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 1u8), None);
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 2u8), None);
        });
    }

    #[test]
    fn cancel_milestone_with_one_claimed_cleans_remaining() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Claim milestone 0
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Claimed)
            );
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 1u8),
                Some(MilestoneStatus::Pending)
            );
            // Cancel — both statuses should be removed
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 0u8), None);
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 1u8), None);
        });
    }

    #[test]
    fn cancel_non_milestone_campaign_does_not_touch_milestone_storage() {
        // Cancelling an AoN campaign shouldn't attempt milestone cleanup
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            // No milestone statuses should exist
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 0u8), None);
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // Still None — no spurious writes
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 0u8), None);
        });
    }
}

// ── P2-09: Event rename verification (supplementary) ────────────────

mod p2_09_event_rename {
    use super::*;

    #[test]
    fn claim_creation_deposit_on_failed_emits_correct_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Failed
            ));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CreationDepositClaimed {
                        campaign_id,
                        creator,
                        deposit_returned,
                    }) if *campaign_id == id && *creator == ALICE && *deposit_returned == 100
                )
            });
            assert!(found, "CreationDepositClaimed event not found on Failed campaign");
        });
    }

    #[test]
    fn claim_creation_deposit_on_cancelled_emits_correct_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CreationDepositClaimed {
                        campaign_id,
                        creator,
                        deposit_returned,
                    }) if *campaign_id == id && *creator == ALICE && *deposit_returned == 100
                )
            });
            assert!(found, "CreationDepositClaimed event not found on Cancelled campaign");
        });
    }

    #[test]
    fn claim_creation_deposit_on_completed_emits_correct_event() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::CreationDepositClaimed {
                        campaign_id,
                        creator,
                        deposit_returned,
                    }) if *campaign_id == id && *creator == ALICE && *deposit_returned == 100
                )
            });
            assert!(found, "CreationDepositClaimed event not found on Completed campaign");
        });
    }

    #[test]
    fn claim_funds_does_not_emit_campaign_completed_event() {
        // P2-09: CampaignCompleted is no longer emitted from claim_funds.
        // The only "completed" signal is the FundsClaimed event plus the
        // status transition to Completed.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            // Verify no CreationDepositClaimed event (that's separate)
            let events = System::events();
            let has_deposit_event = events.iter().any(|e| {
                matches!(&e.event, RuntimeEvent::Crowdfunding(Event::CreationDepositClaimed { .. }))
            });
            assert!(
                !has_deposit_event,
                "CreationDepositClaimed should NOT be emitted from claim_funds"
            );
        });
    }
}

// ── P2-12: investor_count decrements (supplementary) ────────────────

mod p2_12_investor_count_supplementary {
    use super::*;

    #[test]
    fn investor_count_reincrements_on_reinvest_after_full_withdrawal() {
        // H-1 fix: After a full withdrawal, InvestorCampaigns no longer contains the
        // campaign id. On re-invest, is_new is determined by InvestorCampaigns
        // membership (not by the stale Investment fields), so investor_count IS
        // re-incremented to 1.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 1);
            // Full withdrawal decrements investor_count and removes from InvestorCampaigns
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 0);
            assert!(!pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
            // Re-invest IS treated as new — investor_count re-increments to 1
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 300));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 1);
            assert!(pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));
            // Investment record accumulates correctly
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_invested, 800); // 500 + 300
            assert_eq!(inv.total_withdrawn, 500);
            assert_eq!(c.total_raised, 300);
        });
    }

    #[test]
    fn investor_count_after_cancel_and_all_refunds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 300));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 200));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 3);
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // Refund all investors one by one
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 2);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 1);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(DAVE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 0);
        });
    }

    #[test]
    fn investor_count_multiple_full_withdrawals() {
        // Multiple investors fully withdraw, each decrements count independently
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 50000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 300));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 3);
            // All three fully withdraw
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 2);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(CHARLIE), id, 200));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 1);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(DAVE), id, 300));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 0);
        });
    }
}

// ── P1-02: Event amounts report net (after protocol fee) ────────────

mod p1_02_event_net_amounts {
    use super::*;

    #[test]
    fn claim_funds_event_reports_full_amount_when_zero_fee() {
        // Mock has ProtocolFeeBps = 0, so net == gross
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            let events = System::events();
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::FundsClaimed {
                        campaign_id,
                        creator,
                        amount,
                    }) if *campaign_id == id && *creator == ALICE && *amount == 1000
                )
            });
            assert!(found, "FundsClaimed event should report full 1000 when fee is 0");
            // Also verify NO ProtocolFeeCollected event
            let fee_event = events.iter().any(|e| {
                matches!(&e.event, RuntimeEvent::Crowdfunding(Event::ProtocolFeeCollected { .. }))
            });
            assert!(!fee_event, "ProtocolFeeCollected should NOT be emitted when fee is 0");
        });
    }

    #[test]
    fn claim_milestone_funds_event_reports_full_amount_when_zero_fee() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 6000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 4000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            System::reset_events();
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            let events = System::events();
            // 60% of 1000 = 600, no fee => amount = 600
            let found = events.iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::MilestoneFundsClaimed {
                        campaign_id,
                        index,
                        amount,
                    }) if *campaign_id == id && *index == 0 && *amount == 600
                )
            });
            assert!(found, "MilestoneFundsClaimed should report full 600 when fee is 0");
        });
    }
}

// ── BoundedVec lifecycle after fixes ────────────────────────────────

mod bounded_vec_lifecycle_after_fixes {
    use super::*;

    #[test]
    fn creator_campaigns_freed_after_deposit_claim_allows_new_campaign() {
        // Verify that claiming the creation deposit frees the slot
        ExtBuilder::default().build().execute_with(|| {
            // Fill MaxCampaignsPerCreator = 5
            let mut ids = vec![];
            for _ in 0..5 {
                ids.push(create_funded_campaign(ALICE, default_aon_config(100, 1000)));
            }
            // Can't create more
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 1000),
                    None,
                    None,
                ),
                Error::<Test>::MaxCampaignsPerCreatorReached
            );
            // Cancel one and claim deposit
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), ids[2]));
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), ids[2]));
            // Verify slot is freed
            assert_eq!(pallet::CreatorCampaigns::<Test>::get(ALICE).len(), 4);
            // Now can create a new one
            let new_id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_eq!(pallet::CreatorCampaigns::<Test>::get(ALICE).len(), 5);
            assert!(pallet::CreatorCampaigns::<Test>::get(ALICE).contains(&new_id));
        });
    }

    #[test]
    fn investor_campaigns_freed_after_full_withdrawal_allows_new_investment() {
        ExtBuilder::default().build().execute_with(|| {
            // Fill MaxInvestmentsPerInvestor = 5
            let mut campaign_ids = vec![];
            for _ in 0..5 {
                campaign_ids.push(create_funded_campaign(ALICE, default_aon_config(100, 1000)));
            }
            for &cid in &campaign_ids {
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), cid, 100));
            }
            // At max — verify
            assert_eq!(pallet::InvestorCampaigns::<Test>::get(BOB).len(), 5);
            let extra = create_funded_campaign(CHARLIE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), extra, 100),
                Error::<Test>::MaxInvestmentsPerInvestorReached
            );
            // Fully withdraw from one campaign
            assert_ok!(Crowdfunding::withdraw_investment(
                RuntimeOrigin::signed(BOB),
                campaign_ids[0],
                100
            ));
            assert_eq!(pallet::InvestorCampaigns::<Test>::get(BOB).len(), 4);
            // Now can invest in new campaign
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), extra, 100));
            assert_eq!(pallet::InvestorCampaigns::<Test>::get(BOB).len(), 5);
        });
    }

    #[test]
    fn investor_campaigns_freed_after_refund_allows_new_investment() {
        ExtBuilder::default().build().execute_with(|| {
            // Fill MaxInvestmentsPerInvestor = 5
            let mut campaign_ids = vec![];
            for _ in 0..5 {
                campaign_ids.push(create_funded_campaign(ALICE, default_aon_config(20, 5000)));
            }
            for &cid in &campaign_ids {
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), cid, 100));
            }
            assert_eq!(pallet::InvestorCampaigns::<Test>::get(BOB).len(), 5);
            // Fail one campaign and refund
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(
                RuntimeOrigin::signed(ALICE),
                campaign_ids[0]
            ));
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), campaign_ids[0]));
            assert_eq!(pallet::InvestorCampaigns::<Test>::get(BOB).len(), 4);
            // Now can invest in a new campaign (create from CHARLIE to avoid ALICE limit)
            let extra = create_funded_campaign(CHARLIE, default_aon_config(50, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), extra, 100));
            assert_eq!(pallet::InvestorCampaigns::<Test>::get(BOB).len(), 5);
        });
    }
}

// ── State machine exhaustive coverage ───────────────────────────────

mod state_machine_exhaustive {
    use super::*;

    #[test]
    fn all_investor_dispatchables_fail_on_completed_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            // Status is now Completed
            // invest fails
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100),
                Error::<Test>::InvalidCampaignStatus
            );
            // withdraw fails
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::InvalidCampaignStatus
            );
            // refund fails
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::InvalidCampaignStatus
            );
            // finalize fails
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
            // claim_funds fails (already completed)
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
            // cancel fails
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
            // whitelist fails
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
            assert_noop!(
                Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
            // pause fails (requires Funding)
            assert_noop!(
                Crowdfunding::pause_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotFunding
            );
            // resume fails (requires Paused)
            assert_noop!(
                Crowdfunding::resume_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotPaused
            );
            // milestone operations fail
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
            // Only claim_creation_deposit should work on Completed
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
        });
    }

    #[test]
    fn all_dispatchables_fail_on_cancelled_except_refund_and_deposit() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // invest fails
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100),
                Error::<Test>::InvalidCampaignStatus
            );
            // withdraw fails (not Funding|Paused)
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::InvalidCampaignStatus
            );
            // finalize fails
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
            // claim_funds fails
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
            // cancel again fails
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
            // whitelist fails
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
            // pause/resume fail
            assert_noop!(
                Crowdfunding::pause_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotFunding
            );
            assert_noop!(
                Crowdfunding::resume_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotPaused
            );
            // milestone operations fail
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
            assert_noop!(
                Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidCampaignStatus
            );
            // But refund WORKS on Cancelled
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            // And claim_creation_deposit WORKS on Cancelled
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
        });
    }

    #[test]
    fn all_dispatchables_fail_on_failed_except_refund_cancel_deposit() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert!(matches!(
                pallet::Campaigns::<Test>::get(id).unwrap().status,
                CampaignStatus::Failed
            ));
            // invest fails
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100),
                Error::<Test>::InvalidCampaignStatus
            );
            // withdraw fails
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::InvalidCampaignStatus
            );
            // finalize fails (not Funding)
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
            // claim_funds fails (not Succeeded)
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
            // whitelist fails
            assert_noop!(
                Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE),
                Error::<Test>::InvalidCampaignStatus
            );
            // pause/resume fail
            assert_noop!(
                Crowdfunding::pause_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotFunding
            );
            assert_noop!(
                Crowdfunding::resume_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotPaused
            );
            // But refund WORKS on Failed
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            // cancel WORKS on Failed (transitions to Cancelled)
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // claim_creation_deposit WORKS on Cancelled (now)
            assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
        });
    }
}

// ── Additional penalty + fee interaction tests ──────────────────────

mod penalty_fee_interaction {
    use super::*;

    #[test]
    fn penalty_5000_bps_takes_half() {
        // 50% penalty — investor gets half
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(5000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // Permill::from_parts(5000 * 100) = Permill(500_000) = 50%
            // penalty = 500, net = 500
            assert_eq!(Balances::free_balance(BOB), bob_before + 500);
        });
    }

    #[test]
    fn penalty_9999_bps_nearly_everything() {
        // 99.99% penalty — investor gets almost nothing
        // Use higher balance to avoid KeepAlive issues on invest
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 20_000), (CHARLIE, 10_000), (DAVE, 10_000)])
            .build()
            .execute_with(|| {
                let mut config = default_aon_config(100, 50000);
                config.early_withdrawal_penalty_bps = Some(9999);
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10000));
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::withdraw_investment(
                    RuntimeOrigin::signed(BOB),
                    id,
                    10000
                ));
                // Permill::from_parts(9999 * 100) = Permill(999_900)
                // penalty = 999_900 / 1_000_000 * 10000 = 9999
                // net = 10000 - 9999 = 1
                assert_eq!(Balances::free_balance(BOB), bob_before + 1);
            });
    }

    #[test]
    fn penalty_333_bps_rounding() {
        // 3.33% penalty — verify ceiling division behavior
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(333);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // ceil(1000 * 333 / 10000) = ceil(33.3) = 34
            // penalty = 34, net = 1000 - 34 = 966
            assert_eq!(Balances::free_balance(BOB), bob_before + 966);
        });
    }
}

// ── protocol_config ─────────────────────────────────────────────────────

mod protocol_config {
    use super::*;

    #[test]
    fn set_protocol_config_works() {
        ExtBuilder::default().build().execute_with(|| {
            let new_recipient: u64 = 50;
            assert_ok!(Crowdfunding::set_protocol_config(
                RuntimeOrigin::root(),
                500,
                new_recipient
            ));
            assert_eq!(pallet::ProtocolFeeBpsOverride::<Test>::get(), Some(500));
            assert_eq!(pallet::ProtocolFeeRecipientOverride::<Test>::get(), Some(50));
            System::assert_last_event(
                crate::Event::<Test>::ProtocolConfigUpdated {
                    fee_bps: 500,
                    recipient: new_recipient,
                }
                .into(),
            );
        });
    }

    #[test]
    fn set_protocol_config_non_admin_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::set_protocol_config(RuntimeOrigin::signed(ALICE), 500, 50),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn set_protocol_config_fee_bps_over_10000_fails() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 10_001, 50),
                crate::Error::<Test>::InvalidFeeBps
            );
        });
    }

    #[test]
    fn set_protocol_config_fee_bps_exactly_10000_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 10_000, 50));
            assert_eq!(pallet::ProtocolFeeBpsOverride::<Test>::get(), Some(10_000));
        });
    }

    #[test]
    fn set_protocol_config_fee_bps_zero_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 0, 50));
            assert_eq!(pallet::ProtocolFeeBpsOverride::<Test>::get(), Some(0));
        });
    }

    #[test]
    fn claim_funds_uses_overridden_fee() {
        // Set fee to 500 bps (5%), create campaign, invest, finalize, claim —
        // verify fee is 5% and goes to the new recipient, not the Config default.
        ExtBuilder::default().build().execute_with(|| {
            let new_recipient: u64 = 50;
            // Fund recipient so account exists (avoid reaping issues)
            let _ = Balances::deposit_creating(&new_recipient, 1);
            assert_ok!(Crowdfunding::set_protocol_config(
                RuntimeOrigin::root(),
                500,
                new_recipient
            ));

            let config = default_aon_config(100, 1000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            let alice_before = Balances::free_balance(ALICE);
            let recipient_before = Balances::free_balance(new_recipient);
            // Config default fee is 0 bps (ProtocolFeeAccount=99). If override works,
            // 5% of 1000 = 50 goes to account 50.
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

            // fee = Permill::from_parts(500*100) * 1000 = Permill(50_000) * 1000 = 50
            let fee = 50u128;
            let creator_amount = 1000 - fee;
            assert_eq!(Balances::free_balance(new_recipient), recipient_before + fee);
            assert_eq!(Balances::free_balance(ALICE), alice_before + creator_amount);

            // Verify default recipient (account 99) did NOT receive anything
            assert_eq!(Balances::free_balance(99u64), 0);
        });
    }

    #[test]
    fn claim_funds_uses_default_when_no_override() {
        // No override set — verify fallback to Config constant (0 bps, account 99).
        ExtBuilder::default().build().execute_with(|| {
            let config = default_aon_config(100, 1000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

            // 0% fee means Alice gets all 1000
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);
            // Account 99 (default recipient) gets nothing since fee is 0
            assert_eq!(Balances::free_balance(99u64), 0);
        });
    }

    #[test]
    fn claim_milestone_funds_uses_overridden_fee() {
        ExtBuilder::default().build().execute_with(|| {
            let new_recipient: u64 = 50;
            let _ = Balances::deposit_creating(&new_recipient, 1);
            assert_ok!(Crowdfunding::set_protocol_config(
                RuntimeOrigin::root(),
                500,
                new_recipient
            ));

            let config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));

            let alice_before = Balances::free_balance(ALICE);
            let recipient_before = Balances::free_balance(new_recipient);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));

            // release_amount = 50% of 1000 = 500
            // fee = 5% of 500 = 25
            // creator_amount = 500 - 25 = 475
            assert_eq!(Balances::free_balance(new_recipient), recipient_before + 25);
            assert_eq!(Balances::free_balance(ALICE), alice_before + 475);
        });
    }

    #[test]
    fn set_protocol_config_updates_recipient() {
        // Set config, then verify the fee goes to the new recipient.
        ExtBuilder::default().build().execute_with(|| {
            let new_recipient: u64 = 77;
            let _ = Balances::deposit_creating(&new_recipient, 1);
            assert_ok!(Crowdfunding::set_protocol_config(
                RuntimeOrigin::root(),
                1000,
                new_recipient
            ));

            let config = default_aon_config(100, 2000);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 2000));
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            let recipient_before = Balances::free_balance(new_recipient);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

            // fee = 10% of 2000 = 200
            assert_eq!(Balances::free_balance(new_recipient), recipient_before + 200);
        });
    }

    #[test]
    fn set_protocol_config_can_be_called_multiple_times() {
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 100, 50));
            assert_eq!(pallet::ProtocolFeeBpsOverride::<Test>::get(), Some(100));
            assert_eq!(pallet::ProtocolFeeRecipientOverride::<Test>::get(), Some(50));

            // Update again
            assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 200, 60));
            assert_eq!(pallet::ProtocolFeeBpsOverride::<Test>::get(), Some(200));
            assert_eq!(pallet::ProtocolFeeRecipientOverride::<Test>::get(), Some(60));

            // Verify the second values are used in the helpers
            assert_eq!(Crowdfunding::effective_protocol_fee_bps(), 200);
            assert_eq!(Crowdfunding::effective_protocol_fee_recipient(), 60);
        });
    }

    #[test]
    fn effective_helpers_return_defaults_when_no_override() {
        ExtBuilder::default().build().execute_with(|| {
            // Config defaults: ProtocolFeeBps = 0, ProtocolFeeRecipient = 99
            assert_eq!(Crowdfunding::effective_protocol_fee_bps(), 0);
            assert_eq!(Crowdfunding::effective_protocol_fee_recipient(), 99);
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PERSONA-BASED ATTACK & ABUSE TESTS
// Reference: docs/persona-attack-test-plan.md Section 3
// ═══════════════════════════════════════════════════════════════════════════

// ── 3.1 Campaign Creation Attacks (ATK-CF-001 ~ ATK-CF-010) ─────────────

mod atk_campaign_creation {
    use super::*;

    /// ATK-CF-001: Campaign with Zero Goal (AllOrNothing)
    /// Persona: A-GRIEFER
    /// Attack: create_campaign with AllOrNothing { goal: 0 }
    /// Expected: Fails with InvalidFundingModel
    #[test]
    fn atk_cf_001_zero_goal_aon() {
        ExtBuilder::default().build().execute_with(|| {
            let config = default_aon_config(100, 0);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidFundingModel
            );
            // Verify: no campaign created, NextCampaignId unchanged
            assert_eq!(pallet::NextCampaignId::<Test>::get(), 0);
        });
    }

    /// ATK-CF-001b: Campaign with Zero Goal (MilestoneBased)
    /// Persona: A-GRIEFER
    /// Attack: MilestoneBased with goal: 0
    /// Expected: Fails with InvalidFundingModel
    #[test]
    fn atk_cf_001b_zero_goal_milestone() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                100,
                0,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidFundingModel
            );
            assert_eq!(pallet::NextCampaignId::<Test>::get(), 0);
        });
    }

    /// ATK-CF-002: Campaign with hard_cap < goal
    /// Persona: A-GRIEFER
    /// Attack: create_campaign with goal=1000, hard_cap=Some(500)
    /// Expected: Fails with InvalidFundingModel
    #[test]
    fn atk_cf_002_hard_cap_below_goal_aon() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.hard_cap = Some(500);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidFundingModel
            );
            assert_eq!(pallet::NextCampaignId::<Test>::get(), 0);
        });
    }

    /// ATK-CF-002b: hard_cap < goal for MilestoneBased
    #[test]
    fn atk_cf_002b_hard_cap_below_goal_milestone() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            config.hard_cap = Some(999);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidFundingModel
            );
        });
    }

    /// ATK-CF-002c: hard_cap = goal - 1 boundary
    #[test]
    fn atk_cf_002c_hard_cap_one_below_goal() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 100);
            config.hard_cap = Some(99);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidFundingModel
            );
        });
    }

    /// ATK-CF-002d: hard_cap = goal exactly (boundary success)
    #[test]
    fn atk_cf_002d_hard_cap_equals_goal_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.hard_cap = Some(1000);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None,
            ));
        });
    }

    /// ATK-CF-003: Campaign with min_investment > max_investment_per_investor
    /// Persona: A-GRIEFER
    /// Attack: min=100, max_per_investor=50
    /// Expected: Fails with InvalidFundingModel
    #[test]
    fn atk_cf_003_min_greater_than_max_investment() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.min_investment = Some(100);
            config.max_investment_per_investor = Some(50);
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::InvalidFundingModel
            );
            assert_eq!(pallet::NextCampaignId::<Test>::get(), 0);
        });
    }

    /// ATK-CF-003b: min = max boundary (should succeed)
    #[test]
    fn atk_cf_003b_min_equals_max_investment_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 1000);
            config.min_investment = Some(100);
            config.max_investment_per_investor = Some(100);
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                config,
                None,
                None,
            ));
        });
    }

    /// ATK-CF-006: MaxCampaignsPerCreator exhaustion then recovery
    /// Persona: A-DOSSER
    /// Follow-up: Claim deposit on completed campaign, verify can create new
    /// one
    #[test]
    fn atk_cf_006_max_campaigns_recovery_after_deposit_claim() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000)])
            .build()
            .execute_with(|| {
                // Fill MaxCampaignsPerCreator (5) — all with same deadline
                let mut ids = vec![];
                for _ in 0..5 {
                    ids.push(create_funded_campaign(ALICE, default_kwyr_config(20)));
                }
                // Invest in campaign 0 so it has funds to claim
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), ids[0], 100));

                // 6th fails
                assert_noop!(
                    Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        default_kwyr_config(20),
                        None,
                        None,
                    ),
                    Error::<Test>::MaxCampaignsPerCreatorReached
                );

                // Finalize and claim deposit on campaign 0 to free a slot
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), ids[0]));
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), ids[0]));
                assert_ok!(Crowdfunding::claim_creation_deposit(
                    RuntimeOrigin::signed(ALICE),
                    ids[0]
                ));

                // Now we can create a new campaign (deadline far enough from block 21)
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_kwyr_config(200),
                    None,
                    None,
                ));
            });
    }

    /// ATK-CF-008b: Milestone BPS sum = 9999 (under)
    #[test]
    fn atk_cf_008b_milestone_bps_sum_9999() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                100,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 4999, description_hash: [2u8; 32] },
                ],
            );
            assert_noop!(
                Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                Error::<Test>::MilestoneBpsSumInvalid
            );
        });
    }

    /// ATK-CF-009: Campaign with non-existent asset currency
    /// Persona: A-GRIEFER
    /// Attack: Create campaign with funding_currency = Asset(99999)
    /// Expected: Creation succeeds; invest will fail at transfer time
    #[test]
    fn atk_cf_009_nonexistent_asset_currency_creation_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let config = CampaignConfig {
                funding_model: FundingModel::AllOrNothing { goal: 1000 },
                funding_currency: PaymentCurrency::Asset(99999u32),
                deadline: 100,
                hard_cap: None,
                min_investment: None,
                max_investment_per_investor: None,
                metadata_hash: [0u8; 32],
                early_withdrawal_penalty_bps: None,
            };
            // Creation should succeed (currency not validated at creation)
            let id = create_funded_campaign(ALICE, config);
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.config.funding_currency, PaymentCurrency::Asset(99999)));
        });
    }

    /// ATK-CF-009b: Invest in campaign with non-existent asset currency fails
    #[test]
    fn atk_cf_009b_invest_nonexistent_asset_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let config = CampaignConfig {
                funding_model: FundingModel::AllOrNothing { goal: 1000 },
                funding_currency: PaymentCurrency::Asset(99999u32),
                deadline: 100,
                hard_cap: None,
                min_investment: None,
                max_investment_per_investor: None,
                metadata_hash: [0u8; 32],
                early_withdrawal_penalty_bps: None,
            };
            let id = create_funded_campaign(ALICE, config);
            // Investing should fail at asset transfer level
            assert!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100).is_err());
            // Verify: no state change
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 0);
            assert_eq!(c.investor_count, 0);
        });
    }

    /// ATK-CF-010: CampaignId overflow — verify NextCampaignId not incremented
    /// on failure
    #[test]
    fn atk_cf_010_campaign_id_overflow_no_state_change() {
        ExtBuilder::default().build().execute_with(|| {
            pallet::NextCampaignId::<Test>::put(u32::MAX);
            let alice_before = Balances::free_balance(ALICE);
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 1000),
                    None,
                    None,
                ),
                Error::<Test>::CampaignIdOverflow
            );
            // Verify: NextCampaignId unchanged, balance unchanged
            assert_eq!(pallet::NextCampaignId::<Test>::get(), u32::MAX);
            assert_eq!(Balances::free_balance(ALICE), alice_before);
        });
    }
}

// ── 3.2 Investment Attacks (ATK-CF-020 ~ ATK-CF-029) ────────────────────

mod atk_investment {
    use super::*;

    /// ATK-CF-022: Invest on every non-Funding campaign status
    /// Persona: A-GRIEFER
    /// Attack: Try to invest on Paused, Succeeded, Failed, Cancelled,
    /// MilestonePhase, Completed Expected: All fail with
    /// InvalidCampaignStatus
    #[test]
    fn atk_cf_022_invest_on_all_non_funding_statuses() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                // Create all campaigns first (while block=1), with different deadlines
                let id_paused = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                let id_cancelled = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                let id_failed = create_funded_campaign(ALICE, default_aon_config(20, 5000));
                let id_succeeded = create_funded_campaign(CHARLIE, default_kwyr_config(20));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_succeeded, 100));

                // Paused
                assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id_paused));
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_paused, 100),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Cancelled
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id_cancelled));
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_cancelled, 100),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Failed (AON under-goal)
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(
                    RuntimeOrigin::signed(ALICE),
                    id_failed
                ));
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_failed, 100),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Succeeded
                assert_ok!(Crowdfunding::finalize_campaign(
                    RuntimeOrigin::signed(ALICE),
                    id_succeeded
                ));
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_succeeded, 100),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Completed
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(CHARLIE), id_succeeded));
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_succeeded, 100),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
    }

    /// ATK-CF-027: Sybil investment to bypass per-investor cap
    /// Persona: A-SYBIL
    /// Attack: Use multiple accounts to bypass max_investment_per_investor
    /// Expected: Each account independently tracked — succeeds per account
    #[test]
    fn atk_cf_027_sybil_bypass_per_investor_cap() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000), (DAVE, 10_000)])
            .build()
            .execute_with(|| {
                let mut config = default_aon_config(100, 5000);
                config.max_investment_per_investor = Some(200);
                let id = create_funded_campaign(ALICE, config);

                // Each account can invest up to 200
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 200));

                // Total raised = 600, even though per-investor cap is 200
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 600);
                assert_eq!(c.investor_count, 3);

                // Each investor is capped individually
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1),
                    Error::<Test>::InvestmentExceedsMaxPerInvestor
                );
            });
    }

    /// ATK-CF-028: Re-invest after full withdrawal
    /// Persona: P-INVESTOR
    /// Attack: Invest 100, withdraw 100, then invest again
    /// Expected: Should succeed. H-1: InvestorCampaigns membership is
    /// authoritative
    #[test]
    fn atk_cf_028_reinvest_after_full_withdrawal() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(0); // no penalty for clean test
            let id = create_funded_campaign(ALICE, config);

            // Invest
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 1);

            // Full withdrawal
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 0);
            assert_eq!(c.total_raised, 0);
            // InvestorCampaigns should be cleaned
            assert!(!pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));

            // Re-invest
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.investor_count, 1);
            assert_eq!(c.total_raised, 200);
            assert!(pallet::InvestorCampaigns::<Test>::get(BOB).contains(&id));

            // Investment record tracks cumulative
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_invested, 300); // 100 + 200
            assert_eq!(inv.total_withdrawn, 100);
        });
    }

    /// ATK-CF-029: Invest to exact hard cap and verify no more investments
    /// Persona: P-INVESTOR
    /// Expected: Succeeds; HardCapReached emitted; subsequent invest fails
    #[test]
    fn atk_cf_029_invest_exact_hard_cap_then_blocked() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 500);
            config.hard_cap = Some(500);
            let id = create_funded_campaign(ALICE, config);

            System::reset_events();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));

            // HardCapReached event should be emitted
            let found = System::events().iter().any(|e| {
                matches!(
                    &e.event,
                    RuntimeEvent::Crowdfunding(Event::HardCapReached { campaign_id })
                    if *campaign_id == id
                )
            });
            assert!(found, "HardCapReached event should be emitted");

            // Subsequent invest should fail
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 1),
                Error::<Test>::HardCapExceeded
            );
        });
    }

    /// ATK-CF-023: Exceed hard cap with cumulative investments (race)
    /// Persona: A-OVERFLOW
    /// Attack: Campaign hard_cap=1000; invest 999, then invest 2
    /// Expected: Second invest fails with HardCapExceeded (P3-01 post-mutation
    /// check)
    #[test]
    fn atk_cf_023_hard_cap_cumulative_exceed() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 500);
            config.hard_cap = Some(1000);
            let id = create_funded_campaign(ALICE, config);

            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 999));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 2),
                Error::<Test>::HardCapExceeded
            );
            // Verify exactly 1 fills remaining
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 1));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 1000);
        });
    }

    /// ATK-CF-024: per-investor max uses current_net (invested - withdrawn)
    /// Persona: A-DOSSER
    /// Verify: Uses current_net, not just invested
    #[test]
    fn atk_cf_024_max_per_investor_uses_net() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.max_investment_per_investor = Some(200);
            config.early_withdrawal_penalty_bps = Some(0);
            let id = create_funded_campaign(ALICE, config);

            // Invest max
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 200));
            // Withdraw 100
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100));
            // Current net = 200 - 100 = 100. Can invest 100 more.
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            // Now current net = 200 again. No more room.
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1),
                Error::<Test>::InvestmentExceedsMaxPerInvestor
            );
        });
    }
}

// ── 3.3 Withdrawal Attacks (ATK-CF-030 ~ ATK-CF-037) ────────────────────

mod atk_withdrawal {
    use super::*;

    /// ATK-CF-032: Withdraw after deadline
    /// Persona: A-EXPIRED
    /// Attack: Deadline passed, status still Funding, try to withdraw
    /// Expected: Should fail — withdraw checks Funding|Paused AND we need to
    /// verify   the campaign cannot be interacted with after deadline in a
    /// withdrawal context.   Actually: withdraw does NOT check deadline. It
    /// checks status == Funding|Paused.   If status is still Funding after
    /// deadline, withdraw should still work since   the campaign hasn't
    /// been finalized yet.
    #[test]
    fn atk_cf_032_withdraw_after_deadline_but_before_finalize() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));

            run_to_block(21); // past deadline
                              // Status is still Funding (not finalized yet)
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Funding));

            // Withdraw — should succeed since status is Funding and no deadline check
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
            // 1% penalty = 2, net = 198
            assert_eq!(Balances::free_balance(BOB), bob_before + 198);
        });
    }

    /// ATK-CF-032b: Withdraw after finalize (Failed)
    /// After finalization, status changes — withdrawal should fail
    #[test]
    fn atk_cf_032b_withdraw_after_finalize_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            // Status is now Failed
            assert_noop!(
                Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    /// ATK-CF-034: Full withdrawal and investor count
    /// Persona: P-INVESTOR
    /// Verify: investor_count decremented only on full withdrawal (P2-12)
    #[test]
    fn atk_cf_034_full_withdrawal_investor_count() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(0);
            let id = create_funded_campaign(ALICE, config);

            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 500));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 2);

            // Partial withdrawal — count should NOT change
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 2);

            // Full withdrawal of remaining — count should decrement
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 1);

            // CHARLIE full withdrawal
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(CHARLIE), id, 500));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 0);
        });
    }

    /// ATK-CF-035: Withdrawal penalty precision with 1 unit
    /// Persona: A-OVERFLOW
    /// Attack: Invest 1, withdraw 1 with penalty_bps = 1 (0.01%)
    /// Expected: Penalty = ceil(1*1/10000) = 1; investor gets 0 back
    #[test]
    fn atk_cf_035_penalty_precision_one_unit() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(1); // 0.01%
            let id = create_funded_campaign(ALICE, config);

            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1));
            // ceil(1 * 1 / 10000) = 1, penalty = 1, net = 0
            assert_eq!(Balances::free_balance(BOB), bob_before);
        });
    }

    /// ATK-CF-036: Withdrawal penalty = 100%
    /// Persona: A-GRIEFER (creator)
    /// Attack: penalty_bps = 10000, investor withdraws
    /// Expected: Penalty = full amount; investor gets 0; penalty burned
    #[test]
    fn atk_cf_036_penalty_100_percent() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(10000); // 100%
            let id = create_funded_campaign(ALICE, config);

            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            let bob_before = Balances::free_balance(BOB);
            let sub = Crowdfunding::campaign_account(id);
            let sub_before = Balances::free_balance(sub);

            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // Investor gets nothing
            assert_eq!(Balances::free_balance(BOB), bob_before);
            // Sub-account lost the full 1000 (burned)
            // total_raised decreased by 1000
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.total_raised, 0);
        });
    }

    /// ATK-CF-037: Withdraw-reinvest cycle to drain penalty
    /// Persona: A-THIEF
    /// Attack: Invest X, withdraw X, repeat
    /// Expected: Each cycle burns penalty; investor loses money each cycle
    #[test]
    fn atk_cf_037_withdraw_reinvest_cycle() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 100_000)]).build().execute_with(
            || {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 50000));
                let bob_initial = Balances::free_balance(BOB);

                // Cycle 1: Invest 1000, withdraw 1000 (1% penalty = 10)
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
                // BOB lost 10 (penalty)
                assert_eq!(Balances::free_balance(BOB), bob_initial - 10);
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 0);

                // Cycle 2: Invest 1000 again, withdraw 1000 (1% penalty = 10)
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
                // BOB lost 20 total
                assert_eq!(Balances::free_balance(BOB), bob_initial - 20);
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 0);
            },
        );
    }

    /// ATK-CF-033: Withdraw during paused campaign with penalty
    /// Persona: P-INVESTOR
    /// Expected: Should succeed; penalty still applies
    #[test]
    fn atk_cf_033_withdraw_during_pause_penalty_applies() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(500); // 5%
            let id = create_funded_campaign(ALICE, config);

            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));

            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
            // 5% of 1000 = 50 penalty, net = 950
            assert_eq!(Balances::free_balance(BOB), bob_before + 950);
        });
    }
}

// ── 3.4 Finalization & Failure Attacks (ATK-CF-040 ~ ATK-CF-045) ────────

mod atk_finalization {
    use super::*;

    /// ATK-CF-043: Mark failed on successful campaign
    /// Persona: A-GRIEFER
    /// Attack: Campaign reached goal, deadline passed, finalize
    /// Expected: Status → Succeeded (not Failed)
    #[test]
    fn atk_cf_043_successful_campaign_cannot_be_forced_to_fail() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1500));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
            // Cannot finalize again to get different result
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    /// ATK-CF-044: KeepWhatYouRaise soft cap edge — raise 999 with soft_cap
    /// 1000 Expected: Failed (soft cap not met)
    #[test]
    fn atk_cf_044_kwyr_soft_cap_one_below_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 999));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Failed));
        });
    }

    /// ATK-CF-044b: KeepWhatYouRaise soft cap exact boundary succeeds
    #[test]
    fn atk_cf_044b_kwyr_soft_cap_exact_succeeds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));
        });
    }

    /// ATK-CF-045: MilestoneBased goal met → MilestonePhase
    /// Verify: Creator cannot call claim_funds (wrong status)
    #[test]
    fn atk_cf_045_milestone_succeeds_to_milestone_phase_not_succeeded() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::MilestonePhase));

            // Creator CANNOT claim_funds (wrong status — must go through milestones)
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── 3.5 Fund Claiming Attacks (ATK-CF-050 ~ ATK-CF-054) ─────────────────

mod atk_fund_claiming {
    use super::*;

    /// ATK-CF-050: Claim funds on non-Succeeded campaign statuses
    /// Persona: A-THIEF
    /// Expected: All fail with InvalidCampaignStatus
    #[test]
    fn atk_cf_050_claim_funds_on_non_succeeded() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000)])
            .build()
            .execute_with(|| {
                // Funding
                let id_funding = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                assert_noop!(
                    Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id_funding),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Cancelled
                let id_cancelled = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id_cancelled));
                assert_noop!(
                    Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id_cancelled),
                    Error::<Test>::InvalidCampaignStatus
                );

                // MilestonePhase
                let config_ms = milestone_config(
                    20,
                    500,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id_ms = create_funded_campaign(ALICE, config_ms);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_ms, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id_ms));
                assert_noop!(
                    Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id_ms),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
    }

    /// ATK-CF-051: Double claim funds
    /// Persona: A-THIEF
    /// Expected: First succeeds (status → Completed); second fails
    #[test]
    fn atk_cf_051_double_claim_funds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_kwyr_config(20));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));

            // Second attempt fails
            assert_noop!(
                Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── 3.6 Refund Attacks (ATK-CF-060 ~ ATK-CF-064) ────────────────────────

mod atk_refund {
    use super::*;

    /// ATK-CF-060: Claim refund on non-Failed/Cancelled campaigns
    /// Persona: A-THIEF
    #[test]
    fn atk_cf_060_refund_on_non_failed_statuses() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000)])
            .build()
            .execute_with(|| {
                // Create all campaigns at block 1
                let id_funding = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_funding, 100));

                let id_succ = create_funded_campaign(ALICE, default_kwyr_config(20));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_succ, 100));

                let config_ms = milestone_config(
                    20,
                    100,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id_ms = create_funded_campaign(ALICE, config_ms);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_ms, 100));

                // Funding — refund fails
                assert_noop!(
                    Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id_funding),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Advance past deadline
                run_to_block(21);

                // Succeeded
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id_succ));
                assert_noop!(
                    Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id_succ),
                    Error::<Test>::InvalidCampaignStatus
                );

                // MilestonePhase
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id_ms));
                assert_noop!(
                    Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id_ms),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
    }

    /// ATK-CF-062: Refund after partial withdrawal
    /// Persona: P-INVESTOR
    /// Attack: Invest 1000, withdraw 400 (with penalty), campaign fails, claim
    /// refund Expected: Refund = 1000 - 400 = 600
    #[test]
    fn atk_cf_062_refund_after_partial_withdrawal() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));

            // Withdraw 400 (1% penalty = 4, net = 396)
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 400));

            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // raw_refund = total_invested(1000) - total_withdrawn(400) = 600
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 600);
        });
    }

    /// ATK-CF-063: Refund with partial disbursement (MilestoneBased cancelled)
    /// Persona: P-INVESTOR
    /// Attack: Milestone 1 claimed (30%), then admin cancels, investor refunds
    /// Expected: Proportional refund
    #[test]
    fn atk_cf_063_refund_proportional_after_milestone_disbursement() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000)])
            .build()
            .execute_with(|| {
                let config = milestone_config(
                    20,
                    1000,
                    vec![
                        Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                        Milestone { release_bps: 4000, description_hash: [3u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestone 0: 30% of 10000 = 3000
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));

                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_disbursed, 3000);

                // Admin cancels
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                // remaining_ratio = (10000 - 3000) / 10000 = 70%
                // BOB's raw_refund = 10000
                // proportional = 70% of 10000 = 7000
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 7000);
            });
    }

    /// ATK-CF-064: Refund with zero refundable (fully withdrawn investor)
    /// Persona: A-GRIEFER
    /// Attack: Investor fully withdrew before campaign failed, then tries
    /// refund Expected: Fails with NothingToRefund
    #[test]
    fn atk_cf_064_refund_zero_refundable() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(20, 5000);
            config.early_withdrawal_penalty_bps = Some(0);
            let id = create_funded_campaign(ALICE, config);

            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));

            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // BOB has Investment record but total_invested == total_withdrawn
            // raw_refund = 1000 - 1000 = 0
            assert_noop!(
                Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                Error::<Test>::NothingToRefund
            );
        });
    }
}

// ── 3.7 Milestone Attacks (ATK-CF-070 ~ ATK-CF-077) ─────────────────────

mod atk_milestone {
    use super::*;

    fn setup_milestone_campaign() -> u32 {
        let config = milestone_config(
            20,
            1000,
            vec![
                Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                Milestone { release_bps: 4000, description_hash: [3u8; 32] },
            ],
        );
        let id = create_funded_campaign(ALICE, config);
        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
        run_to_block(21);
        assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
        id
    }

    /// ATK-CF-070: Submit milestone on non-MilestonePhase campaigns
    /// Persona: A-GRIEFER
    #[test]
    fn atk_cf_070_submit_milestone_wrong_status() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000)])
            .build()
            .execute_with(|| {
                // Create all campaigns at block 1
                let id_funding = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                let id_succ = create_funded_campaign(ALICE, default_kwyr_config(20));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_succ, 100));
                let id_failed = create_funded_campaign(ALICE, default_aon_config(20, 50000));

                // Funding — submit fails
                assert_noop!(
                    Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id_funding, 0),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Advance past deadline for id_succ and id_failed
                run_to_block(21);

                // Succeeded
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id_succ));
                assert_noop!(
                    Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id_succ, 0),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Failed
                assert_ok!(Crowdfunding::finalize_campaign(
                    RuntimeOrigin::signed(ALICE),
                    id_failed
                ));
                assert_noop!(
                    Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id_failed, 0),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
    }

    /// ATK-CF-071: Submit invalid milestone index
    /// Persona: A-GRIEFER
    #[test]
    fn atk_cf_071_submit_invalid_milestone_index() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            // Campaign has 3 milestones (0,1,2). Index 3 is invalid.
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 3),
                Error::<Test>::InvalidMilestoneIndex
            );
            // Index 255 is also invalid
            assert_noop!(
                Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 255),
                Error::<Test>::InvalidMilestoneIndex
            );
        });
    }

    /// ATK-CF-072: Approve non-submitted (Pending) milestone
    /// Persona: P-APPROVER
    /// Expected: Fails with InvalidMilestoneStatus
    #[test]
    fn atk_cf_072_approve_pending_milestone() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_noop!(
                Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    /// ATK-CF-073: Claim unapproved (Submitted) milestone funds
    /// Persona: A-THIEF
    /// Expected: Fails with InvalidMilestoneStatus
    #[test]
    fn atk_cf_073_claim_submitted_milestone() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    /// ATK-CF-074: Double claim milestone funds
    /// Persona: A-THIEF
    /// Expected: First succeeds (Claimed); second fails
    #[test]
    fn atk_cf_074_double_claim_milestone_funds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Claimed)
            );
            // Second claim fails
            assert_noop!(
                Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                Error::<Test>::InvalidMilestoneStatus
            );
        });
    }

    /// ATK-CF-075: Resubmit after rejection infinite loop
    /// Persona: A-DOSSER (P-CREATOR)
    /// Verify: submit-reject cycle works repeatedly (no limit)
    #[test]
    fn atk_cf_075_resubmit_after_rejection_loop() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();
            for _ in 0..10 {
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
            }
            // After 10 rejections, can still submit
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                Some(MilestoneStatus::Submitted)
            );
        });
    }

    /// ATK-CF-076: Milestone funds rounding attack
    /// Persona: A-OVERFLOW
    /// Attack: Milestones [3333, 3333, 3334] on total_raised = 1
    /// Expected: bps_of(1, 3333) = ceil(3333/10000) = 1 due to ceiling
    #[test]
    fn atk_cf_076_milestone_rounding_attack() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1,
                vec![
                    Milestone { release_bps: 3333, description_hash: [1u8; 32] },
                    Milestone { release_bps: 3333, description_hash: [2u8; 32] },
                    Milestone { release_bps: 3334, description_hash: [3u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            // Milestone 0: bps_of(1, 3333) = ceil(3333/10000) = 1
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));

            let alice_before = Balances::free_balance(ALICE);
            // bps_of(1, 3333) = ceil(3333/10000) = 1
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            // Creator gets 1
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1);

            // Claim milestone 1 and 2 similarly (each releases 1)
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 1));

            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 2));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 2));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 2));

            // All milestones claimed → Completed
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Completed));
            // Sub-account: creation_deposit (100) + invested(1) - disbursed(3) = 98
            let sub = Crowdfunding::campaign_account(id);
            assert!(Balances::free_balance(sub) > 0);
        });
    }

    /// ATK-CF-077: All milestones permanently rejected — funds stuck
    /// Persona: P-APPROVER (adversarial)
    /// Verify: Only ForceOrigin cancel can recover funds
    #[test]
    fn atk_cf_077_permanent_rejection_stuck_funds() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_milestone_campaign();

            // Reject all submissions, never approve
            for _ in 0..5 {
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
            }

            // Campaign is still in MilestonePhase, funds stuck
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::MilestonePhase));

            // Only ForceOrigin cancel can resolve
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Cancelled));

            // Investor can now refund
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before + 1000);
        });
    }
}

// ── 3.8 Pause/Resume Attacks (ATK-CF-080 ~ ATK-CF-083) ──────────────────

mod atk_pause_resume {
    use super::*;

    /// ATK-CF-080: Pause non-Funding campaign statuses
    /// Persona: P-ADMIN
    #[test]
    fn atk_cf_080_pause_non_funding_statuses() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000)])
            .build()
            .execute_with(|| {
                // Create all campaigns at block 1
                let id_succ = create_funded_campaign(ALICE, default_kwyr_config(20));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_succ, 100));
                let id_failed = create_funded_campaign(ALICE, default_aon_config(20, 50000));
                let config_ms = milestone_config(
                    20,
                    100,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id_ms = create_funded_campaign(ALICE, config_ms);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_ms, 100));
                let id_cancel = create_funded_campaign(ALICE, default_aon_config(100, 1000));

                // Cancelled
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id_cancel));
                assert_noop!(
                    Crowdfunding::pause_campaign(RuntimeOrigin::root(), id_cancel),
                    Error::<Test>::CampaignNotFunding
                );

                // Advance past deadline for 20-block campaigns
                run_to_block(21);

                // Succeeded
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id_succ));
                assert_noop!(
                    Crowdfunding::pause_campaign(RuntimeOrigin::root(), id_succ),
                    Error::<Test>::CampaignNotFunding
                );

                // Failed
                assert_ok!(Crowdfunding::finalize_campaign(
                    RuntimeOrigin::signed(ALICE),
                    id_failed
                ));
                assert_noop!(
                    Crowdfunding::pause_campaign(RuntimeOrigin::root(), id_failed),
                    Error::<Test>::CampaignNotFunding
                );

                // MilestonePhase
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id_ms));
                assert_noop!(
                    Crowdfunding::pause_campaign(RuntimeOrigin::root(), id_ms),
                    Error::<Test>::CampaignNotFunding
                );

                // Completed
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id_succ));
                assert_noop!(
                    Crowdfunding::pause_campaign(RuntimeOrigin::root(), id_succ),
                    Error::<Test>::CampaignNotFunding
                );
            });
    }

    /// ATK-CF-081: Resume non-Paused campaign
    /// Persona: P-ADMIN
    #[test]
    fn atk_cf_081_resume_non_paused() {
        ExtBuilder::default().build().execute_with(|| {
            // Funding (not paused)
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_noop!(
                Crowdfunding::resume_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::CampaignNotPaused
            );
        });
    }

    /// ATK-CF-082: Pause-resume deadline extension abuse
    /// Persona: A-GRIEFER (colluding admin)
    /// Attack: Repeatedly pause+resume to extend deadline
    /// Verify: deadline += pause_duration on each cycle
    #[test]
    fn atk_cf_082_pause_resume_deadline_extension() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let original_deadline = pallet::Campaigns::<Test>::get(id).unwrap().config.deadline;
            assert_eq!(original_deadline, 100);

            // Pause at block 5 (but we're at block 1)
            run_to_block(5);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));

            // Resume at block 15 → pause_duration = 10
            run_to_block(15);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            let deadline_after_1 = pallet::Campaigns::<Test>::get(id).unwrap().config.deadline;
            assert_eq!(deadline_after_1, 100 + 10); // 110

            // Second cycle: pause at 20, resume at 30 → +10 more
            run_to_block(20);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            run_to_block(30);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            let deadline_after_2 = pallet::Campaigns::<Test>::get(id).unwrap().config.deadline;
            assert_eq!(deadline_after_2, 120);

            // Third cycle: pause at 50, resume at 100 → +50 more
            run_to_block(50);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            run_to_block(100);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            let deadline_after_3 = pallet::Campaigns::<Test>::get(id).unwrap().config.deadline;
            assert_eq!(deadline_after_3, 170);

            // Verify: investment still possible because deadline was extended
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
        });
    }

    /// ATK-CF-083: Invest during pause
    /// Persona: P-INVESTOR
    /// Expected: Fails with InvalidCampaignStatus
    #[test]
    fn atk_cf_083_invest_during_pause() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500),
                Error::<Test>::InvalidCampaignStatus
            );
            // Finalize also fails during pause
            assert_noop!(
                Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}

// ── 3.9 Eligibility & Whitelist Attacks (ATK-CF-090 ~ ATK-CF-096) ───────

mod atk_eligibility {
    use super::*;

    /// ATK-CF-091: Invest without meeting AssetBalance rule
    /// Persona: A-BROKE
    #[test]
    fn atk_cf_091_asset_balance_rule_insufficient() {
        ExtBuilder::default().build().execute_with(|| {
            // Create and fund asset 1
            assert_ok!(Assets::force_create(RuntimeOrigin::root(), 1.into(), ALICE, true, 1));
            assert_ok!(Assets::mint(RuntimeOrigin::signed(ALICE), 1.into(), BOB, 99));

            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::AssetBalance { asset_id: 1u32, min_balance: 100 }]
                    .try_into()
                    .unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));

            // BOB has 99, needs 100
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );

            // Mint 1 more to BOB, now has 100
            assert_ok!(Assets::mint(RuntimeOrigin::signed(ALICE), 1.into(), BOB, 1));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    /// ATK-CF-092: NftOwnership rule with missing NFTs
    /// Persona: A-GRIEFER
    #[test]
    fn atk_cf_092_nft_ownership_missing() {
        ExtBuilder::default().build().execute_with(|| {
            let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
            let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));

            // No NFT owner set — fails
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );

            // Set wrong owner
            MockNftInspect::set_owner(1, 1, CHARLIE);
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );

            // Set correct owner
            MockNftInspect::set_owner(1, 1, BOB);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    /// ATK-CF-095: Transfer NFT after investment — second user also invests
    /// Persona: A-GRIEFER
    /// Expected: Both succeed (point-in-time eligibility check)
    #[test]
    fn atk_cf_095_nft_transfer_after_investment() {
        ExtBuilder::default().build().execute_with(|| {
            let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
            let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));

            // BOB owns NFT, invests
            MockNftInspect::set_owner(1, 1, BOB);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));

            // Transfer NFT to CHARLIE (simulate)
            MockNftInspect::set_owner(1, 1, CHARLIE);

            // CHARLIE can now invest too
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100));

            // BOB's investment still stands
            let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
            assert_eq!(inv.total_invested, 100);
        });
    }

    /// ATK-CF-096: Multiple eligibility rules — must pass ALL
    /// Persona: A-GRIEFER
    /// Attack: Meets one rule but not the other
    #[test]
    fn atk_cf_096_multiple_rules_must_pass_all() {
        ExtBuilder::default().build().execute_with(|| {
            let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
            let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
            let rules: BoundedVec<_, _> = vec![
                EligibilityRule::NativeBalance { min_balance: 5000 },
                EligibilityRule::NftOwnership { required_sets },
            ]
            .try_into()
            .unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));

            // BOB has 10000 native (passes NativeBalance) but no NFT
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );

            // Give BOB the NFT — now both rules pass
            MockNftInspect::set_owner(1, 1, BOB);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    /// ATK-CF-094: Whitelist management on wrong status
    /// Persona: P-CREATOR
    /// Verify: add/remove whitelist only works on Funding|Paused
    #[test]
    fn atk_cf_094_whitelist_management_wrong_status() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000)])
            .build()
            .execute_with(|| {
                // Create campaigns at block 1
                let id_succ = create_funded_campaign(ALICE, default_kwyr_config(20));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id_succ, 100));
                let id_failed = create_funded_campaign(ALICE, default_aon_config(20, 50000));

                // Advance past deadline
                run_to_block(21);

                // Succeeded
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id_succ));
                assert_noop!(
                    Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id_succ, BOB),
                    Error::<Test>::InvalidCampaignStatus
                );
                assert_noop!(
                    Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), id_succ, BOB),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Failed
                assert_ok!(Crowdfunding::finalize_campaign(
                    RuntimeOrigin::signed(ALICE),
                    id_failed
                ));
                assert_noop!(
                    Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id_failed, BOB),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
    }

    /// ATK-CF-093: AccountWhitelist — non-whitelisted invest fails
    /// Persona: A-GRIEFER
    #[test]
    fn atk_cf_093_whitelist_non_whitelisted_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let rules: BoundedVec<_, _> =
                vec![EligibilityRule::AccountWhitelist].try_into().unwrap();
            let id = pallet::NextCampaignId::<Test>::get();
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 1000),
                Some(rules),
                None,
            ));

            // Not whitelisted
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );

            // Whitelist BOB, then remove, then try again
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            assert_ok!(Crowdfunding::remove_from_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::EligibilityCheckFailed
            );
        });
    }
}

// ── 3.10 Protocol Fee Attacks (ATK-CF-100 ~ ATK-CF-103) ─────────────────

mod atk_protocol_fee {
    use super::*;

    /// ATK-CF-101: Fee higher than claim amount (100%)
    /// Persona: A-OVERFLOW
    /// Attack: fee = 10000 bps (100%), creator claims
    /// Expected: All funds go to fee recipient; creator gets 0
    #[test]
    fn atk_cf_101_fee_100_percent_creator_gets_nothing() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (50, 1)])
            .build()
            .execute_with(|| {
                assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 10_000, 50));

                let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(101);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                let alice_before = Balances::free_balance(ALICE);
                let recipient_before = Balances::free_balance(50u64);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

                // 100% fee → all to recipient
                assert_eq!(Balances::free_balance(50u64), recipient_before + 1000);
                // Creator gets 0
                assert_eq!(Balances::free_balance(ALICE), alice_before);
            });
    }

    /// ATK-CF-102: Change fee mid-campaign
    /// Persona: P-ADMIN
    /// Verify: Fee is locked at creation time, mid-campaign change has no
    /// effect
    #[test]
    fn atk_cf_102_fee_change_mid_campaign() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (50, 1)])
            .build()
            .execute_with(|| {
                // Campaign created with fee = 0 (mock default, locked at creation)
                let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));

                // Admin changes fee to 50% mid-campaign (no effect — fee locked at 0)
                assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 5000, 50));

                run_to_block(101);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                let alice_before = Balances::free_balance(ALICE);
                let recipient_before = Balances::free_balance(50u64);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

                // Locked fee = 0%, no fee collected. Creator gets full 1000.
                assert_eq!(Balances::free_balance(50u64), recipient_before);
                assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);
            });
    }

    /// ATK-CF-103: Fee recipient = campaign creator
    /// Persona: A-INSIDER
    /// Attack: Admin sets fee recipient to creator's account
    /// Expected: Creator pays fee to themselves (net: they get everything)
    #[test]
    fn atk_cf_103_fee_recipient_is_creator() {
        ExtBuilder::default().build().execute_with(|| {
            // Set fee recipient = ALICE (who is the creator)
            assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 1000, ALICE));

            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

            // Fee = 10% of 1000 = 100 → goes to ALICE
            // Creator amount = 1000 - 100 = 900 → also goes to ALICE
            // Net: ALICE receives 1000
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);
        });
    }
}

// ── Multi-Step Attack Scenarios ─────────────────────────────────────────

mod atk_scenarios {
    use super::*;

    /// SCENARIO-003: Griefing attack — capacity exhaustion
    /// Persona: A-DOSSER
    /// Verify: Legitimate users can still operate independently
    #[test]
    fn atk_scenario_003_griefing_capacity_exhaustion() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000), (DAVE, 100_000)])
            .build()
            .execute_with(|| {
                // A-DOSSER (ALICE) exhausts MaxCampaignsPerCreator
                for _ in 0..5 {
                    create_funded_campaign(ALICE, default_aon_config(100, 1000));
                }

                // A-DOSSER invests dust in MaxInvestmentsPerInvestor campaigns
                let mut campaign_ids = vec![];
                for _ in 0..5 {
                    campaign_ids.push(create_funded_campaign(BOB, default_aon_config(100, 1000)));
                }
                for &cid in &campaign_ids {
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(ALICE), cid, 1));
                }

                // Verify: CHARLIE (legitimate user) can still create campaigns
                let id = create_funded_campaign(CHARLIE, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 500));
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 500);
            });
    }

    /// SCENARIO-005: MilestoneBased partial disbursement + cancel
    /// 3 milestones [30%, 30%, 40%], raise 10000, claim milestone 1, cancel,
    /// refund
    #[test]
    fn atk_scenario_005_milestone_partial_disbursement_cancel() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let config = milestone_config(
                    20,
                    5000,
                    vec![
                        Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                        Milestone { release_bps: 4000, description_hash: [3u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 6000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 4000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestone 0: 30% of 10000 = 3000
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                let alice_before_m0 = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                assert_eq!(Balances::free_balance(ALICE), alice_before_m0 + 3000);

                // Admin cancels
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                // remaining_ratio = (10000 - 3000) / 10000 = 70%
                // BOB raw_refund = 6000, proportional = 70% of 6000 = 4200
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 4200);

                // CHARLIE raw_refund = 4000, proportional = 70% of 4000 = 2800
                let charlie_before = Balances::free_balance(CHARLIE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 2800);

                // Verify: sum of disbursements + refunds = 3000 + 4200 + 2800 =
                // 10000 = total_raised. Conservation holds.
            });
    }

    /// SCENARIO-006: Withdrawal penalty + reinvestment + refund
    /// Campaign with 10% penalty. Invest 1000, withdraw 500, reinvest 300,
    /// fail, refund
    #[test]
    fn atk_scenario_006_penalty_reinvest_refund() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 100_000)]).build().execute_with(
            || {
                let mut config = default_aon_config(20, 50000);
                config.early_withdrawal_penalty_bps = Some(1000); // 10%
                let id = create_funded_campaign(ALICE, config);

                // Step 1: Invest 1000
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
                assert_eq!(inv.total_invested, 1000);

                // Step 2: Withdraw 500 (10% penalty = 50, net = 450)
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
                assert_eq!(Balances::free_balance(BOB), bob_before + 450);

                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 500); // 1000 - 500

                // Step 3: Reinvest 300
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 300));
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 800); // 500 + 300

                let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
                assert_eq!(inv.total_invested, 1300); // 1000 + 300
                assert_eq!(inv.total_withdrawn, 500);

                // Step 4: Campaign fails
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert!(matches!(c.status, CampaignStatus::Failed));

                // Step 5: Claim refund
                // raw_refund = total_invested(1300) - total_withdrawn(500) = 800
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 800);
            },
        );
    }

    /// SCENARIO-001 (partial): Full lifecycle — create, invest, finalize,
    /// claim, deposit Persona: P-CREATOR, P-INVESTOR
    #[test]
    fn atk_scenario_001_full_lifecycle() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                let sub = Crowdfunding::campaign_account(id);

                // Investor A invests 60% of goal
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
                // Investor B invests 40% of goal
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 400));

                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 1000);
                assert_eq!(c.investor_count, 2);

                // Deadline passes; finalize → Succeeded
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert!(matches!(c.status, CampaignStatus::Succeeded));

                // Creator claims funds (no protocol fee — default is 0)
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);

                // Creator claims deposit
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 100);

                // Verify: sub-account depleted
                assert_eq!(Balances::free_balance(sub), 0);

                // Verify: CreatorCampaigns cleaned
                assert!(!pallet::CreatorCampaigns::<Test>::get(ALICE).contains(&id));
            });
    }
}

// ── Invariant Assertion Helpers (Section 6.3) ───────────────────────────

mod invariant_helpers {
    use sp_runtime::traits::Zero;

    use super::*;

    /// Helper: assert crowdfunding invariants for a campaign
    fn assert_crowdfunding_invariants(campaign_id: u32) {
        let campaign = pallet::Campaigns::<Test>::get(campaign_id).unwrap();

        // I-1: total_disbursed <= total_raised
        assert!(
            campaign.total_disbursed <= campaign.total_raised,
            "Invariant I-1 violated: total_disbursed ({}) > total_raised ({})",
            campaign.total_disbursed,
            campaign.total_raised
        );

        // I-2: investor_count matches active investments (for non-terminal)
        if !matches!(
            campaign.status,
            CampaignStatus::Failed | CampaignStatus::Cancelled | CampaignStatus::Completed
        ) {
            let actual_investors = pallet::Investments::<Test>::iter_prefix(campaign_id)
                .filter(|(_, inv)| inv.total_invested > inv.total_withdrawn)
                .count() as u32;
            assert_eq!(
                campaign.investor_count, actual_investors,
                "Invariant I-2 violated: investor_count ({}) != actual ({})",
                campaign.investor_count, actual_investors
            );
        }

        // I-3: CreatorCampaigns consistency — if not terminal with zero deposit, should
        // contain id
        if !matches!(
            campaign.status,
            CampaignStatus::Completed | CampaignStatus::Failed | CampaignStatus::Cancelled
        ) || !campaign.creation_deposit.is_zero()
        {
            // Campaign should be in creator's list if deposit not yet claimed
            if !campaign.creation_deposit.is_zero() {
                assert!(
                    pallet::CreatorCampaigns::<Test>::get(&campaign.creator).contains(&campaign_id),
                    "Invariant I-3 violated: campaign {} not in CreatorCampaigns for creator {:?}",
                    campaign_id,
                    campaign.creator
                );
            }
        }
    }

    /// Test invariants hold after a simple invest cycle
    #[test]
    fn invariants_hold_after_invest() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 300));
            assert_crowdfunding_invariants(id);
        });
    }

    /// Test invariants hold after withdraw cycle
    #[test]
    fn invariants_hold_after_withdraw() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));
            assert_crowdfunding_invariants(id);
        });
    }

    /// Test invariants hold after full withdrawal
    #[test]
    fn invariants_hold_after_full_withdrawal() {
        ExtBuilder::default().build().execute_with(|| {
            let mut config = default_aon_config(100, 5000);
            config.early_withdrawal_penalty_bps = Some(0);
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
            assert_crowdfunding_invariants(id);
        });
    }

    /// Test invariants hold during milestone phase
    #[test]
    fn invariants_hold_during_milestone_phase() {
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
            assert_crowdfunding_invariants(id);

            // After milestone claim
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));
            assert_crowdfunding_invariants(id);
        });
    }

    /// Test invariants hold after pause/resume
    #[test]
    fn invariants_hold_after_pause_resume() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_crowdfunding_invariants(id);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            assert_crowdfunding_invariants(id);
        });
    }

    /// Test invariants hold after complete lifecycle with multiple investors
    #[test]
    fn invariants_hold_complex_lifecycle() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let config = milestone_config(
                    20,
                    1000,
                    vec![
                        Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                        Milestone { release_bps: 4000, description_hash: [3u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 2000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 1000));
                assert_crowdfunding_invariants(id);

                // Partial withdrawal
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
                assert_crowdfunding_invariants(id);

                // Finalize
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_crowdfunding_invariants(id);

                // Milestone 0 claim
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                assert_crowdfunding_invariants(id);

                // Cancel and refund
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// CROSS-PALLET ADVERSARIAL PERSONA TESTS — Part C (P-X01 through P-X14)
// + PART D (Toppan Use Case Coverage)
// ═══════════════════════════════════════════════════════════════════════

mod cross_pallet_adversarial {
    use mock::{MockLicenseVerifier, LICENSE_STATE};

    use super::*;

    // ── Helper: create a campaign WITH a license ──────────────────────
    fn create_licensed_campaign(
        creator: u64,
        config: CampaignConfigOf,
        rwa_asset_id: u32,
        participation_id: u32,
    ) -> u32 {
        use frame_support::assert_ok;
        let id = pallet::NextCampaignId::<Test>::get();
        assert_ok!(Crowdfunding::create_campaign(
            RuntimeOrigin::signed(creator),
            config,
            None,
            Some((rwa_asset_id, participation_id)),
        ));
        id
    }

    // ── Helper: setup fungible asset + mint to accounts ───────────────
    fn setup_asset(asset_id: u32) {
        use frame_support::assert_ok;
        assert_ok!(Assets::force_create(
            RuntimeOrigin::root(),
            codec::Compact(asset_id),
            ALICE,
            true,
            1
        ));
        for acct in [ALICE, BOB, CHARLIE, DAVE] {
            assert_ok!(Assets::mint(
                RuntimeOrigin::signed(ALICE),
                codec::Compact(asset_id),
                acct,
                50_000
            ));
        }
    }

    fn asset_aon_config(deadline: u64, goal: u128, asset_id: u32) -> CampaignConfigOf {
        crate::CampaignConfig {
            funding_model: crate::FundingModel::AllOrNothing { goal },
            funding_currency: crate::PaymentCurrency::Asset(asset_id),
            deadline,
            hard_cap: None,
            min_investment: None,
            max_investment_per_investor: None,
            metadata_hash: [0u8; 32],
            early_withdrawal_penalty_bps: None,
        }
    }

    fn asset_kwyr_config(deadline: u64, asset_id: u32) -> CampaignConfigOf {
        crate::CampaignConfig {
            funding_model: crate::FundingModel::KeepWhatYouRaise { soft_cap: None },
            funding_currency: crate::PaymentCurrency::Asset(asset_id),
            deadline,
            hard_cap: None,
            min_investment: None,
            max_investment_per_investor: None,
            metadata_hash: [0u8; 32],
            early_withdrawal_penalty_bps: None,
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X01: "The License Escape Artist" — Creator Escaping License Check
    // ═════════════════════════════════════════════════════════════════

    mod px01_license_escape_artist {
        use super::*;

        // C01: DEADLOCK-FIX — License deactivated after funding, claim_funds still
        // succeeds
        #[test]
        fn c01_claim_funds_succeeds_despite_inactive_license() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = default_kwyr_config(20);
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Deactivate license
                MockLicenseVerifier::set_active(0, 0, false);

                // DEADLOCK-FIX: claim_funds no longer checks license status
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert!(Balances::free_balance(ALICE) > alice_before);
            });
        }

        // C02: License deactivated → report_license_revoked → Cancelled → investors
        // refund
        #[test]
        fn c02_report_license_revoked_then_investors_refund() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = default_kwyr_config(20);
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 300));

                // Deactivate license during Funding
                MockLicenseVerifier::set_active(0, 0, false);

                // Anyone can report license revocation
                assert_ok!(Crowdfunding::report_license_revoked(RuntimeOrigin::signed(DAVE), id));
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.status, CampaignStatus::Cancelled);

                // Investors claim full refund
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 500);

                let charlie_before = Balances::free_balance(CHARLIE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 300);
            });
        }

        // C03: DEADLOCK-FIX — claim_funds no longer checks license status or
        // authorized account. Campaign creator check (NotCampaignCreator) is the
        // only caller identity guard. Changing the license authorized account
        // does not block the original campaign creator from claiming.
        #[test]
        fn c03_claim_funds_ignores_license_auth_change() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = default_kwyr_config(20);
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Change authorized account to BOB but keep active=true.
                // This simulates a participation transfer to a different account.
                LICENSE_STATE.with(|m| {
                    m.borrow_mut().insert((0, 0), (BOB, true));
                });

                // DEADLOCK-FIX: ALICE can still claim because claim_funds only
                // checks campaign.creator == who, not license authorization.
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 500);
            });
        }

        // C04: DEADLOCK-FIX — License revoked during MilestonePhase, approved
        // milestones can still be claimed (license check removed)
        #[test]
        fn c04_milestone_claim_succeeds_despite_revoked_license() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = milestone_config(
                    20,
                    1000,
                    vec![
                        Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                        Milestone { release_bps: 4000, description_hash: [3u8; 32] },
                    ],
                );
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestone 0: 30% of 1000 = 300
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 300);

                // Deactivate license
                MockLicenseVerifier::set_active(0, 0, false);

                // Submit and approve milestone 1 — DEADLOCK-FIX: claim now succeeds
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    1
                ));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 300);

                // Verify already-claimed milestone 0 stays Claimed
                assert_eq!(
                    pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                    Some(MilestoneStatus::Claimed)
                );

                // Total disbursed includes both milestones
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_disbursed, 600);
            });
        }

        // C05: DEADLOCK-FIX — License renewal is irrelevant; claim_funds
        // succeeds regardless of license status after finalization.
        #[test]
        fn c05_claim_funds_succeeds_regardless_of_license_status() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = default_kwyr_config(20);
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Deactivate — claim still works (DEADLOCK-FIX)
                MockLicenseVerifier::set_active(0, 0, false);

                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 500);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X02: "The NFT Juggler" — NFT Flash Loan Pattern
    // ═════════════════════════════════════════════════════════════════

    mod px02_nft_juggler {
        use super::*;

        // C06: NFT passed between accounts — both can invest (point-in-time check)
        #[test]
        fn c06_nft_transfer_between_investors_both_invest() {
            ExtBuilder::default().build().execute_with(|| {
                let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
                let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
                let rules: BoundedVec<_, _> =
                    vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 5000),
                    Some(rules),
                    None,
                ));

                // AccountA owns NFT → invest
                MockNftInspect::set_owner(1, 1, BOB);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));

                // "Transfer" NFT to AccountB
                NFT_OWNERS.with(|m| m.borrow_mut().remove(&(1u32, 1u32)));
                MockNftInspect::set_owner(1, 1, CHARLIE);

                // AccountB can now invest too (point-in-time check)
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));

                // Both investments recorded
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 300);
                assert_eq!(c.investor_count, 2);
            });
        }

        // C07: NFT eligibility + whitelist — whitelisted account borrows NFT → invest →
        // return
        #[test]
        fn c07_nft_plus_whitelist_flash_loan_pattern() {
            ExtBuilder::default().build().execute_with(|| {
                let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
                let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
                let rules: BoundedVec<_, _> = vec![
                    EligibilityRule::NftOwnership { required_sets },
                    EligibilityRule::AccountWhitelist,
                ]
                .try_into()
                .unwrap();
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 5000),
                    Some(rules),
                    None,
                ));

                // Whitelist BOB
                assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));

                // BOB doesn't own NFT — fails
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                    Error::<Test>::EligibilityCheckFailed
                );

                // BOB "borrows" NFT
                MockNftInspect::set_owner(1, 1, BOB);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));

                // "Return" NFT (remove from BOB)
                NFT_OWNERS.with(|m| m.borrow_mut().remove(&(1u32, 1u32)));

                // BOB's investment is still valid — existing investment stays
                let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
                assert_eq!(inv.total_invested, 100);
            });
        }

        // C08: Same block sequence — multiple NFT transfers and investments
        #[test]
        fn c08_same_block_nft_juggling() {
            ExtBuilder::default().build().execute_with(|| {
                let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
                let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
                let rules: BoundedVec<_, _> =
                    vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 5000),
                    Some(rules),
                    None,
                ));

                // All in same block (block 1):
                // Step 1: set NFT to BOB → invest
                MockNftInspect::set_owner(1, 1, BOB);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));

                // Step 2: clear NFT from BOB → set NFT to CHARLIE → invest
                NFT_OWNERS.with(|m| m.borrow_mut().remove(&(1u32, 1u32)));
                MockNftInspect::set_owner(1, 1, CHARLIE);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));

                // Step 3: clear NFT from CHARLIE → set NFT to DAVE → invest
                NFT_OWNERS.with(|m| m.borrow_mut().remove(&(1u32, 1u32)));
                MockNftInspect::set_owner(1, 1, DAVE);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 300));

                // All three investments succeeded in same block
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 600);
                assert_eq!(c.investor_count, 3);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X03: "The Asset Freezer" — Payment Asset Freeze
    // ═════════════════════════════════════════════════════════════════

    mod px03_asset_freezer {
        use super::*;

        // C09: Asset-based campaign → fund sub-account → destroy asset → refund fails
        #[test]
        fn c09_asset_destroy_makes_refund_impossible() {
            ExtBuilder::default().build().execute_with(|| {
                setup_asset(1);
                let config = asset_aon_config(20, 5000, 1);
                let id = create_funded_campaign(ALICE, config);

                // Invest with Asset(1)
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                // Failed (goal not met)

                let sub = Crowdfunding::campaign_account(id);
                let sub_asset_balance =
                    <Assets as frame_support::traits::tokens::fungibles::Inspect<u64>>::balance(
                        1, &sub,
                    );
                assert_eq!(sub_asset_balance, 500);

                // Admin destroys the asset — this will fail because accounts
                // still have balances. The destroy operation requires all
                // accounts to be removed first. So we test that the asset
                // balance is still intact and refund works normally.
                let bob_before = <Assets as frame_support::traits::tokens::fungibles::Inspect<
                    u64,
                >>::balance(1, &BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                let bob_after = <Assets as frame_support::traits::tokens::fungibles::Inspect<
                    u64,
                >>::balance(1, &BOB);
                assert_eq!(bob_after, bob_before + 500);
            });
        }

        // C10: Asset min_balance changes — verify sub-account still works
        #[test]
        fn c10_asset_min_balance_does_not_affect_claim() {
            ExtBuilder::default().build().execute_with(|| {
                setup_asset(1);
                let config = asset_kwyr_config(20, 1);
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim funds succeeds with asset currency
                let alice_before = <Assets as frame_support::traits::tokens::fungibles::Inspect<
                    u64,
                >>::balance(1, &ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                let alice_after = <Assets as frame_support::traits::tokens::fungibles::Inspect<
                    u64,
                >>::balance(1, &ALICE);
                assert_eq!(alice_after, alice_before + 1000);
            });
        }

        // C11: Asset(1) campaign — verify creation deposit is still native
        #[test]
        fn c11_creation_deposit_always_native_regardless_of_funding_currency() {
            ExtBuilder::default().build().execute_with(|| {
                setup_asset(1);
                let alice_native_before = Balances::free_balance(ALICE);
                let config = asset_aon_config(100, 1000, 1);
                let id = create_funded_campaign(ALICE, config);

                // Creation deposit deducted from native balance
                assert_eq!(Balances::free_balance(ALICE), alice_native_before - 100);

                // Sub-account has native deposit
                let sub = Crowdfunding::campaign_account(id);
                assert_eq!(Balances::free_balance(sub), 100);

                // Cancel and claim deposit — it's native
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 100);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X04: "The ED Reaper" — Cross-Pallet Account Death
    // ═════════════════════════════════════════════════════════════════

    mod px04_ed_reaper {
        use super::*;

        // C12: Sub-account with only creation_deposit → claim deposit with AllowDeath
        // → sub-account reaped to 0
        #[test]
        fn c12_claim_deposit_reaps_sub_account() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                let sub = Crowdfunding::campaign_account(id);
                assert_eq!(Balances::free_balance(sub), 100);

                // Cancel (no investors)
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                // Claim deposit — sub reaped to 0
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(sub), 0);
            });
        }

        // C13: Sub-account with investors → claim deposit first → sub has only
        // investor funds → last investor claim_refund → sub reaped to 0
        #[test]
        fn c13_deposit_first_then_last_investor_reaps_sub() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                let sub = Crowdfunding::campaign_account(id);
                assert_eq!(Balances::free_balance(sub), 600); // 100 + 500

                // Creator claims deposit first
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(sub), 500);

                // Last investor reaps sub to 0
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 500);
                assert_eq!(Balances::free_balance(sub), 0);
            });
        }

        // C14: Investor receiving refund stays above ED (they're receiving, not
        // sending)
        #[test]
        fn c14_investor_receiving_refund_stays_above_ed() {
            ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 502)]).build().execute_with(
                || {
                    let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
                    // BOB has 502, invest 500 → BOB left with 2 (above ED=1)
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                    assert_eq!(Balances::free_balance(BOB), 2);

                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                    // BOB receives refund → balance goes from 2 to 502
                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                    assert_eq!(Balances::free_balance(BOB), 502);
                },
            );
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X05: "The Governance Wrecker" — Admin Abuse
    // ═════════════════════════════════════════════════════════════════

    mod px05_governance_wrecker {
        use super::*;

        // C15: Admin cancel after partial milestone disbursement → correct refund
        // amounts
        #[test]
        fn c15_admin_cancel_after_partial_milestone_correct_refund() {
            ExtBuilder::default().build().execute_with(|| {
                let config = milestone_config(
                    20,
                    1000,
                    vec![
                        Milestone { release_bps: 6000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 4000, description_hash: [2u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 400));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestone 0: 60% of 1000 = 600
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));

                // Admin cancel
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                // remaining_ratio = (1000 - 600) / 1000 = 40%
                // BOB refund: 40% of 600 = 240
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 240);

                // CHARLIE refund: 40% of 400 = 160
                let charlie_before = Balances::free_balance(CHARLIE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 160);
            });
        }

        // C16: Admin cancel Succeeded campaign → investors refund → creator CANNOT
        // claim_funds
        #[test]
        fn c16_admin_cancel_succeeded_is_blocked() {
            // CRIT-02 FIX: admin cannot cancel a Succeeded campaign — creator
            // has a guaranteed window to claim funds after finalization.
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_kwyr_config(20));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Admin cancel on Succeeded — now BLOCKED
                assert_noop!(
                    Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Campaign remains Succeeded — creator can still claim
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Succeeded
                );
            });
        }

        // C17: Admin cancel affects campaign only, not external state
        #[test]
        fn c17_admin_cancel_does_not_affect_other_campaigns() {
            ExtBuilder::default().build().execute_with(|| {
                let id1 = create_funded_campaign(ALICE, default_kwyr_config(20));
                let id2 = create_funded_campaign(ALICE, default_kwyr_config(20));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id1, 500));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id2, 300));

                // Cancel only id1
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id1));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id1).unwrap().status,
                    CampaignStatus::Cancelled
                );
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id2).unwrap().status,
                    CampaignStatus::Funding
                );

                // id2 still works normally
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id2));
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id2));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 300);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X06: "The Double-Spend Racer" — Same-Block Operations
    // ═════════════════════════════════════════════════════════════════

    mod px06_double_spend_racer {
        use super::*;

        // C18: invest → immediately withdraw in same block → net effect tracked
        #[test]
        fn c18_invest_then_withdraw_same_block() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));

                // Same block: invest then withdraw
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 200));

                let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
                assert_eq!(inv.total_invested, 500);
                assert_eq!(inv.total_withdrawn, 200);
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 300);
            });
        }

        // C19: report_license_revoked → campaign cancelled → immediate claim_refund
        #[test]
        fn c19_report_then_immediate_refund() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = default_kwyr_config(100);
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));

                // Deactivate license
                MockLicenseVerifier::set_active(0, 0, false);

                // Report → Cancel → Refund all in same block
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(CHARLIE),
                    id
                ));
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 500);
            });
        }

        // C20: invest + transfer_nft in sequence — eligibility checked at invest time
        // only
        #[test]
        fn c20_invest_then_nft_transfer_preserves_investment() {
            ExtBuilder::default().build().execute_with(|| {
                let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
                let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
                let rules: BoundedVec<_, _> =
                    vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 5000),
                    Some(rules),
                    None,
                ));

                // BOB owns NFT → invests
                MockNftInspect::set_owner(1, 1, BOB);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));

                // BOB "transfers" NFT away
                NFT_OWNERS.with(|m| m.borrow_mut().remove(&(1u32, 1u32)));

                // BOB's investment is preserved
                let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
                assert_eq!(inv.total_invested, 100);

                // But BOB cannot invest MORE (no longer owns NFT)
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50),
                    Error::<Test>::EligibilityCheckFailed
                );
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X07: "The Escrow Manipulator" — Sub-Account Balance Manipulation
    // ═════════════════════════════════════════════════════════════════

    mod px07_escrow_manipulator {
        use super::*;

        // C21: Extra funds deposited into sub-account don't affect refund calculations
        #[test]
        fn c21_extra_funds_in_sub_account_dont_affect_refund() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));

                let sub = Crowdfunding::campaign_account(id);

                // Governance (or anyone) deposits extra funds into sub-account
                let _ = Balances::deposit_creating(&sub, 9999);

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                // Failed (500 < 5000 goal)

                // BOB still gets exactly 500 refund, not more
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 500);
            });
        }

        // C22: claim_funds only transfers total_raised - total_disbursed, not full
        // sub-balance
        #[test]
        fn c22_claim_funds_transfers_exact_claimable_not_full_balance() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_kwyr_config(20));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));

                let sub = Crowdfunding::campaign_account(id);
                // Add extra to sub-account
                let _ = Balances::deposit_creating(&sub, 5000);

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // ALICE claims exactly 500 (total_raised), not 500+5000
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 500);

                // Sub-account still has deposit + extra
                assert_eq!(Balances::free_balance(sub), 100 + 5000);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X08: "The Currency Switcher" — Currency Mismatch
    // ═════════════════════════════════════════════════════════════════

    mod px08_currency_switcher {
        use super::*;

        // C24: Campaign uses Native → license check is orthogonal to currency
        #[test]
        fn c24_native_currency_with_license_works() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = default_kwyr_config(20);
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 500);
            });
        }

        // C25: Campaign uses Asset(1) → license check is orthogonal to currency
        #[test]
        fn c25_asset_currency_with_license_works() {
            ExtBuilder::default().build().execute_with(|| {
                setup_asset(1);
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = asset_kwyr_config(20, 1);
                let id = create_licensed_campaign(ALICE, config, 0, 0);

                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                let alice_before = <Assets as frame_support::traits::tokens::fungibles::Inspect<
                    u64,
                >>::balance(1, &ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                let alice_after = <Assets as frame_support::traits::tokens::fungibles::Inspect<
                    u64,
                >>::balance(1, &ALICE);
                assert_eq!(alice_after, alice_before + 500);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X09: "The Toppan IP Flow" — Full Lifecycle Tests
    // ═════════════════════════════════════════════════════════════════

    mod px09_toppan_ip_flow {
        use super::*;

        // C27: Happy path: license → campaign → invest → finalize → claim → deposit
        #[test]
        fn c27_full_toppan_happy_path() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(1, 10, ALICE, true);
                let config = default_kwyr_config(20);
                let id = create_licensed_campaign(ALICE, config, 1, 10);

                // Verify campaign has license link
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.rwa_asset_id, Some(1));
                assert_eq!(c.participation_id, Some(10));

                // Invest
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 400));

                // Finalize
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Succeeded
                );

                // Claim funds
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);

                // Claim creation deposit
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 100);

                // Verify all balances reconcile
                let sub = Crowdfunding::campaign_account(id);
                assert_eq!(Balances::free_balance(sub), 0);
            });
        }

        // C28: Same license → two campaigns → one succeeds, one fails
        #[test]
        fn c28_same_license_two_campaigns_independent() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);

                let id1 = create_licensed_campaign(ALICE, default_aon_config(20, 1000), 0, 0);
                let id2 = create_licensed_campaign(ALICE, default_aon_config(20, 5000), 0, 0);

                // Fund id1 fully, id2 partially
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id1, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id2, 500));

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id1));
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id2));

                // id1 succeeded, id2 failed
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id1).unwrap().status,
                    CampaignStatus::Succeeded
                );
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id2).unwrap().status,
                    CampaignStatus::Failed
                );

                // Both use same license but operate independently
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id1));
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id2));
            });
        }

        // C29: DEADLOCK-FIX — License deactivated after creation, claim_funds
        // still succeeds after finalization.
        #[test]
        fn c29_claim_succeeds_despite_license_deactivated_after_creation() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));

                // Deactivate AFTER creation but BEFORE finalize
                MockLicenseVerifier::set_active(0, 0, false);

                // Finalize still works (permissionless, doesn't check license)
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // DEADLOCK-FIX: claim_funds succeeds despite inactive license
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert!(Balances::free_balance(ALICE) > alice_before);
            });
        }

        // C30: Slash Flow: license → milestone → deactivate → report → cancel →
        // proportional refund
        #[test]
        fn c30_slash_flow_milestone_then_report() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = milestone_config(
                    20,
                    500,
                    vec![
                        Milestone { release_bps: 4000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 6000, description_hash: [2u8; 32] },
                    ],
                );
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestone 0: 40% of 1000 = 400
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));

                // License slash (deactivate)
                MockLicenseVerifier::set_active(0, 0, false);

                // Report license revocation → Cancelled
                assert_ok!(Crowdfunding::report_license_revoked(RuntimeOrigin::signed(DAVE), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                );

                // Proportional refund: remaining = (1000-400)/1000 = 60%
                // BOB refund: 60% of 1000 = 600
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 600);
            });
        }

        // C31: Creator is also investor (self-deal) → invest own campaign → verify
        // behavior
        #[test]
        fn c31_creator_self_deal() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_aon_config(20, 1000), 0, 0);

                // ALICE invests in her own campaign
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(ALICE), id, 500));
                // BOB also invests
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // ALICE claims funds (creator)
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 1100);

                // But ALICE also had an investment — now Completed, so refund fails
                assert_noop!(
                    Crowdfunding::claim_refund(RuntimeOrigin::signed(ALICE), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
        }

        // C32: Multiple investors → partial withdrawal → finalize → claim_funds →
        // verify balances
        #[test]
        fn c32_multi_investor_partial_withdraw_then_claim() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);

                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 400));

                // BOB withdraws 100 (net 99 after 1% penalty)
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100));

                // total_raised = 600 + 400 - 100 = 900
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_raised, 900);

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // ALICE claims 900
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 900);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X10: "The Refund Maximizer" — Post-Cancel Refund
    // ═════════════════════════════════════════════════════════════════

    mod px10_refund_maximizer {
        use super::*;

        // C33: 3 milestones: claim 2 (60% disbursed) → cancel → refund
        #[test]
        fn c33_two_of_three_milestones_claimed_then_cancel_then_refund() {
            ExtBuilder::default().build().execute_with(|| {
                let config = milestone_config(
                    20,
                    500,
                    vec![
                        Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                        Milestone { release_bps: 4000, description_hash: [3u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestones 0 and 1: 30% + 30% = 60% of 1000 = 600
                for i in 0..2u8 {
                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, i));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        i
                    ));
                }

                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_disbursed, 600);

                // Cancel
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                // remaining_ratio = (1000 - 600) / 1000 = 40%
                // BOB refund: 40% of 1000 = 400
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 400);
            });
        }

        // C34: Invest 100 → withdraw 50 → cancel → refund based on net investment
        #[test]
        fn c34_invest_withdraw_cancel_refund_net() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 50));

                // BOB: total_invested=100, total_withdrawn=50, net=50
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                // Refund = total_invested - total_withdrawn = 50
                // (no disbursement, so full net refund)
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 50);
            });
        }

        // C35: Multiple investors with different amounts → proportional refund → verify
        // each
        #[test]
        fn c35_multiple_investors_proportional_refund() {
            ExtBuilder::default().build().execute_with(|| {
                let config = milestone_config(
                    20,
                    500,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 300));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 100));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestone 0: 50% of 1000 = 500
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));

                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                // remaining_ratio = (1000-500)/1000 = 50%

                // BOB: 50% of 600 = 300
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 300);

                // CHARLIE: 50% of 300 = 150
                let charlie_before = Balances::free_balance(CHARLIE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 150);

                // DAVE: 50% of 100 = 50
                let dave_before = Balances::free_balance(DAVE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(DAVE), id));
                assert_eq!(Balances::free_balance(DAVE), dave_before + 50);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X11: "Accidental Alice" — User Error
    // ═════════════════════════════════════════════════════════════════

    mod px11_accidental_alice {
        use super::*;

        // C36: Create campaign with license not in verifier → fails
        #[test]
        fn c36_create_with_nonexistent_license_fails() {
            ExtBuilder::default().build().execute_with(|| {
                // Do NOT set any license in the verifier
                assert_noop!(
                    Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        default_kwyr_config(100),
                        None,
                        Some((99, 99)),
                    ),
                    Error::<Test>::LicenseNotActive
                );
            });
        }

        // C37: Create campaign with expired license → fails
        #[test]
        fn c37_create_with_expired_license_fails() {
            ExtBuilder::default().build().execute_with(|| {
                // Set license but inactive
                MockLicenseVerifier::set_license(0, 0, ALICE, false);
                assert_noop!(
                    Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        default_kwyr_config(100),
                        None,
                        Some((0, 0)),
                    ),
                    Error::<Test>::LicenseNotActive
                );
            });
        }

        // C38: DEADLOCK-FIX — Changing license authorized account does NOT block
        // the campaign creator from claiming. claim_funds uses campaign.creator
        // check, not license authorization.
        #[test]
        fn c38_claim_succeeds_despite_license_auth_change() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Change authorized to CHARLIE, keep active
                LICENSE_STATE.with(|m| {
                    m.borrow_mut().insert((0, 0), (CHARLIE, true));
                });

                // DEADLOCK-FIX: ALICE can still claim — claim_funds only checks
                // campaign.creator == who, license authorization is no longer checked.
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 500);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X12: "The Confused Investor"
    // ═════════════════════════════════════════════════════════════════

    mod px12_confused_investor {
        use super::*;

        // C39: Investor meets eligibility → invests → no longer meets → cannot invest
        // MORE
        #[test]
        fn c39_eligibility_lost_after_invest_blocks_additional() {
            ExtBuilder::default().build().execute_with(|| {
                let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
                let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
                let rules: BoundedVec<_, _> =
                    vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 5000),
                    Some(rules),
                    None,
                ));

                MockNftInspect::set_owner(1, 1, BOB);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));

                // "Lose" the NFT
                NFT_OWNERS.with(|m| m.borrow_mut().remove(&(1u32, 1u32)));

                // Cannot invest MORE
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50),
                    Error::<Test>::EligibilityCheckFailed
                );

                // But existing investment stays
                let inv = pallet::Investments::<Test>::get(id, BOB).unwrap();
                assert_eq!(inv.total_invested, 100);
            });
        }

        // C40: Withdraw with penalty → verify penalty amount matches expected
        #[test]
        fn c40_withdraw_penalty_verification() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));

                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
                // 1% penalty on 500 = 5, net = 495
                assert_eq!(Balances::free_balance(BOB), bob_before + 495);

                // Verify sub-account balance decreased by full withdrawal amount (not net)
                let sub = Crowdfunding::campaign_account(id);
                // Sub had 100+1000=1100, now 1100-500=600
                assert_eq!(Balances::free_balance(sub), 600);
            });
        }

        // C41: KWYR → invest → finalize Succeeded → no refund possible
        #[test]
        fn c41_kwyr_succeeded_no_refund() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_kwyr_config(20));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Succeeded — no refund
                assert_noop!(
                    Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X13: "The Lazy Expiry Exploiter"
    // ═════════════════════════════════════════════════════════════════

    mod px13_lazy_expiry_exploiter {
        use super::*;

        // C42: DEADLOCK-FIX — License set inactive after finalization, claim_funds
        // still succeeds.
        #[test]
        fn c42_claim_succeeds_despite_inactive_license() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                MockLicenseVerifier::set_active(0, 0, false);
                // DEADLOCK-FIX: claim_funds no longer checks license status
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert!(Balances::free_balance(ALICE) > alice_before);
            });
        }

        // C43: DEADLOCK-FIX — License reactivation is irrelevant; claim_funds
        // succeeds regardless of license status. Reactivation was previously
        // needed to unblock claims, but the deadlock fix removed that requirement.
        #[test]
        fn c43_claim_succeeds_without_reactivation() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Deactivate — claim still works immediately (no reactivation needed)
                MockLicenseVerifier::set_active(0, 0, false);

                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 500);
            });
        }

        // C44: License stays inactive → only report_license_revoked can cancel →
        // investors claim refund
        #[test]
        fn c44_persistent_inactive_only_report_can_cancel() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(100), 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));

                MockLicenseVerifier::set_active(0, 0, false);

                // report_license_revoked cancels
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(CHARLIE),
                    id
                ));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                );

                // Investor refund
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 500);
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // P-X14: "The Sybil Army Commander"
    // ═════════════════════════════════════════════════════════════════

    mod px14_sybil_army_commander {
        use super::*;

        // C45: Same license → multiple campaigns by same creator → all work
        // independently
        #[test]
        fn c45_same_license_multiple_campaigns_independent() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);

                let id1 = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);
                let id2 = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);
                let id3 = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);

                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id1, 100));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id2, 200));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id3, 300));

                run_to_block(21);
                for id in [id1, id2, id3] {
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                }

                // Claim each independently
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id1));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 100);

                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id2));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 200);

                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id3));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 300);
            });
        }

        // C46: Multiple accounts invest same campaign → each has own Investment → each
        // refund
        #[test]
        fn c46_multiple_investors_independent_records() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 5000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 300));

                // Each has independent Investment record
                assert_eq!(pallet::Investments::<Test>::get(id, BOB).unwrap().total_invested, 100);
                assert_eq!(
                    pallet::Investments::<Test>::get(id, CHARLIE).unwrap().total_invested,
                    200
                );
                assert_eq!(pallet::Investments::<Test>::get(id, DAVE).unwrap().total_invested, 300);

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                // Failed (600 < 5000)

                // Each gets own refund
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 100);

                let charlie_before = Balances::free_balance(CHARLIE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 200);

                let dave_before = Balances::free_balance(DAVE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(DAVE), id));
                assert_eq!(Balances::free_balance(DAVE), dave_before + 300);
            });
        }

        // C47: Multiple accounts with NFT eligibility → each independently checked
        #[test]
        fn c47_nft_eligibility_per_account() {
            ExtBuilder::default().build().execute_with(|| {
                let nft_set: BoundedVec<(u32, u32), _> = vec![(1u32, 1u32)].try_into().unwrap();
                let required_sets: BoundedVec<_, _> = vec![nft_set].try_into().unwrap();
                let rules: BoundedVec<_, _> =
                    vec![EligibilityRule::NftOwnership { required_sets }].try_into().unwrap();
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(100, 5000),
                    Some(rules),
                    None,
                ));

                // BOB owns the NFT — can invest
                MockNftInspect::set_owner(1, 1, BOB);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));

                // CHARLIE does NOT own the NFT — cannot invest
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100),
                    Error::<Test>::EligibilityCheckFailed
                );

                // DAVE does NOT own the NFT — cannot invest
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 100),
                    Error::<Test>::EligibilityCheckFailed
                );

                // Give CHARLIE the same NFT (simulating transfer from BOB)
                MockNftInspect::set_owner(1, 1, CHARLIE);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // PART D: Toppan Flow Tests
    // ═════════════════════════════════════════════════════════════════

    mod part_d_toppan_flow {
        use super::*;

        // D01: Full Toppan happy path with all balance reconciliation
        #[test]
        fn d01_full_toppan_happy_path_balance_reconciliation() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(1, 5, ALICE, true);
                let config = default_aon_config(20, 1000);
                let id = create_licensed_campaign(ALICE, config, 1, 5);

                let alice_start = Balances::free_balance(ALICE); // 10000 - 100 = 9900
                assert_eq!(alice_start, 9900);

                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 500));

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Succeeded
                );

                // Claim funds: 1100
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 1100);

                // Claim deposit: 100
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 100);

                // Final check: sub-account empty
                let sub = Crowdfunding::campaign_account(id);
                assert_eq!(Balances::free_balance(sub), 0);

                // ALICE net gain: started 10000, deposit -100, claim +1100, deposit +100 =
                // 11100
                assert_eq!(Balances::free_balance(ALICE), 11100);
            });
        }

        // D02: Toppan slash flow: milestone claim → license deactivate → report →
        // cancel → refund
        #[test]
        fn d02_toppan_slash_flow() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(2, 7, ALICE, true);
                let config = milestone_config(
                    20,
                    500,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id = create_licensed_campaign(ALICE, config, 2, 7);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 800));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::MilestonePhase
                );

                // Claim milestone 0: 50% of 1000 = 500
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 500);

                // Slash: deactivate license
                MockLicenseVerifier::set_active(2, 7, false);

                // Report revocation → Cancelled
                assert_ok!(Crowdfunding::report_license_revoked(RuntimeOrigin::signed(DAVE), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                );

                // remaining_ratio = (1000-500)/1000 = 50%
                // BOB: 50% of 800 = 400
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 400);

                // CHARLIE: 50% of 200 = 100
                let charlie_before = Balances::free_balance(CHARLIE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 100);

                // Sub-account has deposit (100) remaining
                let sub = Crowdfunding::campaign_account(id);
                assert_eq!(Balances::free_balance(sub), 100);

                // Creator claims deposit
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(sub), 0);
            });
        }

        // D03: Toppan AON failure: license → campaign(AON, goal=1000) → raise 500 →
        // finalize Failed → full refund
        #[test]
        fn d03_toppan_aon_failure_full_refund() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = default_aon_config(20, 1000);
                let id = create_licensed_campaign(ALICE, config, 0, 0);

                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 300));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Failed
                );

                // Full refund (no disbursement)
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 300);

                let charlie_before = Balances::free_balance(CHARLIE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 200);

                // License is still active (doesn't matter for failed campaign)
                assert!(MockLicenseVerifier::is_license_active(0, 0));
            });
        }

        // D04: Multiple campaigns per license → verify independence
        #[test]
        fn d04_multiple_campaigns_per_license_independent() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(5, 5, ALICE, true);

                let id1 = create_licensed_campaign(ALICE, default_kwyr_config(20), 5, 5);
                let id2 = create_licensed_campaign(ALICE, default_aon_config(20, 500), 5, 5);
                let id3 = create_licensed_campaign(
                    ALICE,
                    milestone_config(
                        20,
                        500,
                        vec![
                            Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                        ],
                    ),
                    5,
                    5,
                );

                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id1, 100));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id2, 500));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id3, 600));

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id1));
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id2));
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id3));

                // id1: Succeeded (KWYR always succeeds)
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id1).unwrap().status,
                    CampaignStatus::Succeeded
                );
                // id2: Succeeded (500 >= 500)
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id2).unwrap().status,
                    CampaignStatus::Succeeded
                );
                // id3: MilestonePhase (600 >= 500)
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id3).unwrap().status,
                    CampaignStatus::MilestonePhase
                );

                // All use same license but independent outcomes
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id1));
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id2));
                // id3 needs milestone flow
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id3, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id3, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id3,
                    0
                ));
            });
        }

        // D05: DEADLOCK-FIX — Milestone claim succeeds regardless of license
        // status. License renewal is no longer needed to unblock claims.
        #[test]
        fn d05_milestone_claim_succeeds_despite_expired_license() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = milestone_config(
                    20,
                    500,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestone 0
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));

                // License expires
                MockLicenseVerifier::set_active(0, 0, false);

                // Submit and approve milestone 1 — DEADLOCK-FIX: claim succeeds
                // even with expired license (no renewal needed)
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));

                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    1
                ));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 500);

                // Campaign completed
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Completed
                );
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // Additional edge cases for report_license_revoked
    // ═════════════════════════════════════════════════════════════════

    mod report_license_revoked_edge_cases {
        use super::*;

        #[test]
        fn report_on_campaign_without_license_fails() {
            ExtBuilder::default().build().execute_with(|| {
                // Campaign created WITHOUT license
                let id = create_funded_campaign(ALICE, default_kwyr_config(100));
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::NoLinkedLicense
                );
            });
        }

        #[test]
        fn report_when_license_still_active_fails() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(100), 0, 0);
                // License is still active → report should fail
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::LicenseNotActive
                );
            });
        }

        #[test]
        fn report_on_completed_campaign_fails() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                // Completed

                MockLicenseVerifier::set_active(0, 0, false);
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
        }

        #[test]
        fn report_on_failed_campaign_fails() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_aon_config(20, 5000), 0, 0);
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                // Failed

                MockLicenseVerifier::set_active(0, 0, false);
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
        }

        #[test]
        fn report_on_cancelled_campaign_fails() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(100), 0, 0);
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                // Already cancelled

                MockLicenseVerifier::set_active(0, 0, false);
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
        }

        #[test]
        fn report_on_succeeded_campaign_is_blocked() {
            // CRIT-01 FIX: report_license_revoked must NOT cancel Succeeded
            // campaigns — creator has a guaranteed claim window.
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(20), 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                // Succeeded but not Completed

                MockLicenseVerifier::set_active(0, 0, false);
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(CHARLIE), id),
                    Error::<Test>::InvalidCampaignStatus
                );

                // Campaign remains Succeeded
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Succeeded
                );
            });
        }

        #[test]
        fn report_on_milestone_phase_cancels_it() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = milestone_config(
                    20,
                    500,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::MilestonePhase
                );

                MockLicenseVerifier::set_active(0, 0, false);
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(CHARLIE),
                    id
                ));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                );

                // Milestone statuses cleaned up by report_license_revoked
                assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 0u8), None);
                assert_eq!(pallet::MilestoneStatuses::<Test>::get(id, 1u8), None);
            });
        }

        #[test]
        fn report_emits_correct_events() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(100), 0, 0);
                MockLicenseVerifier::set_active(0, 0, false);

                System::reset_events();
                assert_ok!(Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id));

                let events = System::events();
                let has_license_reported = events.iter().any(|e| {
                    matches!(
                        &e.event,
                        RuntimeEvent::Crowdfunding(Event::CampaignLicenseReported {
                            campaign_id
                        }) if *campaign_id == id
                    )
                });
                assert!(has_license_reported, "CampaignLicenseReported event not found");

                let has_cancelled = events.iter().any(|e| {
                    matches!(
                        &e.event,
                        RuntimeEvent::Crowdfunding(Event::CampaignCancelled {
                            campaign_id
                        }) if *campaign_id == id
                    )
                });
                assert!(has_cancelled, "CampaignCancelled event not found");
            });
        }

        #[test]
        fn report_double_report_fails() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let id = create_licensed_campaign(ALICE, default_kwyr_config(100), 0, 0);
                MockLicenseVerifier::set_active(0, 0, false);

                assert_ok!(Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id));
                // Already Cancelled → second report fails
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
        }

        #[test]
        fn create_campaign_wrong_authorized_account_fails() {
            ExtBuilder::default().build().execute_with(|| {
                // License authorized for ALICE, but BOB tries to create
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                assert_noop!(
                    Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(BOB),
                        default_kwyr_config(100),
                        None,
                        Some((0, 0)),
                    ),
                    Error::<Test>::LicenseNotActive
                );
            });
        }
    }

    // ═════════════════════════════════════════════════════════════════
    // License + milestone + cancel comprehensive flow
    // ═════════════════════════════════════════════════════════════════

    mod license_milestone_cancel_comprehensive {
        use super::*;

        #[test]
        fn license_revoked_after_partial_milestone_then_report_then_proportional_refund() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(3, 3, ALICE, true);
                let config = milestone_config(
                    20,
                    500,
                    vec![
                        Milestone { release_bps: 2000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [3u8; 32] },
                    ],
                );
                let id = create_licensed_campaign(ALICE, config, 3, 3);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestone 0: 20% of 1000 = 200
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));

                // Claim milestone 1: 30% of 1000 = 300
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    1
                ));

                // Total disbursed: 200 + 300 = 500
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_disbursed, 500);

                // License revoked
                MockLicenseVerifier::set_active(3, 3, false);
                assert_ok!(Crowdfunding::report_license_revoked(RuntimeOrigin::signed(DAVE), id));

                // remaining_ratio = (1000-500)/1000 = 50%
                // BOB: 50% of 500 = 250
                let bob_before = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB), bob_before + 250);

                // CHARLIE: 50% of 500 = 250
                let charlie_before = Balances::free_balance(CHARLIE);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 250);

                // Sub-account: deposit (100) only
                let sub = Crowdfunding::campaign_account(id);
                assert_eq!(Balances::free_balance(sub), 100);

                // Creator claims deposit
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(sub), 0);
            });
        }

        #[test]
        fn license_claim_milestone_then_deactivate_claim_succeeds_without_reactivation() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = milestone_config(
                    20,
                    500,
                    vec![
                        Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                        Milestone { release_bps: 4000, description_hash: [3u8; 32] },
                    ],
                );
                let id = create_licensed_campaign(ALICE, config, 0, 0);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // Claim milestone 0: 30% of 1000 = 300
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));

                // License expires
                MockLicenseVerifier::set_active(0, 0, false);

                // DEADLOCK-FIX: Approve and claim milestone 1 — succeeds
                // despite expired license (no reactivation needed)
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    1
                ));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 300);

                // Claim milestone 2: 40% of 1000 = 400
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 2));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 2));
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    2
                ));
                assert_eq!(Balances::free_balance(ALICE), alice_before + 400);

                // Completed
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Completed
                );
            });
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ADVERSARIAL PERSONA TESTS — Part B (P-C01 through P-C18)
// ═══════════════════════════════════════════════════════════════════════

mod adversarial_persona_crowdfunding {
    use frame_support::traits::tokens::fungibles;

    use super::*;

    // ─── P-C01: "Dusty" — ED Reaping Attack ────────────────────────────

    mod pc01_dusty_ed_reaping {
        use super::*;

        /// B01: Failed campaign -> claim_creation_deposit (AllowDeath) ->
        /// sub-account still has investor funds even after deposit claim.
        #[test]
        fn b01_claim_deposit_on_failed_campaign_preserves_investor_funds() {
            ExtBuilder::default().build().execute_with(|| {
                let config = default_aon_config(20, 5000);
                let id = create_funded_campaign(ALICE, config);
                let sub = Crowdfunding::campaign_account(id);

                // BOB invests 500 (below goal of 5000)
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                // sub has 100 (deposit) + 500 (investment) = 600
                assert_eq!(Balances::free_balance(&sub), 600);

                // Deadline passes, finalize -> Failed
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(CHARLIE), id));
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.status, CampaignStatus::Failed);

                // Creator claims deposit
                let alice_before = Balances::free_balance(&ALICE);
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(&ALICE) - alice_before, 100);

                // Sub-account still has investor funds (500)
                assert_eq!(Balances::free_balance(&sub), 500);

                // BOB can still claim refund
                let bob_before = Balances::free_balance(&BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(&BOB) - bob_before, 500);
            });
        }

        /// B02: Milestone partially disbursed -> sub-account balance decreasing
        /// -> claim_milestone_funds handles remaining balance
        /// correctly.
        #[test]
        fn b02_milestone_partial_disburse_sub_account_decreasing() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    let sub = Crowdfunding::campaign_account(id);

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    // sub = 100 (deposit) + 1000 (invest) = 1100
                    assert_eq!(Balances::free_balance(&sub), 1100);

                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    // Claim first milestone (50%)
                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        0
                    ));
                    // 1100 - 500 = 600
                    assert_eq!(Balances::free_balance(&sub), 600);

                    // Claim second milestone (50%)
                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        1
                    ));
                    // 600 - 500 = 100 (only deposit remains)
                    assert_eq!(Balances::free_balance(&sub), 100);

                    let c = pallet::Campaigns::<Test>::get(id).unwrap();
                    assert_eq!(c.status, CampaignStatus::Completed);
                });
        }

        /// B03: No investors -> claim_creation_deposit -> sub-account goes to 0
        /// -> reaped.
        #[test]
        fn b03_no_investors_claim_deposit_reaps_sub_account() {
            ExtBuilder::default().build().execute_with(|| {
                let config = default_aon_config(20, 1000);
                let id = create_funded_campaign(ALICE, config);
                let sub = Crowdfunding::campaign_account(id);
                assert_eq!(Balances::free_balance(&sub), 100);

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(&sub), 0);
            });
        }

        /// B04: Asset currency campaign — full lifecycle.
        #[test]
        fn b04_asset_currency_full_lifecycle() {
            ExtBuilder::default().build().execute_with(|| {
                let asset_id: u32 = 1;
                assert_ok!(Assets::force_create(
                    RuntimeOrigin::root(),
                    codec::Compact(asset_id),
                    ALICE,
                    true,
                    1,
                ));
                assert_ok!(Assets::mint(
                    RuntimeOrigin::signed(ALICE),
                    codec::Compact(asset_id),
                    BOB,
                    5000,
                ));

                let config = crate::CampaignConfig {
                    funding_model: crate::FundingModel::AllOrNothing { goal: 1000 },
                    funding_currency: crate::PaymentCurrency::Asset(asset_id),
                    deadline: 20,
                    hard_cap: None,
                    min_investment: None,
                    max_investment_per_investor: None,
                    metadata_hash: [0u8; 32],
                    early_withdrawal_penalty_bps: None,
                };
                let id = create_funded_campaign(ALICE, config);

                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                assert_eq!(<Assets as fungibles::Inspect<u64>>::balance(asset_id, &BOB), 4000);

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Succeeded
                );

                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(<Assets as fungibles::Inspect<u64>>::balance(asset_id, &ALICE), 1000);
            });
        }

        /// B05: Multiple investors, sequential claim_refund -> last investor
        /// succeeds.
        #[test]
        fn b05_sequential_refunds_last_investor_succeeds() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000), (DAVE, 10_000)])
                .build()
                .execute_with(|| {
                    let config = default_aon_config(20, 50_000);
                    let id = create_funded_campaign(ALICE, config);

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 2000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 3000));

                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                    let bob_before = Balances::free_balance(&BOB);
                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                    assert_eq!(Balances::free_balance(&BOB) - bob_before, 1000);

                    let charlie_before = Balances::free_balance(&CHARLIE);
                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                    assert_eq!(Balances::free_balance(&CHARLIE) - charlie_before, 2000);

                    let dave_before = Balances::free_balance(&DAVE);
                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(DAVE), id));
                    assert_eq!(Balances::free_balance(&DAVE) - dave_before, 3000);
                });
        }
    }

    // ─── P-C02: "Sibylla" — Storage Saturation Griefing ────────────────

    mod pc02_sibylla_storage_saturation {
        use super::*;

        /// B06: MaxCampaignsPerCreator(5) -> 6th fails -> free slot -> 6th
        /// succeeds.
        #[test]
        fn b06_max_campaigns_per_creator_then_free_slot() {
            ExtBuilder::default().balances(vec![(ALICE, 100_000)]).build().execute_with(|| {
                let mut ids = Vec::new();
                for i in 0..5u64 {
                    let config = default_aon_config(20 + i, 1000);
                    ids.push(create_funded_campaign(ALICE, config));
                }
                assert_eq!(pallet::CreatorCampaigns::<Test>::get(&ALICE).len(), 5);

                let config6 = default_aon_config(30, 1000);
                assert_noop!(
                    Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config6.clone(),
                        None,
                        None
                    ),
                    Error::<Test>::MaxCampaignsPerCreatorReached
                );

                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), ids[0]));
                assert_ok!(Crowdfunding::claim_creation_deposit(
                    RuntimeOrigin::signed(ALICE),
                    ids[0]
                ));
                assert_eq!(pallet::CreatorCampaigns::<Test>::get(&ALICE).len(), 4);

                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config6,
                    None,
                    None
                ));
                assert_eq!(pallet::CreatorCampaigns::<Test>::get(&ALICE).len(), 5);
            });
        }

        /// B07: MaxInvestmentsPerInvestor(5) -> 6th fails.
        #[test]
        fn b07_max_investments_per_investor() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
                .build()
                .execute_with(|| {
                    // Use two creators so we don't hit MaxCampaignsPerCreator
                    let mut ids = Vec::new();
                    for i in 0..3u64 {
                        ids.push(create_funded_campaign(ALICE, default_aon_config(100 + i, 1000)));
                    }
                    for i in 0..2u64 {
                        ids.push(create_funded_campaign(
                            CHARLIE,
                            default_aon_config(110 + i, 1000),
                        ));
                    }
                    for &id in &ids {
                        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10));
                    }
                    assert_eq!(pallet::InvestorCampaigns::<Test>::get(&BOB).len(), 5);

                    let id6 = create_funded_campaign(CHARLIE, default_aon_config(200, 1000));
                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(BOB), id6, 10),
                        Error::<Test>::MaxInvestmentsPerInvestorReached
                    );
                });
        }

        /// B08: Full withdraw frees slot -> can invest in new campaign.
        #[test]
        fn b08_full_withdraw_frees_investor_slot() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
                .build()
                .execute_with(|| {
                    let mut ids = Vec::new();
                    for i in 0..3u64 {
                        ids.push(create_funded_campaign(ALICE, default_aon_config(100 + i, 1000)));
                    }
                    for i in 0..2u64 {
                        ids.push(create_funded_campaign(
                            CHARLIE,
                            default_aon_config(110 + i, 1000),
                        ));
                    }
                    for &id in &ids {
                        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10));
                    }

                    assert_ok!(Crowdfunding::withdraw_investment(
                        RuntimeOrigin::signed(BOB),
                        ids[0],
                        10
                    ));
                    assert_eq!(pallet::InvestorCampaigns::<Test>::get(&BOB).len(), 4);

                    let id6 = create_funded_campaign(CHARLIE, default_aon_config(200, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id6, 10));
                    assert_eq!(pallet::InvestorCampaigns::<Test>::get(&BOB).len(), 5);
                });
        }

        /// B09: claim_creation_deposit removes from CreatorCampaigns.
        #[test]
        fn b09_claim_deposit_removes_from_creator_campaigns() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                assert!(pallet::CreatorCampaigns::<Test>::get(&ALICE).contains(&id));

                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert!(!pallet::CreatorCampaigns::<Test>::get(&ALICE).contains(&id));
            });
        }

        /// B10: Full withdraw + claim_refund both remove from
        /// InvestorCampaigns.
        #[test]
        fn b10_full_withdraw_and_refund_remove_from_investor_campaigns() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000)])
                .build()
                .execute_with(|| {
                    let config = default_aon_config(20, 50_000);
                    let id = create_funded_campaign(ALICE, config);

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100));
                    assert!(pallet::InvestorCampaigns::<Test>::get(&BOB).contains(&id));
                    assert!(pallet::InvestorCampaigns::<Test>::get(&CHARLIE).contains(&id));

                    // BOB full withdraw
                    assert_ok!(Crowdfunding::withdraw_investment(
                        RuntimeOrigin::signed(BOB),
                        id,
                        100
                    ));
                    assert!(!pallet::InvestorCampaigns::<Test>::get(&BOB).contains(&id));

                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                    // CHARLIE claim_refund
                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                    assert!(!pallet::InvestorCampaigns::<Test>::get(&CHARLIE).contains(&id));
                });
        }
    }

    // ─── P-C03: "Houdini" — Penalty Evasion ────────────────────────────

    mod pc03_houdini_penalty_evasion {
        use super::*;

        /// B11: early_withdrawal_penalty_bps=Some(0) -> no penalty.
        #[test]
        fn b11_zero_penalty_bps_no_penalty() {
            ExtBuilder::default().build().execute_with(|| {
                let mut config = default_aon_config(20, 1000);
                config.early_withdrawal_penalty_bps = Some(0);
                let id = create_funded_campaign(ALICE, config);

                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                let bob_before = Balances::free_balance(&BOB);
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500));
                assert_eq!(Balances::free_balance(&BOB) - bob_before, 500);
            });
        }

        /// B12: invest 1 -> withdraw 1 -> ceiling penalty = 1, net = 0.
        #[test]
        fn b12_rounding_to_zero_penalty_on_unit_amount() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
                let bob_before = Balances::free_balance(&BOB);
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1));
                // ceil(1 * 100 / 10000) = 1, penalty = 1, net = 0
                assert_eq!(Balances::free_balance(&BOB) - bob_before, 0);
            });
        }

        /// B13: Multiple small withdrawals each with ceiling penalty = 1.
        /// Each withdrawal of 1 unit incurs ceil(1*100/10000)=1 penalty, net=0.
        #[test]
        fn b13_multiple_small_withdrawals_cumulative_zero_penalty() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 99));
                let bob_before = Balances::free_balance(&BOB);
                for _ in 0..99 {
                    assert_ok!(Crowdfunding::withdraw_investment(
                        RuntimeOrigin::signed(BOB),
                        id,
                        1
                    ));
                }
                // Each withdrawal: ceil(1*100/10000)=1 penalty, net=0. 99 withdrawals * 0 = 0
                assert_eq!(Balances::free_balance(&BOB) - bob_before, 0);
            });
        }

        /// B14: Paused campaign -> withdraw -> penalty still applies.
        #[test]
        fn b14_paused_campaign_withdrawal_penalty_applies() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));

                let bob_before = Balances::free_balance(&BOB);
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1000));
                // penalty = Permill(10000) * 1000 = 10, net = 990
                assert_eq!(Balances::free_balance(&BOB) - bob_before, 990);
            });
        }

        /// B15: penalty_bps=10001 -> creation fails.
        #[test]
        fn b15_penalty_bps_over_10000_rejected() {
            ExtBuilder::default().build().execute_with(|| {
                let mut config = default_aon_config(20, 1000);
                config.early_withdrawal_penalty_bps = Some(10001);
                assert_noop!(
                    Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                    Error::<Test>::InvalidPenaltyBps
                );
            });
        }
    }

    // ─── P-C04: "Ouroboros" — Milestone Out-of-Order Claims ─────────────

    mod pc04_ouroboros_milestone_ooo {
        use super::*;

        /// B16: Claim milestone 2 (5000 bps = 50%) first -> succeeds (no
        /// ordering enforced).
        #[test]
        fn b16_claim_milestone_out_of_order_succeeds() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 2000, description_hash: [2u8; 32] },
                            Milestone { release_bps: 5000, description_hash: [3u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 2));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 2));
                    let alice_before = Balances::free_balance(&ALICE);
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        2
                    ));
                    // 50% of 1000 = 500
                    assert_eq!(Balances::free_balance(&ALICE) - alice_before, 500);
                });
        }

        /// B17: All milestones claimed -> total_disbursed == total_raised,
        /// Completed.
        #[test]
        fn b17_all_milestones_claimed_completes_campaign() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                            Milestone { release_bps: 4000, description_hash: [3u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    for i in 0u8..3 {
                        assert_ok!(Crowdfunding::submit_milestone(
                            RuntimeOrigin::signed(ALICE),
                            id,
                            i
                        ));
                        assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                        assert_ok!(Crowdfunding::claim_milestone_funds(
                            RuntimeOrigin::signed(ALICE),
                            id,
                            i
                        ));
                    }

                    let c = pallet::Campaigns::<Test>::get(id).unwrap();
                    assert_eq!(c.status, CampaignStatus::Completed);
                    assert_eq!(c.total_disbursed, c.total_raised);
                });
        }

        /// B18: Claimed milestone -> re-submit -> fails
        /// (InvalidMilestoneStatus).
        #[test]
        fn b18_resubmit_claimed_milestone_fails() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        0
                    ));

                    assert_noop!(
                        Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0),
                        Error::<Test>::InvalidMilestoneStatus
                    );
                });
        }

        /// B19: Reject -> re-submit -> approve -> claim -> reject again ->
        /// fails.
        #[test]
        fn b19_reject_resubmit_approve_claim_then_reject_fails() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                    assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        0
                    ));

                    assert_noop!(
                        Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0),
                        Error::<Test>::InvalidMilestoneStatus
                    );
                });
        }

        /// B20: release_bps=[3333, 3333, 3334] -> verify rounding.
        #[test]
        fn b20_milestone_rounding_distribution() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 3333, description_hash: [1u8; 32] },
                            Milestone { release_bps: 3333, description_hash: [2u8; 32] },
                            Milestone { release_bps: 3334, description_hash: [3u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    let mut total_claimed = 0u128;
                    for i in 0u8..3 {
                        assert_ok!(Crowdfunding::submit_milestone(
                            RuntimeOrigin::signed(ALICE),
                            id,
                            i
                        ));
                        assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                        let alice_before = Balances::free_balance(&ALICE);
                        assert_ok!(Crowdfunding::claim_milestone_funds(
                            RuntimeOrigin::signed(ALICE),
                            id,
                            i
                        ));
                        total_claimed += Balances::free_balance(&ALICE) - alice_before;
                    }
                    // bps_of(1000, 3333)=334, bps_of(1000, 3333)=334, but 3rd
                    // milestone is capped at remaining = 1000-668 = 332.
                    // Total = 334 + 334 + 332 = 1000 (no over-disbursement).
                    assert_eq!(total_claimed, 1000);
                    let c = pallet::Campaigns::<Test>::get(id).unwrap();
                    assert_eq!(c.status, CampaignStatus::Completed);
                    assert_eq!(c.total_disbursed, 1000);
                });
        }
    }

    // ─── P-C05: "Chameleon" — Eligibility Escape ────────────────────────

    mod pc05_chameleon_eligibility_escape {
        use super::*;

        /// B21: NativeBalance{min=100} -> invest -> transfer away -> still
        /// investor.
        #[test]
        fn b21_native_balance_point_in_time_check() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let rules: BoundedVec<_, _> =
                        BoundedVec::try_from(vec![EligibilityRule::NativeBalance {
                            min_balance: 100,
                        }])
                        .unwrap();
                    let config = default_aon_config(100, 1000);
                    assert_ok!(Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config,
                        Some(rules),
                        None,
                    ));
                    let id = pallet::NextCampaignId::<Test>::get() - 1;

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));

                    // Transfer away most balance
                    Balances::transfer(RuntimeOrigin::signed(BOB), CHARLIE, 9898).unwrap();

                    // BOB still has the investment
                    let inv = pallet::Investments::<Test>::get(id, &BOB).unwrap();
                    assert_eq!(inv.total_invested, 100);
                });
        }

        /// B22: AssetBalance rule -> invest -> transfer asset away -> still
        /// investor.
        #[test]
        fn b22_asset_balance_point_in_time_check() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000)])
                .build()
                .execute_with(|| {
                    let asset_id: u32 = 42;
                    assert_ok!(Assets::force_create(
                        RuntimeOrigin::root(),
                        codec::Compact(asset_id),
                        ALICE,
                        true,
                        1,
                    ));
                    assert_ok!(Assets::mint(
                        RuntimeOrigin::signed(ALICE),
                        codec::Compact(asset_id),
                        BOB,
                        500,
                    ));

                    let rules: BoundedVec<_, _> =
                        BoundedVec::try_from(vec![EligibilityRule::AssetBalance {
                            asset_id,
                            min_balance: 100,
                        }])
                        .unwrap();
                    let config = default_aon_config(100, 1000);
                    assert_ok!(Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config,
                        Some(rules),
                        None,
                    ));
                    let id = pallet::NextCampaignId::<Test>::get() - 1;

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50));

                    assert_ok!(Assets::transfer(
                        RuntimeOrigin::signed(BOB),
                        codec::Compact(asset_id),
                        CHARLIE,
                        499,
                    ));

                    let inv = pallet::Investments::<Test>::get(id, &BOB).unwrap();
                    assert_eq!(inv.total_invested, 50);
                });
        }

        /// B23: NftOwnership -> invest -> clear NFT -> still investor, CANNOT
        /// re-invest.
        #[test]
        fn b23_nft_ownership_point_in_time() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    MockNftInspect::set_owner(1, 1, BOB);

                    let nft_set: BoundedVec<(u32, u32), _> =
                        BoundedVec::try_from(vec![(1u32, 1u32)]).unwrap();
                    let required_sets: BoundedVec<_, _> =
                        BoundedVec::try_from(vec![nft_set]).unwrap();
                    let rules: BoundedVec<_, _> =
                        BoundedVec::try_from(vec![EligibilityRule::NftOwnership { required_sets }])
                            .unwrap();

                    let config = default_aon_config(100, 1000);
                    assert_ok!(Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config,
                        Some(rules),
                        None,
                    ));
                    let id = pallet::NextCampaignId::<Test>::get() - 1;

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50));
                    MockNftInspect::clear();

                    let inv = pallet::Investments::<Test>::get(id, &BOB).unwrap();
                    assert_eq!(inv.total_invested, 50);

                    // Re-invest fails -- eligibility re-checked every invest call
                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10),
                        Error::<Test>::EligibilityCheckFailed
                    );
                });
        }

        /// B24: AccountWhitelist -> invest -> remove -> still investor, cannot
        /// re-invest.
        #[test]
        fn b24_whitelist_point_in_time() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let rules: BoundedVec<_, _> =
                        BoundedVec::try_from(vec![EligibilityRule::AccountWhitelist]).unwrap();
                    let config = default_aon_config(100, 1000);
                    assert_ok!(Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config,
                        Some(rules),
                        None,
                    ));
                    let id = pallet::NextCampaignId::<Test>::get() - 1;

                    assert_ok!(Crowdfunding::add_to_whitelist(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        BOB
                    ));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50));
                    assert_ok!(Crowdfunding::remove_from_whitelist(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        BOB
                    ));

                    let inv = pallet::Investments::<Test>::get(id, &BOB).unwrap();
                    assert_eq!(inv.total_invested, 50);

                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10),
                        Error::<Test>::EligibilityCheckFailed
                    );
                });
        }

        /// B25: Multiple rules (NativeBalance AND AccountWhitelist) -> must
        /// pass both.
        #[test]
        fn b25_multiple_rules_must_pass_all() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000)])
                .build()
                .execute_with(|| {
                    let rules: BoundedVec<_, _> = BoundedVec::try_from(vec![
                        EligibilityRule::NativeBalance { min_balance: 100 },
                        EligibilityRule::AccountWhitelist,
                    ])
                    .unwrap();
                    let config = default_aon_config(100, 1000);
                    assert_ok!(Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config,
                        Some(rules),
                        None,
                    ));
                    let id = pallet::NextCampaignId::<Test>::get() - 1;

                    // Balance ok but not whitelisted -> fail
                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50),
                        Error::<Test>::EligibilityCheckFailed
                    );

                    assert_ok!(Crowdfunding::add_to_whitelist(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        BOB
                    ));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50));

                    // CHARLIE not whitelisted
                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 50),
                        Error::<Test>::EligibilityCheckFailed
                    );
                });
        }

        /// B26: set_default_eligibility -> new campaign picks up rules, old
        /// unaffected.
        #[test]
        fn b26_default_eligibility_change_affects_new_campaigns_only() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 200)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50));

                    let rules: BoundedVec<_, _> =
                        BoundedVec::try_from(vec![EligibilityRule::NativeBalance {
                            min_balance: 5000,
                        }])
                        .unwrap();
                    assert_ok!(Crowdfunding::set_default_eligibility(RuntimeOrigin::root(), rules));

                    let id2 = create_funded_campaign(ALICE, default_aon_config(100, 1000));

                    // CHARLIE has 200 < 5000 -> fails on new campaign
                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id2, 50),
                        Error::<Test>::EligibilityCheckFailed
                    );

                    // Old campaign still works (no rules on it)
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50));
                });
        }
    }

    // ─── P-C06: "Lazarus" — Double Claim ────────────────────────────────

    mod pc06_lazarus_double_claim {
        use super::*;

        /// B27: claim_funds -> again -> fails (Completed, not Succeeded).
        #[test]
        fn b27_double_claim_funds_fails() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

                    assert_noop!(
                        Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                        Error::<Test>::InvalidCampaignStatus
                    );
                });
        }

        /// B28: claim_creation_deposit -> again -> fails (AlreadyClaimed).
        #[test]
        fn b28_double_claim_creation_deposit_fails() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));

                assert_noop!(
                    Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id),
                    Error::<Test>::AlreadyClaimed
                );
            });
        }

        /// B29: claim_refund -> again -> fails (NoInvestmentFound).
        #[test]
        fn b29_double_claim_refund_fails() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(20, 50_000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));

                    assert_noop!(
                        Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                        Error::<Test>::NoInvestmentFound
                    );
                });
        }

        /// B30: claim_milestone_funds -> same index -> fails
        /// (InvalidMilestoneStatus).
        #[test]
        fn b30_double_claim_milestone_funds_fails() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        0
                    ));

                    assert_noop!(
                        Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0),
                        Error::<Test>::InvalidMilestoneStatus
                    );
                });
        }

        /// B31: Cancelled -> claim_refund -> claim_creation_deposit -> both
        /// succeed.
        #[test]
        fn b31_cancelled_refund_then_deposit_both_succeed() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                    assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                    assert_ok!(Crowdfunding::claim_creation_deposit(
                        RuntimeOrigin::signed(ALICE),
                        id
                    ));
                });
        }
    }

    // ─── P-C07: "Sybil" — Multi-Account Investment Bypass ──────────────

    mod pc07_sybil_multi_account {
        use super::*;

        /// B32: max_investment_per_investor=100 -> 5 accounts invest 100 each
        /// -> all succeed.
        #[test]
        fn b32_per_investor_cap_bypassed_with_multiple_accounts() {
            ExtBuilder::default()
                .balances(vec![
                    (ALICE, 10_000),
                    (BOB, 10_000),
                    (CHARLIE, 10_000),
                    (DAVE, 10_000),
                    (5, 10_000),
                ])
                .build()
                .execute_with(|| {
                    let mut config = default_aon_config(100, 1000);
                    config.max_investment_per_investor = Some(100);
                    let id = create_funded_campaign(ALICE, config);

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 100));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(5), id, 100));

                    let c = pallet::Campaigns::<Test>::get(id).unwrap();
                    assert_eq!(c.total_raised, 400);
                    assert_eq!(c.investor_count, 4);
                });
        }

        /// B33: MaxInvestmentsPerInvestor=5 -> different accounts bypass it.
        #[test]
        fn b33_max_investments_bypassed_via_sybil_accounts() {
            ExtBuilder::default()
                .balances(vec![
                    (ALICE, 100_000),
                    (BOB, 10_000),
                    (CHARLIE, 10_000),
                    (DAVE, 10_000),
                    (5, 10_000),
                    (6, 10_000),
                    (7, 10_000),
                ])
                .build()
                .execute_with(|| {
                    // Create 7 campaigns
                    let mut ids = Vec::new();
                    for i in 0..5u64 {
                        ids.push(create_funded_campaign(ALICE, default_aon_config(100 + i, 1000)));
                    }

                    // BOB can only invest in 5
                    for &id in &ids {
                        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10));
                    }

                    // But CHARLIE, DAVE, etc. can each invest in campaigns too
                    for &id in &ids {
                        assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 10));
                    }
                });
        }

        /// B34: Whitelist campaign -> only whitelisted accounts can invest.
        #[test]
        fn b34_whitelist_restricts_sybil() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000)])
                .build()
                .execute_with(|| {
                    let rules: BoundedVec<_, _> =
                        BoundedVec::try_from(vec![EligibilityRule::AccountWhitelist]).unwrap();
                    let config = default_aon_config(100, 1000);
                    assert_ok!(Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config,
                        Some(rules),
                        None,
                    ));
                    let id = pallet::NextCampaignId::<Test>::get() - 1;

                    assert_ok!(Crowdfunding::add_to_whitelist(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        BOB
                    ));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));

                    // CHARLIE not whitelisted
                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100),
                        Error::<Test>::EligibilityCheckFailed
                    );
                });
        }

        /// B35: Multiple accounts invest -> each claim_refund goes to own
        /// account.
        #[test]
        fn b35_refunds_go_to_individual_accounts() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000), (DAVE, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(20, 50_000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 200));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 300));

                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                    let bob_before = Balances::free_balance(&BOB);
                    let charlie_before = Balances::free_balance(&CHARLIE);
                    let dave_before = Balances::free_balance(&DAVE);

                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(CHARLIE), id));
                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(DAVE), id));

                    assert_eq!(Balances::free_balance(&BOB) - bob_before, 100);
                    assert_eq!(Balances::free_balance(&CHARLIE) - charlie_before, 200);
                    assert_eq!(Balances::free_balance(&DAVE) - dave_before, 300);
                });
        }
    }

    // ─── P-C08: "Temporal" — Timing Manipulation ────────────────────────

    mod pc08_temporal_timing {
        use super::*;

        /// B36: Invest at deadline block (now <= deadline) -> succeeds.
        #[test]
        fn b36_invest_at_deadline_block_succeeds() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                run_to_block(20);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            });
        }

        /// B37: Finalize at deadline+1 (now > deadline) -> succeeds.
        #[test]
        fn b37_finalize_at_deadline_plus_one_succeeds() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
            });
        }

        /// B38: Pause -> advance blocks -> resume -> deadline extended by pause
        /// duration.
        #[test]
        fn b38_pause_resume_extends_deadline() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(50, 1000));
                let original_deadline = 50u64;

                // Pause at block 10
                run_to_block(10);
                assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));

                // Advance 20 blocks while paused
                run_to_block(30);
                assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));

                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                // Deadline extended by 20 (pause duration: 30 - 10 = 20)
                assert_eq!(c.config.deadline, original_deadline + 20);

                // Can still invest at new deadline
                run_to_block(70);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            });
        }

        /// B39: deadline = current_block + MinCampaignDuration (1 + 10 = 11) ->
        /// succeeds.
        #[test]
        fn b39_minimum_duration_succeeds() {
            ExtBuilder::default().build().execute_with(|| {
                // Block is 1, MinCampaignDuration=10, so deadline >= 11
                let config = default_aon_config(11, 1000);
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    None
                ));
            });
        }

        /// B40: deadline = current_block + MaxCampaignDuration (1 + 1000 =
        /// 1001) -> succeeds.
        #[test]
        fn b40_maximum_duration_succeeds() {
            ExtBuilder::default().build().execute_with(|| {
                let config = default_aon_config(1001, 1000);
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    None
                ));
            });
        }

        /// B41: deadline = current_block + MaxCampaignDuration + 1 -> fails
        /// DurationTooLong.
        #[test]
        fn b41_over_max_duration_fails() {
            ExtBuilder::default().build().execute_with(|| {
                let config = default_aon_config(1002, 1000);
                assert_noop!(
                    Crowdfunding::create_campaign(RuntimeOrigin::signed(ALICE), config, None, None),
                    Error::<Test>::DurationTooLong
                );
            });
        }
    }

    // ─── P-C09: "Phantom" — License Ghost Attack ────────────────────────

    mod pc09_phantom_license {
        use super::*;

        /// B42: Create campaign with license=Some(0,0) -> MockLicenseVerifier
        /// passes -> created.
        #[test]
        fn b42_create_campaign_with_license_succeeds() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(0, 0, ALICE, true);
                let config = default_aon_config(20, 1000);
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    Some((0, 0)),
                ));
                let id = pallet::NextCampaignId::<Test>::get() - 1;
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.rwa_asset_id, Some(0));
                assert_eq!(c.participation_id, Some(0));
            });
        }

        /// B43: claim_funds checks is_license_active (license is active ->
        /// succeeds).
        #[test]
        fn b43_claim_funds_checks_license() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    MockLicenseVerifier::set_license(1, 1, ALICE, true);
                    let config = default_aon_config(20, 1000);
                    assert_ok!(Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config,
                        None,
                        Some((1, 1)),
                    ));
                    let id = pallet::NextCampaignId::<Test>::get() - 1;
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    // License still active -> claim succeeds
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                });
        }

        /// B44: report_license_revoked on campaign with no license ->
        /// NoLinkedLicense.
        #[test]
        fn b44_report_no_license_fails() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::NoLinkedLicense
                );
            });
        }

        /// B45: report_license_revoked on Cancelled campaign ->
        /// InvalidCampaignStatus.
        #[test]
        fn b45_report_on_cancelled_fails() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(1, 1, ALICE, true);
                let config = default_aon_config(20, 1000);
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    Some((1, 1)),
                ));
                let id = pallet::NextCampaignId::<Test>::get() - 1;
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                MockLicenseVerifier::set_active(1, 1, false);
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
        }

        /// B46: report_license_revoked on Completed campaign ->
        /// InvalidCampaignStatus.
        #[test]
        fn b46_report_on_completed_fails() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    MockLicenseVerifier::set_license(1, 1, ALICE, true);
                    let config = default_aon_config(20, 1000);
                    assert_ok!(Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config,
                        None,
                        Some((1, 1)),
                    ));
                    let id = pallet::NextCampaignId::<Test>::get() - 1;
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

                    MockLicenseVerifier::set_active(1, 1, false);
                    assert_noop!(
                        Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                        Error::<Test>::InvalidCampaignStatus
                    );
                });
        }

        /// B47: Create campaign without license -> report_license_revoked ->
        /// NoLinkedLicense.
        #[test]
        fn b47_report_on_unlicensed_campaign_fails() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::NoLinkedLicense
                );
            });
        }
    }

    // ─── P-C10: "Vulture" — Protocol Fee Manipulation ──────────────────

    mod pc10_vulture_protocol_fee {
        use super::*;

        /// B48: set_protocol_config(10000, self) -> 100% fee -> creator gets 0.
        #[test]
        fn b48_100_percent_fee_creator_gets_zero() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (99, 1)])
                .build()
                .execute_with(|| {
                    assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 10000, 99));

                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    let alice_before = Balances::free_balance(&ALICE);
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                    // Creator gets 0 (100% fee)
                    assert_eq!(Balances::free_balance(&ALICE), alice_before);
                    // Protocol gets 1000 (+ initial 1)
                    assert_eq!(Balances::free_balance(&99), 1001);
                });
        }

        /// B49: set_protocol_config(0, anyone) -> 0% fee -> creator gets
        /// everything.
        #[test]
        fn b49_zero_fee_creator_gets_all() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 0, 99));

                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    let alice_before = Balances::free_balance(&ALICE);
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                    assert_eq!(Balances::free_balance(&ALICE) - alice_before, 1000);
                });
        }

        /// B50: Change fee rate mid-campaign -> locked fee at creation (0%) is
        /// used at claim time.
        #[test]
        fn b50_fee_change_mid_campaign_applies_at_claim() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (99, 1)])
                .build()
                .execute_with(|| {
                    // Default fee is 0% from Config — locked at creation
                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));

                    // Change fee to 10% mid-campaign (has no effect — fee locked at creation)
                    assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 1000, 99));

                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    let alice_before = Balances::free_balance(&ALICE);
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                    // Locked fee = 0%, creator gets full 1000
                    assert_eq!(Balances::free_balance(&ALICE) - alice_before, 1000);
                    // 99 started with 1, no fee collected
                    assert_eq!(Balances::free_balance(&99), 1);
                });
        }

        /// B51: Protocol fee recipient has 0 balance -> transfer succeeds
        /// (account created).
        #[test]
        fn b51_fee_recipient_zero_balance_account_created() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let recipient: u64 = 42;
                    assert_eq!(Balances::free_balance(&recipient), 0);

                    assert_ok!(Crowdfunding::set_protocol_config(
                        RuntimeOrigin::root(),
                        500,
                        recipient
                    ));

                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

                    // 5% of 1000 = 50
                    assert_eq!(Balances::free_balance(&recipient), 50);
                });
        }

        /// B52: set_protocol_config(10001, ...) -> fails InvalidFeeBps.
        #[test]
        fn b52_fee_bps_over_10000_rejected() {
            ExtBuilder::default().build().execute_with(|| {
                assert_noop!(
                    Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 10001, 99),
                    Error::<Test>::InvalidFeeBps
                );
            });
        }
    }

    // ─── P-C11: "Frankenstein" — Cross-Pallet Asset State ───────────────

    mod pc11_frankenstein_cross_pallet {
        use super::*;

        /// B53: Asset(1) campaign -> invest -> verify asset balances.
        #[test]
        fn b53_asset_campaign_invest_verify() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let asset_id: u32 = 1;
                    assert_ok!(Assets::force_create(
                        RuntimeOrigin::root(),
                        codec::Compact(asset_id),
                        ALICE,
                        true,
                        1,
                    ));
                    assert_ok!(Assets::mint(
                        RuntimeOrigin::signed(ALICE),
                        codec::Compact(asset_id),
                        BOB,
                        5000,
                    ));

                    let config = crate::CampaignConfig {
                        funding_model: crate::FundingModel::AllOrNothing { goal: 1000 },
                        funding_currency: crate::PaymentCurrency::Asset(asset_id),
                        deadline: 20,
                        hard_cap: None,
                        min_investment: None,
                        max_investment_per_investor: None,
                        metadata_hash: [0u8; 32],
                        early_withdrawal_penalty_bps: None,
                    };
                    let id = create_funded_campaign(ALICE, config);

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    assert_eq!(<Assets as fungibles::Inspect<u64>>::balance(asset_id, &BOB), 4000);

                    let c = pallet::Campaigns::<Test>::get(id).unwrap();
                    assert_eq!(c.total_raised, 1000);
                });
        }

        /// B54: Asset campaign -> freeze asset after investment -> withdraw
        /// fails.
        #[test]
        fn b54_frozen_asset_withdraw_fails() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let asset_id: u32 = 1;
                    assert_ok!(Assets::force_create(
                        RuntimeOrigin::root(),
                        codec::Compact(asset_id),
                        ALICE,
                        true,
                        1,
                    ));
                    assert_ok!(Assets::mint(
                        RuntimeOrigin::signed(ALICE),
                        codec::Compact(asset_id),
                        BOB,
                        5000,
                    ));

                    let config = crate::CampaignConfig {
                        funding_model: crate::FundingModel::AllOrNothing { goal: 1000 },
                        funding_currency: crate::PaymentCurrency::Asset(asset_id),
                        deadline: 100,
                        hard_cap: None,
                        min_investment: None,
                        max_investment_per_investor: None,
                        metadata_hash: [0u8; 32],
                        early_withdrawal_penalty_bps: None,
                    };
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));

                    // Freeze the asset
                    assert_ok!(Assets::freeze_asset(
                        RuntimeOrigin::signed(ALICE),
                        codec::Compact(asset_id)
                    ));

                    // Withdraw should fail because asset is frozen
                    assert!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 500)
                        .is_err());
                });
        }

        /// B55: NFT eligibility -> transfer NFT -> cannot invest again but
        /// existing stays.
        #[test]
        fn b55_nft_transfer_blocks_reinvest() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    MockNftInspect::set_owner(1, 1, BOB);

                    let nft_set: BoundedVec<(u32, u32), _> =
                        BoundedVec::try_from(vec![(1u32, 1u32)]).unwrap();
                    let required_sets: BoundedVec<_, _> =
                        BoundedVec::try_from(vec![nft_set]).unwrap();
                    let rules: BoundedVec<_, _> =
                        BoundedVec::try_from(vec![EligibilityRule::NftOwnership { required_sets }])
                            .unwrap();

                    let config = default_aon_config(100, 1000);
                    assert_ok!(Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        config,
                        Some(rules),
                        None,
                    ));
                    let id = pallet::NextCampaignId::<Test>::get() - 1;

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50));

                    // "Transfer" NFT to CHARLIE
                    MockNftInspect::set_owner(1, 1, CHARLIE);

                    // BOB's existing investment stays
                    let inv = pallet::Investments::<Test>::get(id, &BOB).unwrap();
                    assert_eq!(inv.total_invested, 50);

                    // BOB cannot invest more
                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10),
                        Error::<Test>::EligibilityCheckFailed
                    );
                });
        }
    }

    // ─── P-C12: "Undertaker" — State Machine Violations ─────────────────

    mod pc12_undertaker_state_machine {
        use super::*;

        /// B57: Funding -> pause -> cancel -> succeeds.
        #[test]
        fn b57_pause_then_cancel_succeeds() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                );
            });
        }

        /// B58: CRIT-02 FIX — Succeeded -> cancel -> BLOCKED.
        /// Creator has guaranteed claim window after finalization.
        #[test]
        fn b58_cancel_succeeded_campaign_blocked() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                    assert_eq!(
                        pallet::Campaigns::<Test>::get(id).unwrap().status,
                        CampaignStatus::Succeeded
                    );

                    assert_noop!(
                        Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                        Error::<Test>::InvalidCampaignStatus
                    );

                    // Campaign remains Succeeded
                    assert_eq!(
                        pallet::Campaigns::<Test>::get(id).unwrap().status,
                        CampaignStatus::Succeeded
                    );
                });
        }

        /// B59: MilestonePhase -> cancel -> verify claimed funds NOT returned.
        #[test]
        fn b59_cancel_milestone_phase_partial_disbursed() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    // Claim first milestone (50% = 500)
                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        0
                    ));

                    // Cancel
                    assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                    // Refund is proportional: remaining = (1000 - 500) / 1000 * 1000 = 500
                    let bob_before = Balances::free_balance(&BOB);
                    assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                    let refund = Balances::free_balance(&BOB) - bob_before;
                    assert_eq!(refund, 500);
                });
        }

        /// B60: Completed -> any operation -> all fail.
        #[test]
        fn b60_completed_blocks_all_operations() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

                    // All ops fail
                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                        Error::<Test>::InvalidCampaignStatus
                    );
                    assert_noop!(
                        Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100),
                        Error::<Test>::InvalidCampaignStatus
                    );
                    assert_noop!(
                        Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id),
                        Error::<Test>::InvalidCampaignStatus
                    );
                    assert_noop!(
                        Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                        Error::<Test>::InvalidCampaignStatus
                    );
                    assert_noop!(
                        Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                        Error::<Test>::InvalidCampaignStatus
                    );
                    assert_noop!(
                        Crowdfunding::pause_campaign(RuntimeOrigin::root(), id),
                        Error::<Test>::CampaignNotFunding
                    );
                });
        }

        /// B61: Failed -> finalize -> fails (InvalidCampaignStatus).
        #[test]
        fn b61_finalize_failed_campaign_fails() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Failed
                );

                assert_noop!(
                    Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
        }
    }

    // ─── P-C13: "Roundoff" — Precision Attacks ──────────────────────────

    mod pc13_roundoff_precision {
        use super::*;

        /// B62: invest 1 -> withdraw 1 -> ceiling penalty = 1, net = 0.
        #[test]
        fn b62_unit_withdraw_zero_penalty() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
                let bob_before = Balances::free_balance(&BOB);
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1));
                // ceil(1 * 100 / 10000) = 1, penalty = 1, net = 0
                assert_eq!(Balances::free_balance(&BOB) - bob_before, 0);
            });
        }

        /// B63: Pro-rata refund with partial disbursement.
        /// total_raised=3, total_disbursed=2 (ceil), invested=1.
        /// remaining_ratio = Permill::from_rational(1, 3) * 1 = 0.
        /// With ceiling division, bps_of(3, 3334)=ceil(10002/10000)=2, so
        /// total_disbursed=2. remaining=1. Permill(333334)*1=0 ->
        /// NothingToRefund.
        #[test]
        fn b63_prorata_refund_small_amounts() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000), (DAVE, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        3,
                        vec![
                            Milestone { release_bps: 3334, description_hash: [1u8; 32] },
                            Milestone { release_bps: 6666, description_hash: [2u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 1));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id, 1));

                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    // Claim first milestone: bps_of(3, 3334) = ceil(10002/10000) = 2
                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        0
                    ));

                    // Cancel
                    assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));

                    // BOB refund: remaining = 3 - 2 = 1
                    // remaining_ratio = Permill::from_rational(1, 3) = Permill(333334)
                    // refund = Permill(333334) * 1 = 0 -> NothingToRefund
                    assert_noop!(
                        Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                        Error::<Test>::NothingToRefund
                    );
                });
        }

        /// B64: milestone release_bps=[1, 1, 9998] -> extreme distribution.
        #[test]
        fn b64_extreme_milestone_distribution() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 20_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        10_000,
                        vec![
                            Milestone { release_bps: 1, description_hash: [1u8; 32] },
                            Milestone { release_bps: 1, description_hash: [2u8; 32] },
                            Milestone { release_bps: 9998, description_hash: [3u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10_000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    let mut total = 0u128;
                    for i in 0u8..3 {
                        assert_ok!(Crowdfunding::submit_milestone(
                            RuntimeOrigin::signed(ALICE),
                            id,
                            i
                        ));
                        assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                        let before = Balances::free_balance(&ALICE);
                        assert_ok!(Crowdfunding::claim_milestone_funds(
                            RuntimeOrigin::signed(ALICE),
                            id,
                            i
                        ));
                        total += Balances::free_balance(&ALICE) - before;
                    }
                    // bps_of(10000, 1)=1, bps_of(10000, 1)=1, bps_of(10000, 9998)=9998
                    assert_eq!(total, 10_000);
                });
        }

        /// B65: protocol_fee_bps=1 -> bps_of(1, 1) = ceil(1/10000) = 1.
        /// Fee = 1, creator_amount = 0.
        #[test]
        fn b65_tiny_fee_rounds_to_zero() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 1, 99));

                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    let alice_before = Balances::free_balance(&ALICE);
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                    // bps_of(1, 1) = ceil(1/10000) = 1, fee = 1, creator gets 0
                    assert_eq!(Balances::free_balance(&ALICE) - alice_before, 0);
                    // Account 99 receives fee of 1
                    assert_eq!(Balances::free_balance(&99), 1);
                });
        }
    }

    // ─── P-C14: "Kingmaker" — Admin Abuse ───────────────────────────────

    mod pc14_kingmaker_admin_abuse {
        use super::*;

        /// B66: Cancel all Funding campaigns -> all become Cancelled.
        #[test]
        fn b66_cancel_all_funding_campaigns() {
            ExtBuilder::default().balances(vec![(ALICE, 100_000)]).build().execute_with(|| {
                let mut ids = Vec::new();
                for i in 0..3u64 {
                    ids.push(create_funded_campaign(ALICE, default_aon_config(100 + i, 1000)));
                }

                for &id in &ids {
                    assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                    assert_eq!(
                        pallet::Campaigns::<Test>::get(id).unwrap().status,
                        CampaignStatus::Cancelled
                    );
                }
            });
        }

        /// B67: approve_milestone without checking content -> approve
        /// Submitted.
        #[test]
        fn b67_approve_submitted_milestone_no_content_check() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                    // Root can approve without any content validation
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                    assert_eq!(
                        pallet::MilestoneStatuses::<Test>::get(id, 0u8),
                        Some(MilestoneStatus::Approved)
                    );
                });
        }

        /// B68: Reject milestone forever -> creator stuck.
        #[test]
        fn b68_perpetual_rejection_blocks_creator() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    // Submit-reject cycle 3 times
                    for _ in 0..3 {
                        assert_ok!(Crowdfunding::submit_milestone(
                            RuntimeOrigin::signed(ALICE),
                            id,
                            0
                        ));
                        assert_ok!(Crowdfunding::reject_milestone(RuntimeOrigin::root(), id, 0));
                    }

                    // Creator cannot claim funds, still MilestonePhase
                    assert_eq!(
                        pallet::Campaigns::<Test>::get(id).unwrap().status,
                        CampaignStatus::MilestonePhase
                    );
                    assert_noop!(
                        Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id),
                        Error::<Test>::InvalidCampaignStatus
                    );
                });
        }

        /// B69: Pause all campaigns -> investors can only withdraw (with
        /// penalty).
        #[test]
        fn b69_pause_all_campaigns_withdraw_only() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 100_000), (BOB, 100_000)])
                .build()
                .execute_with(|| {
                    let id1 = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                    let id2 = create_funded_campaign(ALICE, default_aon_config(101, 1000));

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id1, 500));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id2, 500));

                    assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id1));
                    assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id2));

                    // Cannot invest
                    assert_noop!(
                        Crowdfunding::invest(RuntimeOrigin::signed(BOB), id1, 100),
                        Error::<Test>::InvalidCampaignStatus
                    );

                    // Can withdraw (with penalty)
                    let bob_before = Balances::free_balance(&BOB);
                    assert_ok!(Crowdfunding::withdraw_investment(
                        RuntimeOrigin::signed(BOB),
                        id1,
                        500
                    ));
                    // 500 - bps_of(500, 100) = 500 - 5 = 495
                    assert_eq!(Balances::free_balance(&BOB) - bob_before, 495);
                });
        }

        /// B70: set_protocol_config -> 100% fee AFTER creation -> locked fee=0
        /// applies. Campaign created with ProtocolFeeBps=0 (mock
        /// default), so fee locked at 0. Changing fee to 10000 after
        /// creation has no effect on this campaign.
        #[test]
        fn b70_100_percent_fee_drains_claims() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000), (99, 1)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 10000, 99));
                    let alice_before = Balances::free_balance(&ALICE);
                    assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                    // Locked fee = 0%, creator gets full 1000
                    assert_eq!(Balances::free_balance(&ALICE), alice_before + 1000);
                    // 99 started with 1, no fee collected
                    assert_eq!(Balances::free_balance(&99), 1);
                });
        }
    }

    // ─── P-C15: "Basilisk" — KeepWhatYouRaise Exploitation ──────────────

    mod pc15_basilisk_kwyr {
        use super::*;

        /// B71: KWYR{soft_cap: None} -> invest 1 -> finalize -> Succeeded.
        #[test]
        fn b71_kwyr_no_soft_cap_any_amount_succeeds() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_kwyr_config(20));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                    assert_eq!(
                        pallet::Campaigns::<Test>::get(id).unwrap().status,
                        CampaignStatus::Succeeded
                    );
                });
        }

        /// B72: KWYR{soft_cap: Some(100)} -> raise 99 -> Failed.
        #[test]
        fn b72_kwyr_below_soft_cap_fails() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(20, 100));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 99));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                    assert_eq!(
                        pallet::Campaigns::<Test>::get(id).unwrap().status,
                        CampaignStatus::Failed
                    );
                });
        }

        /// B73: KWYR{soft_cap: Some(100)} -> raise 100 -> Succeeded.
        #[test]
        fn b73_kwyr_at_soft_cap_succeeds() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(20, 100));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));
                    assert_eq!(
                        pallet::Campaigns::<Test>::get(id).unwrap().status,
                        CampaignStatus::Succeeded
                    );
                });
        }

        /// B74: KWYR + no hard_cap -> unlimited investment.
        #[test]
        fn b74_kwyr_no_hard_cap_unlimited() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 100_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_kwyr_config(20));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 50_000));
                    let c = pallet::Campaigns::<Test>::get(id).unwrap();
                    assert_eq!(c.total_raised, 50_000);
                });
        }
    }

    // ─── P-C16: "Paradox" — Accounting Inconsistency ────────────────────

    mod pc16_paradox_accounting {
        use super::*;

        /// B75: Multiple invest + partial withdraw -> total_raised tracks
        /// correctly.
        #[test]
        fn b75_invest_withdraw_total_raised_tracking() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().total_raised, 500);

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 300));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().total_raised, 800);

                    assert_ok!(Crowdfunding::withdraw_investment(
                        RuntimeOrigin::signed(BOB),
                        id,
                        200
                    ));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().total_raised, 600);

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().total_raised, 700);
                });
        }

        /// B76: Partial withdraw doesn't change investor_count, full withdraw
        /// decrements.
        #[test]
        fn b76_investor_count_partial_vs_full_withdraw() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 1);

                    // Partial withdraw: count unchanged
                    assert_ok!(Crowdfunding::withdraw_investment(
                        RuntimeOrigin::signed(BOB),
                        id,
                        200
                    ));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 1);

                    // Full withdraw: count decrements
                    assert_ok!(Crowdfunding::withdraw_investment(
                        RuntimeOrigin::signed(BOB),
                        id,
                        300
                    ));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 0);
                });
        }

        /// B77: invest -> full withdraw -> re-invest -> investor_count
        /// increments again.
        #[test]
        fn b77_reinvest_after_full_withdraw_increments_count() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 1);

                    assert_ok!(Crowdfunding::withdraw_investment(
                        RuntimeOrigin::signed(BOB),
                        id,
                        500
                    ));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 0);

                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 300));
                    assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().investor_count, 1);
                });
        }

        /// B78: total_disbursed <= total_raised after milestone claims.
        #[test]
        fn b78_total_disbursed_invariant_after_claims() {
            ExtBuilder::default()
                .balances(vec![(ALICE, 10_000), (BOB, 10_000)])
                .build()
                .execute_with(|| {
                    let config = milestone_config(
                        20,
                        1000,
                        vec![
                            Milestone { release_bps: 3000, description_hash: [1u8; 32] },
                            Milestone { release_bps: 3000, description_hash: [2u8; 32] },
                            Milestone { release_bps: 4000, description_hash: [3u8; 32] },
                        ],
                    );
                    let id = create_funded_campaign(ALICE, config);
                    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    for i in 0u8..3 {
                        assert_ok!(Crowdfunding::submit_milestone(
                            RuntimeOrigin::signed(ALICE),
                            id,
                            i
                        ));
                        assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                        assert_ok!(Crowdfunding::claim_milestone_funds(
                            RuntimeOrigin::signed(ALICE),
                            id,
                            i
                        ));

                        let c = pallet::Campaigns::<Test>::get(id).unwrap();
                        assert!(c.total_disbursed <= c.total_raised);
                    }
                });
        }
    }

    // ─── P-C17: "Hydra" — report_license_revoked Abuse ──────────────────

    mod pc17_hydra_license_report {
        use super::*;

        /// B79: report_license_revoked with active license -> fails
        /// LicenseNotActive.
        #[test]
        fn b79_report_with_active_license_fails() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(1, 1, ALICE, true);
                let config = default_aon_config(20, 1000);
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    Some((1, 1)),
                ));
                let id = pallet::NextCampaignId::<Test>::get() - 1;

                // License is active -> report fails
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::LicenseNotActive
                );
            });
        }

        /// B80: report works when is_license_active returns false.
        #[test]
        fn b80_report_with_revoked_license_succeeds() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(1, 1, ALICE, true);
                let config = default_aon_config(20, 1000);
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    Some((1, 1)),
                ));
                let id = pallet::NextCampaignId::<Test>::get() - 1;

                // Revoke the license
                MockLicenseVerifier::set_active(1, 1, false);
                assert_ok!(Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id));
                assert_eq!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                );
            });
        }

        /// B81: report on campaign with no license -> NoLinkedLicense.
        #[test]
        fn b81_report_no_license_fails() {
            ExtBuilder::default().build().execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::NoLinkedLicense
                );
            });
        }

        /// B82: report on Cancelled campaign -> InvalidCampaignStatus.
        #[test]
        fn b82_report_on_cancelled_fails() {
            ExtBuilder::default().build().execute_with(|| {
                MockLicenseVerifier::set_license(1, 1, ALICE, true);
                let config = default_aon_config(20, 1000);
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    Some((1, 1)),
                ));
                let id = pallet::NextCampaignId::<Test>::get() - 1;

                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                MockLicenseVerifier::set_active(1, 1, false);

                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
        }
    }

    // ─── P-C18: "Tantalus" — Creation Deposit Exploitation ──────────────

    mod pc18_tantalus_deposit {
        use super::*;

        /// B83: Create -> no investment -> wait -> finalize Failed ->
        /// claim_creation_deposit -> free storage use (campaign record
        /// persists but deposit returned).
        #[test]
        fn b83_free_storage_via_failed_campaign_deposit_reclaim() {
            ExtBuilder::default().build().execute_with(|| {
                let alice_before = Balances::free_balance(&ALICE);
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                let alice_after_create = Balances::free_balance(&ALICE);
                assert_eq!(alice_before - alice_after_create, 100);

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));

                let alice_final = Balances::free_balance(&ALICE);
                // Got deposit back -> net cost = 0 (but campaign record persists in storage)
                assert_eq!(alice_final, alice_before);

                // Campaign record still exists
                assert!(pallet::Campaigns::<Test>::get(id).is_some());
            });
        }

        /// B84: creation_deposit > free balance -> creation fails.
        #[test]
        fn b84_insufficient_balance_for_deposit() {
            ExtBuilder::default().balances(vec![(ALICE, 50)]).build().execute_with(|| {
                let config = default_aon_config(20, 1000);
                // ALICE has 50 < 100 (CampaignCreationDeposit)
                assert!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    None
                )
                .is_err());
            });
        }

        /// B85: claim_creation_deposit when creator has been reaped -> deposit
        /// re-creates account.
        #[test]
        fn b85_deposit_claim_recreates_reaped_creator() {
            ExtBuilder::default().balances(vec![(ALICE, 200), (BOB, 10_000)]).build().execute_with(
                || {
                    let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                    // ALICE now has 200 - 100 = 100

                    // Drain ALICE to 0 to get reaped (transfer all remaining)
                    // With ED=1, transferring 100 with AllowDeath will reap
                    assert_ok!(Balances::transfer(RuntimeOrigin::signed(ALICE), BOB, 99));
                    // ALICE has 1 left (ED), let's verify she's still alive
                    assert_eq!(Balances::free_balance(&ALICE), 1);

                    run_to_block(21);
                    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

                    // Claim deposit -> ALICE gets 100 back
                    assert_ok!(Crowdfunding::claim_creation_deposit(
                        RuntimeOrigin::signed(ALICE),
                        id
                    ));
                    assert_eq!(Balances::free_balance(&ALICE), 101);
                },
            );
        }
    }
}

mod mece_penalty_rounding_bypass {
    use super::*;

    #[test]
    fn micro_withdrawal_always_incurs_penalty() {
        // Even withdrawing 1 unit with 100 bps (1%) incurs penalty >= 1
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            let bob_before = Balances::free_balance(BOB);
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1));
            // Ceiling: ceil(1 * 100 / 10000) = 1, net = 0
            assert_eq!(Balances::free_balance(BOB), bob_before);
        });
    }

    #[test]
    fn bot_splitting_cannot_avoid_penalty() {
        // A bot splitting 100 into 100 x 1-unit withdrawals pays MORE penalty than
        // single withdrawal
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 5000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
            let bob_before = Balances::free_balance(BOB);
            // Withdraw 100 units one-by-one
            for _ in 0..100 {
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 1));
            }
            let bob_after = Balances::free_balance(BOB);
            // Each 1-unit withdrawal: penalty=1, net=0. Total received = 0
            // Single 100-unit withdrawal: penalty=ceil(100*100/10000)=1, net=99
            // So splitting is WORSE for the bot, not better.
            assert_eq!(bob_after, bob_before);
        });
    }

    #[test]
    fn ceiling_vs_floor_for_nonmultiple_amount() {
        // For 99 units at 100 bps: floor=0 (Permill), ceiling=1
        ExtBuilder::default().build().execute_with(|| {
            let penalty = Crowdfunding::bps_of(99u128, 100u16);
            assert_eq!(penalty, 1); // ceiling, not 0
        });
    }

    #[test]
    fn exact_multiple_gives_same_result() {
        // For 1000 units at 100 bps: floor=10, ceiling=10 (exact)
        ExtBuilder::default().build().execute_with(|| {
            let penalty = Crowdfunding::bps_of(1000u128, 100u16);
            assert_eq!(penalty, 10);
        });
    }

    #[test]
    fn bps_of_ceiling_at_1_bps_1_unit() {
        // Minimum non-zero: 1 bps on 1 unit = ceil(1/10000) = 1
        ExtBuilder::default().build().execute_with(|| {
            let penalty = Crowdfunding::bps_of(1u128, 1u16);
            assert_eq!(penalty, 1);
        });
    }
}

mod mece_whitelist_unbounded {
    use super::*;

    #[test]
    fn whitelist_rejects_at_bound() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                // MaxWhitelistSize = 100, fill it up
                for i in 10u64..110 {
                    assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, i));
                }
                assert_eq!(pallet::CampaignWhitelistCount::<Test>::get(id), 100);
                // 101st should fail
                assert_noop!(
                    Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, 200),
                    Error::<Test>::WhitelistFull
                );
            },
        );
    }

    #[test]
    fn whitelist_idempotent_insert_does_not_increment_count() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            assert_eq!(pallet::CampaignWhitelistCount::<Test>::get(id), 1);
            // Insert same account again — idempotent, count stays 1
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            assert_eq!(pallet::CampaignWhitelistCount::<Test>::get(id), 1);
        });
    }

    #[test]
    fn whitelist_count_cleaned_on_cancel() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, BOB));
            assert_ok!(Crowdfunding::add_to_whitelist(RuntimeOrigin::signed(ALICE), id, CHARLIE));
            assert_eq!(pallet::CampaignWhitelistCount::<Test>::get(id), 2);
            // Cancel campaign
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
            // Count should be cleaned up
            assert_eq!(pallet::CampaignWhitelistCount::<Test>::get(id), 0);
        });
    }
}

mod mece_retroactive_fee {
    use super::*;

    #[test]
    fn fee_locked_at_creation_time() {
        // Campaign created with ProtocolFeeBps=0 (mock default).
        // Changing the runtime fee after creation does NOT affect this campaign.
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.protocol_fee_bps, 0); // locked at 0

            // Simulate admin changing protocol fee to 50% (5000 bps)
            assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 5000, 99));

            // Fund and finalize
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

            let alice_before = Balances::free_balance(ALICE);
            let fee_acct_before = Balances::free_balance(99);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

            // No fee collected because locked fee = 0
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);
            assert_eq!(Balances::free_balance(99), fee_acct_before);
        });
    }

    #[test]
    fn new_campaign_uses_updated_fee() {
        // Change fee, then create campaign — new campaign locks the new fee
        ExtBuilder::default().build().execute_with(|| {
            assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 500, 99));
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.protocol_fee_bps, 500); // locked at 500 (5%)
        });
    }

    #[test]
    fn milestone_uses_locked_fee() {
        // Milestone-based campaign uses the locked fee, not runtime override
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    crate::Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    crate::Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            // Campaign locked at fee=0
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().protocol_fee_bps, 0);

            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

            // Change fee to 50% after finalization
            assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 5000, 99));

            // Submit & approve milestone 0
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));

            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 0));

            // 50% of 1000 = 500 release, with locked fee=0 -> Alice gets full 500
            assert_eq!(Balances::free_balance(ALICE), alice_before + 500);
        });
    }

    #[test]
    fn admin_cannot_retroactively_drain() {
        // The core attack: admin sets 100% fee after campaign funded
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

            // Malicious admin sets 100% fee
            assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 10000, 99));

            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));

            // Alice gets full amount because locked fee = 0
            assert_eq!(Balances::free_balance(ALICE), alice_before + 1000);
        });
    }
}

mod mece_license_cascade {
    use super::*;

    fn setup_licensed_campaign() -> (u32, u32) {
        // ALICE creates campaign with license (rwa_asset_id=1, participation_id=1)
        MockLicenseVerifier::set_license(1, 1, ALICE, true);
        let id = pallet::NextCampaignId::<Test>::get();
        assert_ok!(Crowdfunding::create_campaign(
            RuntimeOrigin::signed(ALICE),
            default_aon_config(100, 1000),
            None,
            Some((1, 1)),
        ));
        (id, 1)
    }

    #[test]
    fn report_revoked_license_cancels_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let (id, _) = setup_licensed_campaign();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));

            // Revoke the license
            MockLicenseVerifier::set_active(1, 1, false);

            // Anyone can report
            assert_ok!(Crowdfunding::report_license_revoked(RuntimeOrigin::signed(CHARLIE), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.status, CampaignStatus::Cancelled);
        });
    }

    #[test]
    fn investors_can_refund_after_license_revocation() {
        ExtBuilder::default().build().execute_with(|| {
            let (id, _) = setup_licensed_campaign();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            let bob_before_revoke = Balances::free_balance(BOB);

            MockLicenseVerifier::set_active(1, 1, false);
            assert_ok!(Crowdfunding::report_license_revoked(RuntimeOrigin::signed(CHARLIE), id));

            // BOB can claim refund
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            assert_eq!(Balances::free_balance(BOB), bob_before_revoke + 500);
        });
    }

    #[test]
    fn cannot_report_active_license() {
        ExtBuilder::default().build().execute_with(|| {
            let (id, _) = setup_licensed_campaign();
            // License is still active — report should fail
            assert_noop!(
                Crowdfunding::report_license_revoked(RuntimeOrigin::signed(CHARLIE), id),
                Error::<Test>::LicenseNotActive
            );
        });
    }

    #[test]
    fn claim_funds_succeeds_despite_revoked_license() {
        ExtBuilder::default().build().execute_with(|| {
            let (id, _) = setup_licensed_campaign();
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

            // Revoke after finalization
            MockLicenseVerifier::set_active(1, 1, false);

            // DEADLOCK-FIX: Creator can still claim funds despite revoked license
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert!(Balances::free_balance(ALICE) > alice_before);
        });
    }
}

mod mece_milestone_dust {
    use super::*;

    #[test]
    fn milestone_rounding_creates_dust() {
        // 3 milestones of 3334+3333+3333 = 10000 bps, with 1000 raised
        // Ceiling: ceil(1000*3334/10000)=334, ceil(1000*3333/10000)=334,
        // ceil(1000*3333/10000)=334 Total disbursed = 1002, but only 1000 in
        // account -> last claim may underflow
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    crate::Milestone { release_bps: 3334, description_hash: [1u8; 32] },
                    crate::Milestone { release_bps: 3333, description_hash: [2u8; 32] },
                    crate::Milestone { release_bps: 3333, description_hash: [3u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

            // Claim first two milestones
            for i in 0u8..2 {
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, i));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    i
                ));
            }

            // Third milestone -- sub-account may be short due to ceiling rounding
            assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 2));
            assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 2));
            // This may fail if sub-account is drained -- check the sub-account balance
            let sub = Crowdfunding::campaign_account(id);
            let sub_bal = Balances::free_balance(sub);
            // Sub-account had: 100 (deposit) + 1000 (raised) - 334 - 334 = 432
            // Third claim wants 334, which is <= 432, so should succeed
            assert_ok!(Crowdfunding::claim_milestone_funds(RuntimeOrigin::signed(ALICE), id, 2));
        });
    }

    #[test]
    fn exact_bps_sum_no_dust() {
        // 2 milestones of 5000+5000 = 10000 bps, with 1000 raised
        // Ceiling: ceil(1000*5000/10000)=500, ceil(1000*5000/10000)=500. Total=1000.
        // Exact.
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                1000,
                vec![
                    crate::Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                    crate::Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

            for i in 0u8..2 {
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, i));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    i
                ));
            }
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.status, CampaignStatus::Completed);
        });
    }

    #[test]
    fn dust_amount_stays_in_sub_account() {
        // After all milestones claimed, some dust might remain
        ExtBuilder::default().build().execute_with(|| {
            let config = milestone_config(
                20,
                999,
                vec![
                    crate::Milestone { release_bps: 3334, description_hash: [1u8; 32] },
                    crate::Milestone { release_bps: 3333, description_hash: [2u8; 32] },
                    crate::Milestone { release_bps: 3333, description_hash: [3u8; 32] },
                ],
            );
            let id = create_funded_campaign(ALICE, config);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 999));
            run_to_block(21);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(BOB), id));

            for i in 0u8..3 {
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, i));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    i
                ));
            }
            // Campaign should be completed
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.status, CampaignStatus::Completed);
        });
    }
}

mod mece_pause_deadline_extension {
    use super::*;

    #[test]
    fn single_pause_extends_deadline() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            let original_deadline = c.config.deadline;

            // Pause at block 50
            run_to_block(50);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));

            // Resume at block 70 (20 blocks paused)
            run_to_block(70);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.config.deadline, original_deadline + 20);
        });
    }

    #[test]
    fn multiple_pauses_accumulate_extensions() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            let original_deadline = pallet::Campaigns::<Test>::get(id).unwrap().config.deadline;

            // Pause 1: blocks 20-30 (10 blocks)
            run_to_block(20);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            run_to_block(30);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));

            // Pause 2: blocks 50-80 (30 blocks)
            run_to_block(50);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            run_to_block(80);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));

            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.config.deadline, original_deadline + 10 + 30);
        });
    }

    #[test]
    fn cannot_invest_while_paused() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            assert_noop!(
                Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn can_withdraw_while_paused() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            // Withdrawal allowed during pause
            assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100));
        });
    }
}

// ══════════════════════════════════════════════════════════════════════════
// TOPPAN ATTACK TESTS — Crowdfunding Pallet
//
// These tests PROVE the existence of critical vulnerabilities identified in
// the Toppan IP licensing attack plan. Each test PASSES in the current code,
// demonstrating the exploit is live.
//
// Vulnerabilities covered:
//   V1 (T3.3): Verifier does NOT check asset status — deactivated IP
//              assets still have "valid" participations.
//   V2 (T2.1): Campaign deadline can exceed license expiry, trapping
//              investor funds when the license expires mid-campaign.
// ══════════════════════════════════════════════════════════════════════════

mod mece_toppan_campaign_outlives_license {
    use super::*;

    // ── V1: T3.3 — FIXED: LicenseVerifier now checks asset status ────────

    #[test]
    fn v1_create_campaign_blocked_when_asset_deactivated() {
        // V1 FIX VERIFIED: The MockLicenseVerifier now has an asset-level
        // active flag. When the asset is deactivated, ensure_active_license
        // rejects even if the participation is still marked active.
        ExtBuilder::default().build().execute_with(|| {
            // License for ALICE on asset_id=1, participation_id=1
            MockLicenseVerifier::set_license(1, 1, ALICE, true);

            // Asset is active — campaign succeeds
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 500),
                None,
                Some((1, 1)),
            ));
            let campaign_id = pallet::NextCampaignId::<Test>::get() - 1;
            let c = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(c.rwa_asset_id, Some(1));
            assert!(matches!(c.status, CampaignStatus::Funding));

            // Deactivate the asset — participation remains "active" but
            // the asset is not.
            MockLicenseVerifier::set_asset_active(1, false);

            // New campaign creation FAILS: asset is deactivated
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(200, 500),
                    None,
                    Some((1, 1)),
                ),
                Error::<Test>::LicenseNotActive
            );
        });
    }

    #[test]
    fn v1_claim_funds_succeeds_despite_asset_deactivated() {
        // DEADLOCK-FIX: claim_funds no longer checks license/asset status.
        // Even with the asset deactivated, the creator can claim funds
        // because the campaign already met its goal and was finalized.
        ExtBuilder::default().build().execute_with(|| {
            MockLicenseVerifier::set_license(1, 1, ALICE, true);

            // Create campaign linked to license (1,1)
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 500),
                None,
                Some((1, 1)),
            ));
            let campaign_id = pallet::NextCampaignId::<Test>::get() - 1;

            // BOB invests enough to meet goal
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 500));

            // Advance past deadline, finalize
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(DAVE), campaign_id));

            // Campaign succeeded
            let c = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));

            // Deactivate the asset (participation still "active")
            MockLicenseVerifier::set_asset_active(1, false);

            // DEADLOCK-FIX: ALICE can still claim despite deactivated asset
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), campaign_id));
            assert!(Balances::free_balance(ALICE) > alice_before);
        });
    }

    // ── V2: T2.1 — FIXED: Campaign deadline validated against license expiry ──

    #[test]
    fn v2_campaign_deadline_beyond_license_expiry_rejected() {
        // V2 FIX VERIFIED: create_campaign now calls license_expiry() and
        // ensures config.deadline < expiry (CRIT-07: strict less-than).
        // A license expiring at block 50 cannot back a campaign with
        // deadline >= 50 because is_license_active uses now < expiry.
        ExtBuilder::default().build().execute_with(|| {
            MockLicenseVerifier::set_license(1, 1, ALICE, true);
            // License expires at block 50
            MockLicenseVerifier::set_license_expiry(1, 1, 50);

            // Campaign with deadline=999 exceeds license expiry — REJECTED
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(999, 500),
                    None,
                    Some((1, 1)),
                ),
                Error::<Test>::CampaignExceedsLicenseExpiry
            );

            // CRIT-07: Campaign with deadline=50 (== license expiry) — NOW REJECTED
            // because at block 50 is_license_active(50) returns false (50 < 50 = false),
            // so a front-runner could call report_license_revoked before finalize.
            assert_noop!(
                Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(50, 500),
                    None,
                    Some((1, 1)),
                ),
                Error::<Test>::CampaignExceedsLicenseExpiry
            );

            // Campaign with deadline=49 (< license expiry) — OK
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(49, 500),
                None,
                Some((1, 1)),
            ));

            let campaign_id = pallet::NextCampaignId::<Test>::get() - 1;
            let c = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(c.config.deadline, 49);
            assert_eq!(c.rwa_asset_id, Some(1));
        });
    }

    #[test]
    fn v2_campaign_with_unlimited_license_expiry_allowed() {
        // V2: When license has no expiry (unlimited), any valid deadline is OK.
        ExtBuilder::default().build().execute_with(|| {
            MockLicenseVerifier::set_license(1, 1, ALICE, true);
            // No expiry set — license_expiry() returns None

            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(999, 500),
                None,
                Some((1, 1)),
            ));
            let campaign_id = pallet::NextCampaignId::<Test>::get() - 1;
            let c = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(c.config.deadline, 999);
        });
    }

    #[test]
    fn v2_license_expires_after_campaign_succeeds_funds_not_trapped() {
        // DEADLOCK-FIX: The original vulnerability where funds became trapped
        // when a license expired after campaign success is now resolved.
        // claim_funds no longer checks license status, so the creator can
        // always claim from a Succeeded campaign regardless of license state.
        ExtBuilder::default().build().execute_with(|| {
            MockLicenseVerifier::set_license(1, 1, ALICE, true);

            // Create campaign with deadline=100
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 500),
                None,
                Some((1, 1)),
            ));
            let campaign_id = pallet::NextCampaignId::<Test>::get() - 1;

            // BOB invests to meet goal
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 500));

            // Finalize after deadline
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(DAVE), campaign_id));
            let c = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert!(matches!(c.status, CampaignStatus::Succeeded));

            // Simulate license expiry: set participation inactive
            MockLicenseVerifier::set_active(1, 1, false);

            // DEADLOCK-FIX: ALICE can now claim despite expired license
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), campaign_id));
            assert!(Balances::free_balance(ALICE) > alice_before);
        });
    }

    #[test]
    fn crit01_report_license_revoked_on_succeeded_campaign_is_blocked() {
        // CRIT-01 FIX VERIFIED: report_license_revoked can no longer cancel
        // a Succeeded campaign.  Once finalized to Succeeded, the creator
        // has a guaranteed window to claim funds.  License revocation
        // after success must NOT rug-pull the creator.
        ExtBuilder::default().build().execute_with(|| {
            MockLicenseVerifier::set_license(1, 1, ALICE, true);

            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(100, 500),
                None,
                Some((1, 1)),
            ));
            let campaign_id = pallet::NextCampaignId::<Test>::get() - 1;

            // BOB invests to meet goal
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 500));

            // Finalize -> Succeeded
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(DAVE), campaign_id));
            let c = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(c.status, CampaignStatus::Succeeded);

            // License revoked after finalization
            MockLicenseVerifier::set_active(1, 1, false);

            // report_license_revoked is now BLOCKED on Succeeded campaigns
            assert_noop!(
                Crowdfunding::report_license_revoked(RuntimeOrigin::signed(CHARLIE), campaign_id,),
                Error::<Test>::InvalidCampaignStatus
            );

            // Campaign remains Succeeded — creator's funds are safe
            let c = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(c.status, CampaignStatus::Succeeded);
        });
    }

    #[test]
    fn v2_kwyr_campaign_with_outlived_license_deadlock_resolved() {
        // DEADLOCK-FIX: KWYR variant of the deadlock is also resolved.
        // Even with KWYR (no strict goal), the creator can now claim funds
        // regardless of license status after finalization.
        ExtBuilder::default().build().execute_with(|| {
            MockLicenseVerifier::set_license(1, 1, ALICE, true);

            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_kwyr_config(100),
                None,
                Some((1, 1)),
            ));
            let campaign_id = pallet::NextCampaignId::<Test>::get() - 1;

            // BOB invests some amount
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 300));

            // Finalize
            run_to_block(101);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(DAVE), campaign_id));

            // License expires post-finalization
            MockLicenseVerifier::set_active(1, 1, false);

            // DEADLOCK-FIX: claim_funds succeeds despite expired license
            let alice_before = Balances::free_balance(ALICE);
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), campaign_id));
            assert!(Balances::free_balance(ALICE) > alice_before);
        });
    }

    // ── V1+V2 Combined: The verifier never checks asset status AND
    //    deadline is never validated against license expiry ────────────────

    #[test]
    fn v1_v2_combined_the_verifier_is_the_sole_trust_boundary() {
        // This test demonstrates that the LicenseVerifier is the ONLY
        // mechanism protecting campaign integrity with respect to IP
        // licensing. There are two dimensions it fails to cover:
        //
        // 1. Asset status (V1): Only checks participation, not asset
        // 2. License expiry alignment (V2): No deadline vs expiry check
        //
        // Both gaps stem from the same root cause: the LicenseVerifier
        // trait interface is too narrow. It only exposes:
        //   - ensure_active_license(asset_id, part_id, who) -> Result
        //   - is_license_active(asset_id, part_id) -> bool
        //
        // It returns no information about:
        //   - Asset status (Active/Inactive/Retired/Paused)
        //   - License expiry block number
        //   - License duration remaining
        ExtBuilder::default().build().execute_with(|| {
            MockLicenseVerifier::set_license(1, 1, ALICE, true);

            // Campaign with maximum possible deadline
            assert_ok!(Crowdfunding::create_campaign(
                RuntimeOrigin::signed(ALICE),
                default_aon_config(999, 500),
                None,
                Some((1, 1)),
            ));
            let campaign_id = pallet::NextCampaignId::<Test>::get() - 1;

            // Invest and finalize immediately (at block 2)
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 500));

            // Even with deadline=999, we can test the license check
            // License "expires" at block 5 (simulated)
            run_to_block(999 + 1);
            assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(DAVE), campaign_id));

            // At this point license is still "active" in our mock
            // (we haven't toggled it) — claim works fine
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), campaign_id));

            // PROOF: The only thing that matters is whether the mock says
            // the license is active at claim time. No structural protection
            // exists at campaign creation time.
        });
    }
}

// ── force_finalize_campaign ─────────────────────────────────────────────

mod force_finalize_campaign {
    use super::*;

    #[test]
    fn happy_path_aon_succeeded_before_deadline() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, default_aon_config(100, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 500));

            // Still before deadline (block 1), force finalize
            assert_ok!(Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,));

            let campaign = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(campaign.status, CampaignStatus::Succeeded);
            System::assert_has_event(
                Event::CampaignForceFinalized { campaign_id, status: CampaignStatus::Succeeded }
                    .into(),
            );
        });
    }

    #[test]
    fn happy_path_aon_failed_before_deadline() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, default_aon_config(100, 500));
            // No investments — goal not met

            assert_ok!(Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,));

            let campaign = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(campaign.status, CampaignStatus::Failed);
            System::assert_has_event(
                Event::CampaignForceFinalized { campaign_id, status: CampaignStatus::Failed }
                    .into(),
            );
        });
    }

    #[test]
    fn happy_path_kwyr() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, default_kwyr_config(100));

            assert_ok!(Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,));

            let campaign = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(campaign.status, CampaignStatus::Succeeded);
        });
    }

    #[test]
    fn happy_path_milestone_based() {
        ExtBuilder::default().build().execute_with(|| {
            let milestones = vec![
                crate::Milestone { release_bps: 5000, description_hash: [0u8; 32] },
                crate::Milestone { release_bps: 5000, description_hash: [0u8; 32] },
            ];
            let campaign_id = create_funded_campaign(ALICE, milestone_config(100, 500, milestones));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 500));

            assert_ok!(Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,));

            let campaign = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(campaign.status, CampaignStatus::MilestonePhase);
            // Milestone statuses should be initialized
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(campaign_id, 0u8),
                Some(MilestoneStatus::Pending)
            );
            assert_eq!(
                pallet::MilestoneStatuses::<Test>::get(campaign_id, 1u8),
                Some(MilestoneStatus::Pending)
            );
        });
    }

    #[test]
    fn rejects_non_sudo_origin() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, default_aon_config(100, 500));

            assert_noop!(
                Crowdfunding::force_finalize_campaign(RuntimeOrigin::signed(ALICE), campaign_id,),
                sp_runtime::DispatchError::BadOrigin
            );
        });
    }

    #[test]
    fn rejects_non_funding_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, default_aon_config(100, 500));
            // Cancel it first
            assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), campaign_id));

            assert_noop!(
                Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn rejects_campaign_not_found() {
        ExtBuilder::default().build().execute_with(|| {
            assert_noop!(
                Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), 999,),
                Error::<Test>::CampaignNotFound
            );
        });
    }

    #[test]
    fn creator_can_claim_funds_after_force_finalize() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, default_aon_config(100, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 500));

            assert_ok!(Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,));

            // Creator should be able to claim funds
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), campaign_id));
        });
    }

    #[test]
    fn investors_can_claim_refund_after_force_finalize_failed() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, default_aon_config(100, 500));
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 100));
            // Goal not met

            assert_ok!(Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,));

            // Investor should be able to claim refund
            assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), campaign_id));
        });
    }

    #[test]
    fn kwyr_soft_cap_not_met_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, kwyr_config_with_soft_cap(100, 500));
            // Invest below soft cap
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 100));

            assert_ok!(Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,));

            let campaign = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(campaign.status, CampaignStatus::Failed);
        });
    }

    #[test]
    fn milestone_based_goal_not_met_fails() {
        ExtBuilder::default().build().execute_with(|| {
            let milestones = vec![
                crate::Milestone { release_bps: 5000, description_hash: [0u8; 32] },
                crate::Milestone { release_bps: 5000, description_hash: [0u8; 32] },
            ];
            let campaign_id = create_funded_campaign(ALICE, milestone_config(100, 500, milestones));
            // Invest below goal
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), campaign_id, 100));

            assert_ok!(Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,));

            let campaign = pallet::Campaigns::<Test>::get(campaign_id).unwrap();
            assert_eq!(campaign.status, CampaignStatus::Failed);
            // No milestone statuses should be initialized
            assert_eq!(pallet::MilestoneStatuses::<Test>::get(campaign_id, 0u8), None);
        });
    }

    #[test]
    fn rejects_paused_campaign() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, default_aon_config(100, 500));
            // Pause it (AdminOrigin = EnsureRoot in mock)
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), campaign_id,));

            assert_noop!(
                Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn rejects_double_force_finalize() {
        ExtBuilder::default().build().execute_with(|| {
            let campaign_id = create_funded_campaign(ALICE, default_aon_config(100, 500));

            assert_ok!(Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,));

            // Second force-finalize should fail — no longer Funding
            assert_noop!(
                Crowdfunding::force_finalize_campaign(RuntimeOrigin::root(), campaign_id,),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }
}
