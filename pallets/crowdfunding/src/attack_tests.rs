//! # Crowdfunding Pallet -- Attack / Penetration Tests
//!
//! Adversarial tests from `docs/crowdfunding-attack-plan.md` and
//! `docs/MASTER_ATTACK_PLAN_2026-03-15.md`.
//!
//! CRIT-01 and CRIT-02 fixes are already applied (Succeeded blocked from
//! both `report_license_revoked` and `cancel_campaign`). Tests verify the
//! fixes hold and probe remaining attack surfaces.

use frame_support::{assert_noop, assert_ok, traits::Currency};

use super::{mock::*, *};

const GRIEFER: u64 = 5;
const ATTACKER_FEE_RECIPIENT: u64 = 88;
const RWA_ASSET_ID: u32 = 1;
const PARTICIPATION_ID: u32 = 1;

fn setup_succeeded_licensed_campaign() -> u32 {
    MockLicenseVerifier::set_license(RWA_ASSET_ID, PARTICIPATION_ID, ALICE, true);
    MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, PARTICIPATION_ID, 200);
    let config = default_aon_config(20, 1000);
    let id = pallet::NextCampaignId::<Test>::get();
    assert_ok!(Crowdfunding::create_campaign(
        RuntimeOrigin::signed(ALICE),
        config,
        None,
        Some((RWA_ASSET_ID, PARTICIPATION_ID)),
    ));
    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
    run_to_block(21);
    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
    assert!(matches!(
        pallet::Campaigns::<Test>::get(id).unwrap().status,
        CampaignStatus::Succeeded
    ));
    id
}

fn setup_funding_licensed_campaign() -> u32 {
    MockLicenseVerifier::set_license(RWA_ASSET_ID, PARTICIPATION_ID, ALICE, true);
    MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, PARTICIPATION_ID, 200);
    let config = default_aon_config(50, 1000);
    let id = pallet::NextCampaignId::<Test>::get();
    assert_ok!(Crowdfunding::create_campaign(
        RuntimeOrigin::signed(ALICE),
        config,
        None,
        Some((RWA_ASSET_ID, PARTICIPATION_ID)),
    ));
    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
    id
}

fn setup_milestone_licensed_campaign(
    milestones: Vec<Milestone>,
    goal: u128,
    investment: u128,
) -> u32 {
    MockLicenseVerifier::set_license(RWA_ASSET_ID, PARTICIPATION_ID, ALICE, true);
    MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, PARTICIPATION_ID, 200);
    let config = milestone_config(20, goal, milestones);
    let id = pallet::NextCampaignId::<Test>::get();
    assert_ok!(Crowdfunding::create_campaign(
        RuntimeOrigin::signed(ALICE),
        config,
        None,
        Some((RWA_ASSET_ID, PARTICIPATION_ID)),
    ));
    assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, investment));
    run_to_block(21);
    assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
    assert!(matches!(
        pallet::Campaigns::<Test>::get(id).unwrap().status,
        CampaignStatus::MilestonePhase
    ));
    id
}

// CF-CAT1.1-C: cancel_campaign on Succeeded (CRIT-02 fix verified)
mod attack_cf_cat1_1_c {
    use super::*;

    #[test]
    fn attack_cf_cat1_1_c_force_origin_blocked_on_succeeded() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_succeeded_licensed_campaign();
                assert_noop!(
                    Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                    Error::<Test>::InvalidCampaignStatus
                );
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Succeeded
                ));
                let a = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE) - a, 1000);
            });
    }

    #[test]
    fn attack_cf_cat1_1_c_force_origin_allowed_on_funding() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let id = setup_funding_licensed_campaign();
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                ));
                let b = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB) - b, 500);
            },
        );
    }

    #[test]
    fn attack_cf_cat1_1_c_force_origin_blocked_on_completed() {
        ExtBuilder::default().build().execute_with(|| {
            let id = setup_succeeded_licensed_campaign();
            assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
            assert_noop!(
                Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                Error::<Test>::InvalidCampaignStatus
            );
        });
    }

    #[test]
    fn attack_cf_cat1_1_c_force_origin_allowed_on_milestone_phase() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let id = setup_milestone_licensed_campaign(
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                    1000,
                    1000,
                );
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                ));
            },
        );
    }
}

// CF-CAT2.1-C: report_license_revoked on Succeeded (CRIT-01 fix verified)
mod attack_cf_cat2_1_c {
    use super::*;

    #[test]
    fn attack_cf_cat2_1_c_report_blocked_on_succeeded() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_succeeded_licensed_campaign();
                MockLicenseVerifier::set_active(RWA_ASSET_ID, PARTICIPATION_ID, false);
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(GRIEFER), id),
                    Error::<Test>::InvalidCampaignStatus
                );
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Succeeded
                ));
            });
    }

    #[test]
    fn attack_cf_cat2_1_c_report_allowed_on_funding() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_funding_licensed_campaign();
                MockLicenseVerifier::set_active(RWA_ASSET_ID, PARTICIPATION_ID, false);
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(GRIEFER),
                    id
                ));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                ));
                let b = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB) - b, 500);
            });
    }

    #[test]
    fn attack_cf_cat2_1_c_report_allowed_on_milestone_phase() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_milestone_licensed_campaign(
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                    1000,
                    1000,
                );
                MockLicenseVerifier::set_active(RWA_ASSET_ID, PARTICIPATION_ID, false);
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(GRIEFER),
                    id
                ));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                ));
            });
    }

    #[test]
    fn attack_cf_cat2_1_c_deadlock_resolved_claim_succeeds_despite_revoked_license() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let id = setup_succeeded_licensed_campaign();
                MockLicenseVerifier::set_active(RWA_ASSET_ID, PARTICIPATION_ID, false);
                // DEADLOCK-FIX: claim_funds no longer checks license status.
                // The campaign already met its goal and was finalized to Succeeded,
                // so the creator has earned the right to claim.
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert!(Balances::free_balance(ALICE) > alice_before);
                // report_license_revoked and cancel still blocked on Succeeded (CRIT-01/02)
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
                assert_noop!(
                    Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            },
        );
    }

    #[test]
    fn attack_cf_cat2_1_c_active_license_blocks_report() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_funding_licensed_campaign();
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(GRIEFER), id),
                    Error::<Test>::LicenseNotActive
                );
            });
    }

    #[test]
    fn attack_cf_cat2_1_c_no_linked_license_blocks_report() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(20, 1000));
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(GRIEFER), id),
                    Error::<Test>::NoLinkedLicense
                );
            });
    }
}

// CF-CAT4.1-C: claim_creation_deposit reaps sub-account (asset-funded)
mod attack_cf_cat4_1_c {
    use frame_support::traits::tokens::fungibles;

    use super::*;

    /// **CF-CAT4.1-C -- CRIT-06 Fix Verified: KeepAlive blocks deposit claim
    /// when investors remain**
    ///
    /// The fix uses KeepAlive for asset-funded campaigns when investor_count >
    /// 0. Creator cannot reap the sub-account while investors still need
    /// refunds.
    #[test]
    fn attack_cf_cat4_1_c_crit06_fix_blocks_reap() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let asset_id: u32 = 42;
                assert_ok!(Assets::force_create(
                    RuntimeOrigin::root(),
                    asset_id.into(),
                    ALICE,
                    true,
                    1
                ));
                assert_ok!(Assets::mint(RuntimeOrigin::signed(ALICE), asset_id.into(), BOB, 5000));

                let mut config = default_aon_config(20, 1000);
                config.funding_currency = PaymentCurrency::Asset(asset_id);
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    None
                ));
                let sub = Crowdfunding::campaign_account(id);
                assert_eq!(Balances::free_balance(&sub), 100);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                assert_eq!(<Assets as fungibles::Inspect<u64>>::balance(asset_id, &sub), 500);

                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                // CRIT-06 FIX: claim_creation_deposit uses KeepAlive when investor_count > 0
                // This FAILS because the sub-account's entire native balance IS the deposit,
                // and KeepAlive prevents reducing it below ED.
                assert_noop!(
                    Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id),
                    pallet_balances::Error::<Test>::KeepAlive
                );

                // Investor refunds first
                let b = <Assets as fungibles::Inspect<u64>>::balance(asset_id, &BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(<Assets as fungibles::Inspect<u64>>::balance(asset_id, &BOB) - b, 500);

                // Now investor_count == 0, deposit claim succeeds with AllowDeath
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(&sub), 0);
            },
        );
    }

    #[test]
    fn attack_cf_cat4_1_c_safe_ordering() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let asset_id: u32 = 42;
                assert_ok!(Assets::force_create(
                    RuntimeOrigin::root(),
                    asset_id.into(),
                    ALICE,
                    true,
                    1
                ));
                assert_ok!(Assets::mint(RuntimeOrigin::signed(ALICE), asset_id.into(), BOB, 5000));

                let mut config = default_aon_config(20, 1000);
                config.funding_currency = PaymentCurrency::Asset(asset_id);
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    config,
                    None,
                    None
                ));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 500));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                let b = <Assets as fungibles::Inspect<u64>>::balance(asset_id, &BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(<Assets as fungibles::Inspect<u64>>::balance(asset_id, &BOB) - b, 500);
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            },
        );
    }
}

// CF-CAT1.3-M: Pause/resume bypasses MaxCampaignDuration
mod attack_cf_cat1_3_m {
    use super::*;

    #[test]
    fn attack_cf_cat1_3_m_extends_past_max_duration() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(11, 1000));
            run_to_block(5);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            run_to_block(505);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().config.deadline, 511);
            run_to_block(510);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            run_to_block(1510);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            let c = pallet::Campaigns::<Test>::get(id).unwrap();
            assert_eq!(c.config.deadline, 1511);
            assert!(c.config.deadline.saturating_sub(1) > 1000, "exceeds MaxCampaignDuration");
            run_to_block(1511);
            assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 100));
        });
    }

    #[test]
    fn attack_cf_cat1_3_m_single_pause_correct() {
        ExtBuilder::default().build().execute_with(|| {
            let id = create_funded_campaign(ALICE, default_aon_config(50, 1000));
            run_to_block(10);
            assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
            run_to_block(30);
            assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
            assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().config.deadline, 70);
        });
    }
}

// CF-CAT3.2-H: Ceiling rounding compounds twice in milestone claims
mod attack_cf_cat3_2_h {
    use super::*;

    #[test]
    fn attack_cf_cat3_2_h_fee_amplification() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (99, 10_000)])
            .build()
            .execute_with(|| {
                assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 100, 99));
                let config = milestone_config(
                    20,
                    10,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                let a = Balances::free_balance(ALICE);
                let f = Balances::free_balance(99u64);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                let fee = Balances::free_balance(99u64) - f;
                let creator = Balances::free_balance(ALICE) - a;
                assert_eq!(fee, 1);
                assert_eq!(creator, 4);
                assert_eq!((fee * 100) / (fee + creator), 20); // 20x amplification
            });
    }

    #[test]
    fn attack_cf_cat3_2_h_large_amount_normal() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (99, 10_000)])
            .build()
            .execute_with(|| {
                assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 100, 99));
                let config = milestone_config(
                    20,
                    10_000,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10_000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                let f = Balances::free_balance(99u64);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                assert_eq!(Balances::free_balance(99u64) - f, 50); // 1% of 5000
            });
    }
}

// CF-CAT4.3-H: Protocol fee recipient changed after campaign creation
mod attack_cf_cat4_3_h {
    use super::*;

    #[test]
    fn attack_cf_cat4_3_h_recipient_redirect() {
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (99, 10_000),
                (ATTACKER_FEE_RECIPIENT, 10_000),
            ])
            .build()
            .execute_with(|| {
                assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 500, 99));
                let config = milestone_config(
                    20,
                    10_000,
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10_000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));

                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                let r0 = Balances::free_balance(99u64);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                assert_eq!(Balances::free_balance(99u64) - r0, 250);

                assert_ok!(Crowdfunding::set_protocol_config(
                    RuntimeOrigin::root(),
                    500,
                    ATTACKER_FEE_RECIPIENT
                ));

                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 1));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 1));
                let atk = Balances::free_balance(ATTACKER_FEE_RECIPIENT);
                let orig = Balances::free_balance(99u64);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    1
                ));
                assert_eq!(Balances::free_balance(ATTACKER_FEE_RECIPIENT) - atk, 250);
                assert_eq!(Balances::free_balance(99u64) - orig, 0);
            });
    }

    #[test]
    fn attack_cf_cat4_3_h_bps_locked() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (99, 10_000)])
            .build()
            .execute_with(|| {
                assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 200, 99));
                let id = create_funded_campaign(ALICE, default_kwyr_config(20));
                assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().protocol_fee_bps, 200);
                assert_ok!(Crowdfunding::set_protocol_config(RuntimeOrigin::root(), 5000, 99));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 10_000));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                let f = Balances::free_balance(99u64);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(99u64) - f, 200);
            });
    }
}

// CROSS-4: Boundary block race (deadline == license expiry)
mod attack_cross_4 {
    use super::*;

    /// **CROSS-4 -- CRIT-07 Fix Verified: deadline == expiry rejected at
    /// creation**
    ///
    /// The V2 check now uses strict `<` (not `<=`). Creating a campaign
    /// with deadline == license expiry is rejected.
    #[test]
    fn attack_cross_4_deadline_equals_expiry_rejected() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let expiry = 50u64;
                MockLicenseVerifier::set_license(RWA_ASSET_ID, PARTICIPATION_ID, ALICE, true);
                MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, PARTICIPATION_ID, expiry);
                // CRIT-07 FIX: deadline == expiry is now rejected
                assert_noop!(
                    Crowdfunding::create_campaign(
                        RuntimeOrigin::signed(ALICE),
                        default_aon_config(expiry, 1000),
                        None,
                        Some((RWA_ASSET_ID, PARTICIPATION_ID)),
                    ),
                    Error::<Test>::CampaignExceedsLicenseExpiry
                );
            });
    }

    /// **CROSS-4 -- deadline == expiry - 1 is accepted and safe**
    #[test]
    fn attack_cross_4_deadline_one_before_expiry_accepted() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let expiry = 50u64;
                MockLicenseVerifier::set_license(RWA_ASSET_ID, PARTICIPATION_ID, ALICE, true);
                MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, PARTICIPATION_ID, expiry);
                let id = pallet::NextCampaignId::<Test>::get();
                // deadline = expiry - 1: accepted
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(expiry - 1, 1000),
                    None,
                    Some((RWA_ASSET_ID, PARTICIPATION_ID)),
                ));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                // At expiry block (= deadline + 1), finalize can execute before license expires
                run_to_block(expiry);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Succeeded
                ));
            });
    }

    #[test]
    fn attack_cross_4_deadline_before_expiry_safe() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let expiry = 50u64;
                MockLicenseVerifier::set_license(RWA_ASSET_ID, PARTICIPATION_ID, ALICE, true);
                MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, PARTICIPATION_ID, expiry);
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(expiry - 1, 1000),
                    None,
                    Some((RWA_ASSET_ID, PARTICIPATION_ID))
                ));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(expiry);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Succeeded
                ));
            },
        );
    }
}

// CROSS-5: Post-success license revocation via asset deactivation
mod attack_cross_5 {
    use super::*;

    #[test]
    fn attack_cross_5_blocked_on_succeeded() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_succeeded_licensed_campaign();
                MockLicenseVerifier::set_asset_active(RWA_ASSET_ID, false);
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(GRIEFER), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            });
    }

    #[test]
    fn attack_cross_5_cancels_funding() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_funding_licensed_campaign();
                MockLicenseVerifier::set_asset_active(RWA_ASSET_ID, false);
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(GRIEFER),
                    id
                ));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                ));
            });
    }

    #[test]
    fn attack_cross_5_partial_milestone() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_milestone_licensed_campaign(
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                    1000,
                    1000,
                );
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                MockLicenseVerifier::set_asset_active(RWA_ASSET_ID, false);
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(GRIEFER),
                    id
                ));
                let b = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB) - b, 500);
            });
    }
}

// CROSS-17: Pause-resume extends deadline past license expiry
mod attack_cross_17 {
    use super::*;

    /// **CROSS-17 -- CRIT-04 Fix Verified: resume rejects deadline past license
    /// expiry**
    ///
    /// After the fix, `resume_campaign` re-validates deadline < license expiry.
    /// A long pause that would push deadline past expiry is rejected.
    #[test]
    fn attack_cross_17_resume_rejects_past_expiry() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let expiry = 100u64;
                MockLicenseVerifier::set_license(RWA_ASSET_ID, PARTICIPATION_ID, ALICE, true);
                MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, PARTICIPATION_ID, expiry);
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(50, 1000),
                    None,
                    Some((RWA_ASSET_ID, PARTICIPATION_ID))
                ));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(10);
                assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
                // Resume after 60 blocks would push deadline to 110 > expiry (100)
                run_to_block(70);
                // CRIT-04 FIX: resume_campaign rejects the extension
                assert_noop!(
                    Crowdfunding::resume_campaign(RuntimeOrigin::root(), id),
                    Error::<Test>::CampaignExceedsLicenseExpiry
                );
                // Campaign stays Paused
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Paused
                ));
            });
    }

    /// **CROSS-17 -- Resume within expiry boundary accepted**
    #[test]
    fn attack_cross_17_resume_within_expiry_accepted() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let expiry = 100u64;
                MockLicenseVerifier::set_license(RWA_ASSET_ID, PARTICIPATION_ID, ALICE, true);
                MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, PARTICIPATION_ID, expiry);
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(50, 1000),
                    None,
                    Some((RWA_ASSET_ID, PARTICIPATION_ID))
                ));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(10);
                assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
                // Resume after 39 blocks: deadline = 50 + 39 = 89 < 100
                run_to_block(49);
                assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.config.deadline, 89);
                assert!(c.config.deadline < expiry);
            });
    }

    #[test]
    fn attack_cross_17_within_expiry_safe() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                MockLicenseVerifier::set_license(RWA_ASSET_ID, PARTICIPATION_ID, ALICE, true);
                MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, PARTICIPATION_ID, 200);
                let id = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(50, 1000),
                    None,
                    Some((RWA_ASSET_ID, PARTICIPATION_ID))
                ));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 1000));
                run_to_block(10);
                assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
                run_to_block(20);
                assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
                assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().config.deadline, 60);
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(GRIEFER), id),
                    Error::<Test>::LicenseNotActive
                );
            });
    }
}

// CROSS-2: Deposit double-dip after campaign success
mod attack_cross_2 {
    use super::*;

    #[test]
    fn attack_cross_2_full_double_dip() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let id = setup_succeeded_licensed_campaign();
                let a = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE) - a, 1000);
                let a = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE) - a, 100);
                MockLicenseVerifier::set_active(RWA_ASSET_ID, PARTICIPATION_ID, false);
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            },
        );
    }

    #[test]
    fn attack_cross_2_exit_before_claim_succeeds() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let id = setup_succeeded_licensed_campaign();
                MockLicenseVerifier::set_active(RWA_ASSET_ID, PARTICIPATION_ID, false);
                // DEADLOCK-FIX: claim_funds succeeds despite revoked license.
                // Deposit double-dip is accepted trade-off to prevent fund lockup.
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert!(Balances::free_balance(ALICE) > alice_before);
            },
        );
    }
}

// Supplementary attacks
mod attack_supplementary {
    use super::*;

    #[test]
    fn attack_cf_cat1_2_h_cancel_partial_milestone() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
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
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                let b = Balances::free_balance(BOB);
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
                assert_eq!(Balances::free_balance(BOB) - b, 500);
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
            },
        );
    }

    #[test]
    fn attack_cf_cat3_1_h_rounding_capped() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let config = milestone_config(
                    20,
                    101,
                    vec![
                        Milestone { release_bps: 3333, description_hash: [1u8; 32] },
                        Milestone { release_bps: 3333, description_hash: [2u8; 32] },
                        Milestone { release_bps: 3334, description_hash: [3u8; 32] },
                    ],
                );
                let id = create_funded_campaign(ALICE, config);
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 101));
                run_to_block(21);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                let mut amounts = vec![];
                for i in 0..3u8 {
                    assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, i));
                    assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, i));
                    let b = Balances::free_balance(ALICE);
                    assert_ok!(Crowdfunding::claim_milestone_funds(
                        RuntimeOrigin::signed(ALICE),
                        id,
                        i
                    ));
                    amounts.push(Balances::free_balance(ALICE) - b);
                }
                assert_eq!(amounts, vec![34, 34, 33]);
                let c = pallet::Campaigns::<Test>::get(id).unwrap();
                assert_eq!(c.total_disbursed, 101);
                assert!(matches!(c.status, CampaignStatus::Completed));
            },
        );
    }

    #[test]
    fn attack_cross_6_milestone_claim_succeeds_despite_revoked_license() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_milestone_licensed_campaign(
                    vec![
                        Milestone { release_bps: 5000, description_hash: [1u8; 32] },
                        Milestone { release_bps: 5000, description_hash: [2u8; 32] },
                    ],
                    1000,
                    1000,
                );
                assert_ok!(Crowdfunding::submit_milestone(RuntimeOrigin::signed(ALICE), id, 0));
                assert_ok!(Crowdfunding::approve_milestone(RuntimeOrigin::root(), id, 0));
                MockLicenseVerifier::set_active(RWA_ASSET_ID, PARTICIPATION_ID, false);
                // DEADLOCK-FIX: claim_milestone_funds succeeds despite revoked license.
                // The milestone was already approved by governance — blocking the claim
                // would strand funds.
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_milestone_funds(
                    RuntimeOrigin::signed(ALICE),
                    id,
                    0
                ));
                assert!(Balances::free_balance(ALICE) > alice_before);
                // report_license_revoked still works on MilestonePhase (protects unapproved
                // milestones)
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(GRIEFER),
                    id
                ));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Cancelled
                ));
            });
    }

    #[test]
    fn attack_cf_cat2_4_m_no_timeout() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
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
                run_to_block(10_000);
                assert_noop!(
                    Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
                assert_noop!(
                    Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 100),
                    Error::<Test>::InvalidCampaignStatus
                );
                assert_ok!(Crowdfunding::cancel_campaign(RuntimeOrigin::root(), id));
                assert_ok!(Crowdfunding::claim_refund(RuntimeOrigin::signed(BOB), id));
            },
        );
    }

    #[test]
    fn attack_cf_cat2_2_h_paused_asymmetric() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (CHARLIE, 10_000)])
            .build()
            .execute_with(|| {
                let id = create_funded_campaign(ALICE, default_aon_config(100, 1000));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(BOB), id, 600));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 400));
                run_to_block(10);
                assert_ok!(Crowdfunding::pause_campaign(RuntimeOrigin::root(), id));
                assert_ok!(Crowdfunding::withdraw_investment(RuntimeOrigin::signed(BOB), id, 600));
                assert_noop!(
                    Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id, 100),
                    Error::<Test>::InvalidCampaignStatus
                );
                run_to_block(50);
                assert_ok!(Crowdfunding::resume_campaign(RuntimeOrigin::root(), id));
                assert_eq!(pallet::Campaigns::<Test>::get(id).unwrap().total_raised, 400);
                run_to_block(141);
                assert_ok!(Crowdfunding::finalize_campaign(RuntimeOrigin::signed(ALICE), id));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id).unwrap().status,
                    CampaignStatus::Failed
                ));
            });
    }

    #[test]
    fn attack_cross_13_nuclear_retirement() {
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 50_000),
                (BOB, 50_000),
                (CHARLIE, 50_000),
                (DAVE, 50_000),
                (GRIEFER, 10_000),
            ])
            .build()
            .execute_with(|| {
                MockLicenseVerifier::set_license(RWA_ASSET_ID, 1, ALICE, true);
                MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, 1, 200);
                MockLicenseVerifier::set_license(RWA_ASSET_ID, 2, BOB, true);
                MockLicenseVerifier::set_license_expiry(RWA_ASSET_ID, 2, 200);
                let id_a = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(ALICE),
                    default_aon_config(20, 1000),
                    None,
                    Some((RWA_ASSET_ID, 1))
                ));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(CHARLIE), id_a, 1000));
                let id_b = pallet::NextCampaignId::<Test>::get();
                assert_ok!(Crowdfunding::create_campaign(
                    RuntimeOrigin::signed(BOB),
                    default_aon_config(20, 1000),
                    None,
                    Some((RWA_ASSET_ID, 2))
                ));
                assert_ok!(Crowdfunding::invest(RuntimeOrigin::signed(DAVE), id_b, 1000));
                MockLicenseVerifier::set_asset_active(RWA_ASSET_ID, false);
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(GRIEFER),
                    id_a
                ));
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(GRIEFER),
                    id_b
                ));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id_a).unwrap().status,
                    CampaignStatus::Cancelled
                ));
                assert!(matches!(
                    pallet::Campaigns::<Test>::get(id_b).unwrap().status,
                    CampaignStatus::Cancelled
                ));
            });
    }

    #[test]
    fn attack_cf_cat8_1_l_deposit_independent() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 10_000), (BOB, 10_000), (GRIEFER, 10_000)])
            .build()
            .execute_with(|| {
                let id = setup_funding_licensed_campaign();
                MockLicenseVerifier::set_active(RWA_ASSET_ID, PARTICIPATION_ID, false);
                assert_ok!(Crowdfunding::report_license_revoked(
                    RuntimeOrigin::signed(GRIEFER),
                    id
                ));
                let a = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_creation_deposit(RuntimeOrigin::signed(ALICE), id));
                assert_eq!(Balances::free_balance(ALICE) - a, 100);
            });
    }

    #[test]
    fn attack_cf_cat9_3_l_claim_succeeds_on_succeeded_despite_revoked() {
        ExtBuilder::default().balances(vec![(ALICE, 10_000), (BOB, 10_000)]).build().execute_with(
            || {
                let id = setup_succeeded_licensed_campaign();
                MockLicenseVerifier::set_active(RWA_ASSET_ID, PARTICIPATION_ID, false);
                // DEADLOCK-FIX: claim_funds succeeds despite revoked license.
                let alice_before = Balances::free_balance(ALICE);
                assert_ok!(Crowdfunding::claim_funds(RuntimeOrigin::signed(ALICE), id));
                assert!(Balances::free_balance(ALICE) > alice_before);
                // report_license_revoked still blocked on Succeeded (CRIT-01)
                assert_noop!(
                    Crowdfunding::report_license_revoked(RuntimeOrigin::signed(BOB), id),
                    Error::<Test>::InvalidCampaignStatus
                );
            },
        );
    }
}
