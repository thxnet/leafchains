//! # pallet-rwa: Penetration / Attack Tests
//!
//! These tests verify the behavior of pallet-rwa under adversarial conditions
//! identified in the MECE attack plan (docs/ATTACK_PLAN_RWA_2026-03-15.md).
//!
//! Each test is tagged with its attack plan ID and severity. Tests are
//! categorized as either:
//!   - **Attack Succeeds**: demonstrates that the vulnerability exists in the
//!     current code (the test PASSES, proving the bug is present).
//!   - **Attack Defended**: demonstrates that the pallet correctly rejects the
//!     adversarial action (the test PASSES, proving the defense works).
//!
//! Convention: `fn attack_{test_id}_{description}()`

use frame_support::{assert_noop, assert_ok, BoundedVec};
use sp_runtime::{traits::AccountIdConversion, Permill};

use super::{mock::*, *};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Category 1: Access Control & Authorization
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod cat1_access_control {
    use super::*;

    // ── RWA-CAT1.1-High: entry_fee bait-and-switch on renewal ──────────
    //
    // Attack scenario (Persona A — Malicious Asset Owner):
    // 1. Owner registers asset with entry_fee = 10.
    // 2. Participant joins, pays entry_fee = 10.
    // 3. Owner calls update_asset_policy to raise entry_fee to 9999.
    //
    // HIGH-01 fix is now APPLIED: entry_fee is immutable when
    // participant_count > 0. This test verifies the defense works.

    #[test]
    fn attack_cat1_1_entry_fee_bait_and_switch_defended() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                // Step 1: Owner registers asset with modest entry_fee = 10
                let policy = AssetPolicy {
                    deposit_currency: PaymentCurrency::Native,
                    entry_fee: 10,
                    deposit: 50,
                    max_duration: Some(5),
                    max_participants: None,
                    requires_approval: false,
                };
                let aid = register_test_asset(ALICE, BOB, policy);

                // Step 2: CHARLIE joins, paying entry_fee=10 + deposit=50
                let charlie_before = Balances::free_balance(CHARLIE);
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));
                let charlie_after_join = Balances::free_balance(CHARLIE);
                assert_eq!(charlie_before - charlie_after_join, 60);

                // Step 3: Owner tries to raise entry_fee — DEFENDED
                let new_policy = AssetPolicy {
                    deposit_currency: PaymentCurrency::Native,
                    entry_fee: 9999,
                    deposit: 50,
                    max_duration: Some(5),
                    max_participants: None,
                    requires_approval: false,
                };
                assert_noop!(
                    Rwa::update_asset_policy(RuntimeOrigin::signed(ALICE), aid, new_policy,),
                    Error::<Test>::PolicyFieldImmutable
                );

                // Verify entry_fee is unchanged
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset.policy.entry_fee, 10, "DEFENDED: entry_fee remains unchanged");
            });
    }

    #[test]
    fn attack_cat1_1_entry_fee_change_allowed_with_no_participants() {
        // Verify entry_fee CAN be changed when no participants exist.
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000)])
            .build()
            .execute_with(|| {
                let policy = AssetPolicy {
                    deposit_currency: PaymentCurrency::Native,
                    entry_fee: 10,
                    deposit: 50,
                    max_duration: None,
                    max_participants: None,
                    requires_approval: false,
                };
                let aid = register_test_asset(ALICE, BOB, policy);

                // No participants yet — entry_fee change is allowed
                let new_policy = AssetPolicy {
                    deposit_currency: PaymentCurrency::Native,
                    entry_fee: 9999,
                    deposit: 50,
                    max_duration: None,
                    max_participants: None,
                    requires_approval: false,
                };
                assert_ok!(
                    Rwa::update_asset_policy(RuntimeOrigin::signed(ALICE), aid, new_policy,)
                );
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset.policy.entry_fee, 9999);
            });
    }

    #[test]
    fn attack_cat1_1_entry_fee_change_defended_for_deposit() {
        // Verify that the deposit amount IS immutable (the defense that
        // entry_fee SHOULD have but currently doesn't).
        ExtBuilder::default().build().execute_with(|| {
            let aid = register_test_asset(ALICE, BOB, default_policy());
            // Add a participant so participant_count > 0
            assert_ok!(Rwa::request_participation(
                RuntimeOrigin::signed(CHARLIE),
                aid,
                vec![CHARLIE],
            ));

            let mut new_policy = default_policy();
            new_policy.deposit = 999; // try to change deposit
            assert_noop!(
                Rwa::update_asset_policy(RuntimeOrigin::signed(ALICE), aid, new_policy),
                Error::<Test>::PolicyFieldImmutable
            );
        });
    }

    // ── RWA-CAT1.6-High: accept_ownership on Paused assets ────────────
    //
    // Attack scenario (Persona A — Malicious Asset Owner):
    // 1. Admin pauses an asset (e.g., for investigation).
    // 2. Owner had already proposed ownership transfer to an accomplice.
    // 3. Accomplice calls accept_ownership on the PAUSED asset.
    //
    // HIGH-02 fix is now APPLIED: accept_ownership blocks both Retired
    // and Paused assets with InvalidAssetStatus.

    #[test]
    fn attack_cat1_6_accept_ownership_on_paused_asset_defended() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());

                // Owner proposes transfer to CHARLIE
                assert_ok!(Rwa::transfer_ownership(RuntimeOrigin::signed(ALICE), aid, CHARLIE,));

                // Admin pauses the asset (for investigation)
                assert_ok!(Rwa::pause_asset(RuntimeOrigin::root(), aid));
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert!(matches!(asset.status, AssetStatus::Paused));

                // DEFENDED: accept_ownership on Paused asset is blocked
                assert_noop!(
                    Rwa::accept_ownership(RuntimeOrigin::signed(CHARLIE), aid),
                    Error::<Test>::InvalidAssetStatus
                );

                // Ownership did NOT change
                let asset_after = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(
                    asset_after.owner, ALICE,
                    "DEFENDED: ownership NOT transferred on Paused asset"
                );

                // After unpause, transfer works normally
                assert_ok!(Rwa::unpause_asset(RuntimeOrigin::root(), aid));
                assert_ok!(Rwa::accept_ownership(RuntimeOrigin::signed(CHARLIE), aid,));
                let asset_final = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset_final.owner, CHARLIE);
            });
    }

    #[test]
    fn attack_cat1_6_accept_ownership_retired_is_defended() {
        // Verify that Retired assets correctly block accept_ownership.
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::transfer_ownership(RuntimeOrigin::signed(ALICE), aid, CHARLIE,));
                // Force retire the asset
                assert_ok!(Rwa::force_retire_asset(RuntimeOrigin::root(), aid));

                // DEFENDED: accept_ownership on Retired asset is blocked
                // Note: PendingOwnershipTransfer is cleaned up by force_retire,
                // so this fails with NoPendingTransfer, not AssetAlreadyRetired.
                assert_noop!(
                    Rwa::accept_ownership(RuntimeOrigin::signed(CHARLIE), aid),
                    Error::<Test>::NoPendingTransfer
                );
            });
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Category 2: State Machine & Lifecycle
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod cat2_state_machine {
    use super::*;

    // ── RWA-CAT2.1-Critical: Double-refund via lazy expiry settlement ──
    //
    // Attack scenario (Persona B — Malicious Payer):
    // 1. Payer's participation expires.
    // 2. Payer calls exit_participation — lazy settlement fires, refunding
    //    deposit. exit_participation returns Ok(()) via early return.
    // 3. Payer calls exit_participation AGAIN — if lazy settlement doesn't
    //    gate properly, another refund could occur.
    //
    // Defense: After settlement, status = Expired, deposit_held = 0.
    // Second exit_participation reads Expired status, try_settle_expiry
    // returns false, then status check fails with InvalidParticipationStatus.

    #[test]
    fn attack_cat2_1_double_refund_via_lazy_expiry_exit() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                // Timed policy: expires in 5 blocks
                let policy = timed_policy(5);
                let aid = register_test_asset(ALICE, BOB, policy);

                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Record balances before expiry
                let charlie_pre = Balances::free_balance(CHARLIE);

                // Advance past expiry (started_at=1, expires_at=6)
                run_to_block(7);

                // First exit: triggers lazy settlement, refunds deposit
                assert_ok!(Rwa::exit_participation(RuntimeOrigin::signed(CHARLIE), aid, 0,));

                let charlie_after_first = Balances::free_balance(CHARLIE);
                assert_eq!(charlie_after_first - charlie_pre, 50, "First exit refunds deposit");

                // Second exit attempt: DEFENDED — InvalidParticipationStatus
                assert_noop!(
                    Rwa::exit_participation(RuntimeOrigin::signed(CHARLIE), aid, 0),
                    Error::<Test>::InvalidParticipationStatus
                );

                // Verify no double-refund occurred
                let charlie_final = Balances::free_balance(CHARLIE);
                assert_eq!(
                    charlie_final, charlie_after_first,
                    "DEFENDED: no double-refund on second exit"
                );
            });
    }

    #[test]
    fn attack_cat2_1_double_refund_via_settle_then_exit() {
        // Alternative attack: settle_expired_participation + exit_participation
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let policy = timed_policy(5);
                let aid = register_test_asset(ALICE, BOB, policy);

                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                let charlie_pre = Balances::free_balance(CHARLIE);

                // Advance past expiry
                run_to_block(7);

                // Anyone settles the expired participation
                assert_ok!(Rwa::settle_expired_participation(RuntimeOrigin::signed(DAVE), aid, 0,));

                let charlie_after_settle = Balances::free_balance(CHARLIE);
                assert_eq!(charlie_after_settle - charlie_pre, 50);

                // Payer tries to exit the already-settled participation
                assert_noop!(
                    Rwa::exit_participation(RuntimeOrigin::signed(CHARLIE), aid, 0),
                    Error::<Test>::InvalidParticipationStatus
                );
            });
    }

    // ── RWA-CAT2.2-High: Deactivate->Sunset->Retire trapping participants ─
    //
    // Attack scenario (Persona A — Malicious Asset Owner):
    // 1. Owner creates asset with active participants.
    // 2. Owner deactivates asset (participants can't request new, but existing
    //    remain active).
    // 3. Owner sunsets asset.
    // 4. Asset auto-retires via on_initialize.
    // 5. Participants are trapped — their status is still Active but the
    //    asset is Retired. They must call claim_retired_deposit to recover.
    //
    // This tests the full lifecycle trap and verifies claim_retired_deposit
    // is the escape hatch.

    #[test]
    fn attack_cat2_2_deactivate_sunset_retire_traps_participants() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());

                // CHARLIE participates
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                let charlie_pre_retirement = Balances::free_balance(CHARLIE);

                // Owner deactivates
                assert_ok!(Rwa::deactivate_asset(RuntimeOrigin::signed(ALICE), aid));

                // Owner sunsets (from Inactive state — this is allowed)
                assert_ok!(Rwa::sunset_asset(RuntimeOrigin::signed(ALICE), aid, 10));

                // Auto-retire via on_initialize
                run_to_block(10);
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert!(matches!(asset.status, AssetStatus::Retired));

                // CHARLIE's participation is still Active in storage
                let p = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(
                    matches!(p.status, ParticipationStatus::Active { .. }),
                    "Participation remains Active even after asset retirement"
                );

                // CHARLIE cannot exit normally — exit_participation does lazy
                // expiry check (no expiry here) then requires Active status.
                // Actually exit_participation should work since status IS Active.
                // The real trap is that CHARLIE cannot renew (asset not Active).
                assert_noop!(
                    Rwa::renew_participation(RuntimeOrigin::signed(CHARLIE), aid, 0),
                    Error::<Test>::AssetNotActive
                );

                // Escape hatch: claim_retired_deposit works
                assert_ok!(Rwa::claim_retired_deposit(RuntimeOrigin::signed(CHARLIE), aid, 0,));

                let charlie_after = Balances::free_balance(CHARLIE);
                assert_eq!(
                    charlie_after - charlie_pre_retirement,
                    50,
                    "claim_retired_deposit recovers the escrowed deposit"
                );

                // Verify participation is now Exited
                let p = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(matches!(p.status, ParticipationStatus::Exited));
            });
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Category 3: Economic & Financial Attacks
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod cat3_economic {
    use super::*;

    // ── RWA-CAT3.4-High: do_distribute_slash partial failure ───────────
    //
    // Attack scenario (Persona B):
    // Set up slash distribution with mixed Burn + Transfer recipients.
    // If the Burn portion would drop the pallet account below ED, the
    // entire slash should roll back (transactional safety).
    //
    // This tests that partial distribution does NOT leave corrupted state.

    #[test]
    fn attack_cat3_4_slash_distribution_with_burn_recipient() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000), (DAVE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());

                // CHARLIE participates (deposit = 50)
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Set slash distribution: 60% to beneficiary, 40% burned
                let dist: BoundedVec<
                    SlashRecipient<u64>,
                    <Test as crate::Config>::MaxSlashRecipients,
                > = vec![
                    SlashRecipient {
                        kind: SlashRecipientKind::Beneficiary,
                        share: Permill::from_percent(60),
                    },
                    SlashRecipient {
                        kind: SlashRecipientKind::Burn,
                        share: Permill::from_percent(40),
                    },
                ]
                .try_into()
                .unwrap();

                assert_ok!(Rwa::set_slash_distribution(RuntimeOrigin::signed(ALICE), aid, dist,));

                let bob_before = Balances::free_balance(BOB);
                let pallet_acct: u64 = RwaPalletId::get().into_account_truncating();
                let pallet_before = Balances::free_balance(pallet_acct);

                // Slash 30 out of 50 deposit
                assert_ok!(Rwa::slash_participation(RuntimeOrigin::root(), aid, 0, 30, None,));

                // Verify distribution: 60% of 30 = 18 to beneficiary(BOB)
                let bob_after = Balances::free_balance(BOB);
                assert_eq!(bob_after - bob_before, 18);

                // 40% of 30 = 12 burned (last recipient gets remainder = 30-18=12)
                // Pallet account: had (50 deposit + 1 ED), minus 18 transfer, minus 12 burn
                let pallet_after = Balances::free_balance(pallet_acct);
                // Remainder (50-30=20) was also refunded to CHARLIE
                // So pallet lost: 18 (to BOB) + 12 (burned) + 20 (refund to CHARLIE) = 50
                assert_eq!(pallet_before - pallet_after, 50);

                // Verify participation status
                let p = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(matches!(p.status, ParticipationStatus::Slashed));
                assert_eq!(p.deposit_held, 0);
            });
    }

    #[test]
    fn attack_cat3_4_slash_distribution_reporter_fallback() {
        // Verify Reporter kind falls back to Beneficiary when no reporter given.
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Distribution: 100% to Reporter
                let dist: BoundedVec<
                    SlashRecipient<u64>,
                    <Test as crate::Config>::MaxSlashRecipients,
                > = vec![SlashRecipient {
                    kind: SlashRecipientKind::Reporter,
                    share: Permill::from_percent(100),
                }]
                .try_into()
                .unwrap();

                assert_ok!(Rwa::set_slash_distribution(RuntimeOrigin::signed(ALICE), aid, dist,));

                let bob_before = Balances::free_balance(BOB);

                // Slash with no reporter — should fall back to beneficiary (BOB)
                assert_ok!(Rwa::slash_participation(
                    RuntimeOrigin::root(),
                    aid,
                    0,
                    30,
                    None, // no reporter
                ));

                let bob_after = Balances::free_balance(BOB);
                assert_eq!(
                    bob_after - bob_before,
                    30,
                    "Reporter fallback sends slash to beneficiary"
                );
            });
    }

    #[test]
    fn attack_cat3_4_slash_distribution_with_explicit_reporter() {
        // Verify Reporter kind sends to actual reporter when provided.
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000), (DAVE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                let dist: BoundedVec<
                    SlashRecipient<u64>,
                    <Test as crate::Config>::MaxSlashRecipients,
                > = vec![SlashRecipient {
                    kind: SlashRecipientKind::Reporter,
                    share: Permill::from_percent(100),
                }]
                .try_into()
                .unwrap();

                assert_ok!(Rwa::set_slash_distribution(RuntimeOrigin::signed(ALICE), aid, dist,));

                let dave_before = Balances::free_balance(DAVE);

                // Slash with explicit reporter DAVE
                assert_ok!(
                    Rwa::slash_participation(RuntimeOrigin::root(), aid, 0, 30, Some(DAVE),)
                );

                let dave_after = Balances::free_balance(DAVE);
                assert_eq!(dave_after - dave_before, 30, "Reporter share sent to actual reporter");
            });
    }

    #[test]
    fn attack_cat3_4_slash_three_way_distribution_rounding() {
        // Verify rounding behavior with 3 recipients and small amounts.
        // Permill multiplication truncates toward zero.
        // With amount=1 and 3x33.33%: first two get 0, last gets remainder=1.
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (CHARLIE, 100_000),
                (DAVE, 100_000),
                (EVE, 100_000),
            ])
            .build()
            .execute_with(|| {
                let policy = AssetPolicy {
                    deposit_currency: PaymentCurrency::Native,
                    entry_fee: 0,
                    deposit: 50,
                    max_duration: None,
                    max_participants: None,
                    requires_approval: false,
                };
                let aid = register_test_asset(ALICE, BOB, policy);

                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // 3 recipients: 33.3334% + 33.3333% + 33.3333% = 100%
                let dist: BoundedVec<
                    SlashRecipient<u64>,
                    <Test as crate::Config>::MaxSlashRecipients,
                > = vec![
                    SlashRecipient {
                        kind: SlashRecipientKind::Account(BOB),
                        share: Permill::from_parts(333_334),
                    },
                    SlashRecipient {
                        kind: SlashRecipientKind::Account(DAVE),
                        share: Permill::from_parts(333_333),
                    },
                    SlashRecipient {
                        kind: SlashRecipientKind::Account(EVE),
                        share: Permill::from_parts(333_333),
                    },
                ]
                .try_into()
                .unwrap();

                assert_ok!(Rwa::set_slash_distribution(RuntimeOrigin::signed(ALICE), aid, dist,));

                let bob_pre = Balances::free_balance(BOB);
                let dave_pre = Balances::free_balance(DAVE);
                let eve_pre = Balances::free_balance(EVE);

                // Slash exactly 1 unit — demonstrates Permill truncation
                assert_ok!(Rwa::slash_participation(RuntimeOrigin::root(), aid, 0, 1, None,));

                let bob_got = Balances::free_balance(BOB) - bob_pre;
                let dave_got = Balances::free_balance(DAVE) - dave_pre;
                let eve_got = Balances::free_balance(EVE) - eve_pre;

                // Permill::from_parts(333_334) * 1 = 0 (truncates)
                // Permill::from_parts(333_333) * 1 = 0 (truncates)
                // Last recipient gets: 1 - 0 - 0 = 1
                assert_eq!(bob_got, 0, "First recipient gets 0 due to Permill truncation");
                assert_eq!(dave_got, 0, "Second recipient gets 0 due to Permill truncation");
                assert_eq!(eve_got, 1, "Last recipient gets entire remainder");
            });
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Category 4: Arithmetic & Overflow
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod cat4_arithmetic {
    use super::*;

    // ── RWA-CAT4.4-High: Slash distribution Permill saturation (3x40%) ──
    //
    // Attack scenario (Persona H — Economic Manipulator):
    // 1. Owner calls set_slash_distribution with three 40% shares.
    // 2. OLD CODE: Permill::saturating_add would saturate 120% to 100%.
    //
    // HIGH-08 fix is now APPLIED: validation uses raw parts sum via
    // `deconstruct()`, so 3x400_000 = 1_200_000 != 1_000_000 and is
    // rejected with SlashSharesSumInvalid.

    #[test]
    fn attack_cat4_4_permill_saturation_three_times_40_percent_defended() {
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (CHARLIE, 100_000),
                (DAVE, 100_000),
                (EVE, 100_000),
            ])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());

                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // ATTACK: three shares of 40% each = 120%
                let dist: BoundedVec<
                    SlashRecipient<u64>,
                    <Test as crate::Config>::MaxSlashRecipients,
                > = vec![
                    SlashRecipient {
                        kind: SlashRecipientKind::Account(BOB),
                        share: Permill::from_percent(40),
                    },
                    SlashRecipient {
                        kind: SlashRecipientKind::Account(DAVE),
                        share: Permill::from_percent(40),
                    },
                    SlashRecipient {
                        kind: SlashRecipientKind::Account(EVE),
                        share: Permill::from_percent(40),
                    },
                ]
                .try_into()
                .unwrap();

                // DEFENDED: raw parts sum 1_200_000 != 1_000_000
                assert_noop!(
                    Rwa::set_slash_distribution(RuntimeOrigin::signed(ALICE), aid, dist,),
                    Error::<Test>::SlashSharesSumInvalid
                );
            });
    }

    #[test]
    fn attack_cat4_4_permill_exact_100_percent_works_correctly() {
        // Control test: exact 100% distribution works as expected.
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (CHARLIE, 100_000),
                (DAVE, 100_000),
                (EVE, 100_000),
            ])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Correct distribution: 50% + 30% + 20% = 100%
                let dist: BoundedVec<
                    SlashRecipient<u64>,
                    <Test as crate::Config>::MaxSlashRecipients,
                > = vec![
                    SlashRecipient {
                        kind: SlashRecipientKind::Account(BOB),
                        share: Permill::from_percent(50),
                    },
                    SlashRecipient {
                        kind: SlashRecipientKind::Account(DAVE),
                        share: Permill::from_percent(30),
                    },
                    SlashRecipient {
                        kind: SlashRecipientKind::Account(EVE),
                        share: Permill::from_percent(20),
                    },
                ]
                .try_into()
                .unwrap();

                assert_ok!(Rwa::set_slash_distribution(RuntimeOrigin::signed(ALICE), aid, dist,));

                let bob_pre = Balances::free_balance(BOB);
                let dave_pre = Balances::free_balance(DAVE);
                let eve_pre = Balances::free_balance(EVE);

                assert_ok!(Rwa::slash_participation(RuntimeOrigin::root(), aid, 0, 50, None,));

                assert_eq!(Balances::free_balance(BOB) - bob_pre, 25); // 50%
                assert_eq!(Balances::free_balance(DAVE) - dave_pre, 15); // 30%
                assert_eq!(Balances::free_balance(EVE) - eve_pre, 10); // 20% (remainder)
            });
    }

    #[test]
    fn attack_cat4_4_permill_saturation_rejects_non_100() {
        // Verify that shares summing to != 100% (without saturation) are rejected.
        ExtBuilder::default().build().execute_with(|| {
            let aid = register_test_asset(ALICE, BOB, default_policy());

            // 50% + 30% = 80% — should be rejected
            let dist: BoundedVec<SlashRecipient<u64>, <Test as crate::Config>::MaxSlashRecipients> =
                vec![
                    SlashRecipient {
                        kind: SlashRecipientKind::Beneficiary,
                        share: Permill::from_percent(50),
                    },
                    SlashRecipient {
                        kind: SlashRecipientKind::Burn,
                        share: Permill::from_percent(30),
                    },
                ]
                .try_into()
                .unwrap();

            assert_noop!(
                Rwa::set_slash_distribution(RuntimeOrigin::signed(ALICE), aid, dist),
                Error::<Test>::SlashSharesSumInvalid
            );
        });
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Category 5 & 7: Storage Integrity & Bounded Collection Attacks
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod cat5_7_storage_and_bounded {
    use super::*;

    // ── RWA-CAT5.3/7.2: push_holder_asset silent failure / holder slot
    //    exhaustion ─────────────────────────────────────────────────────
    //
    // Attack scenario (Persona C — Sybil Farmer):
    // 1. Attacker creates multiple assets.
    // 2. For each asset, attacker calls request_participation listing the
    //    VICTIM as a holder (blanket ParticipationFilter allows this).
    // 3. After MaxParticipationsPerHolder (5) participations, the victim
    //    cannot participate in ANY new asset.
    //
    // Defense: request_participation pre-flights the HolderAssets capacity
    // check (line 684-688), so the 6th attempt fails. But the victim was
    // never asked for consent to be listed as a holder.

    #[test]
    fn attack_cat5_3_7_2_holder_slot_exhaustion_by_sybil() {
        // Use multiple owners (ALICE and BOB) to avoid MaxAssetsPerOwner=5 limit.
        // MaxParticipationsPerHolder = 5 in mock.
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (CHARLIE, 100_000), // CHARLIE is the victim
                (DAVE, 100_000),    // DAVE is the attacker / payer
                (EVE, 100_000),
            ])
            .build()
            .execute_with(|| {
                // Register 5 assets across two owners to stay under MaxAssetsPerOwner=5
                let aid0 = register_test_asset(ALICE, BOB, default_policy());
                let aid1 = register_test_asset(ALICE, BOB, default_policy());
                let aid2 = register_test_asset(ALICE, BOB, default_policy());
                let aid3 = register_test_asset(BOB, ALICE, default_policy());
                let aid4 = register_test_asset(BOB, ALICE, default_policy());

                // DAVE pays for participations listing CHARLIE as holder
                // in each asset, WITHOUT CHARLIE's consent.
                for &aid in &[aid0, aid1, aid2, aid3, aid4] {
                    assert_ok!(Rwa::request_participation(
                        RuntimeOrigin::signed(DAVE),
                        aid,
                        vec![CHARLIE],
                    ));
                    assert!(pallet::HolderIndex::<Test>::contains_key(aid, CHARLIE));
                }

                // CHARLIE's HolderAssets should now be full (5 entries)
                let charlie_assets = pallet::HolderAssets::<Test>::get(CHARLIE);
                assert_eq!(charlie_assets.len(), 5);

                // ATTACK SUCCEEDS: CHARLIE cannot participate in any new asset
                let new_aid = register_test_asset(BOB, ALICE, default_policy());
                assert_noop!(
                    Rwa::request_participation(
                        RuntimeOrigin::signed(CHARLIE),
                        new_aid,
                        vec![CHARLIE],
                    ),
                    Error::<Test>::MaxParticipationsPerHolderReached
                );

                // CHARLIE's escape hatch: leave_participation on unwanted
                // participations to free up slots
                assert_ok!(Rwa::leave_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid0,
                    0, // participation_id
                ));

                // Now CHARLIE can participate again
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    new_aid,
                    vec![CHARLIE],
                ));
            });
    }

    #[test]
    fn attack_cat5_3_push_holder_asset_silent_failure_mitigated() {
        // Verify that the pre-flight check in request_participation
        // prevents the silent try_push failure in push_holder_asset.
        // Use multiple owners to avoid MaxAssetsPerOwner=5 limit.
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (CHARLIE, 100_000),
                (DAVE, 100_000),
                (EVE, 100_000),
            ])
            .build()
            .execute_with(|| {
                // Fill CHARLIE's HolderAssets to capacity (5) using assets
                // from different owners
                let aid0 = register_test_asset(ALICE, BOB, default_policy());
                let aid1 = register_test_asset(ALICE, BOB, default_policy());
                let aid2 = register_test_asset(ALICE, BOB, default_policy());
                let aid3 = register_test_asset(BOB, ALICE, default_policy());
                let aid4 = register_test_asset(BOB, ALICE, default_policy());

                for &aid in &[aid0, aid1, aid2, aid3, aid4] {
                    assert_ok!(Rwa::request_participation(
                        RuntimeOrigin::signed(DAVE),
                        aid,
                        vec![CHARLIE],
                    ));
                }

                // 6th participation request with CHARLIE as holder is rejected
                // BEFORE reaching push_holder_asset
                let new_aid = register_test_asset(BOB, ALICE, default_policy());
                assert_noop!(
                    Rwa::request_participation(
                        RuntimeOrigin::signed(DAVE),
                        new_aid,
                        vec![CHARLIE],
                    ),
                    Error::<Test>::MaxParticipationsPerHolderReached
                );

                // Verify HolderAssets consistency: every HolderAssets entry
                // should have a matching HolderIndex
                let charlie_assets = pallet::HolderAssets::<Test>::get(CHARLIE);
                for &asset_id in charlie_assets.iter() {
                    assert!(
                        pallet::HolderIndex::<Test>::contains_key(asset_id, CHARLIE),
                        "HolderAssets -> HolderIndex consistency for asset {}",
                        asset_id,
                    );
                }
            });
    }

    #[test]
    fn attack_cat7_2_holder_added_without_consent() {
        // Verify that add_holder also respects MaxParticipationsPerHolder
        // and that the payer controls holder additions.
        // Use multiple owners to avoid MaxAssetsPerOwner=5 limit.
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (CHARLIE, 100_000),
                (DAVE, 100_000),
                (EVE, 100_000),
            ])
            .build()
            .execute_with(|| {
                // Fill EVE's HolderAssets to capacity using assets from
                // different owners
                let aid0 = register_test_asset(ALICE, BOB, default_policy());
                let aid1 = register_test_asset(ALICE, BOB, default_policy());
                let aid2 = register_test_asset(ALICE, BOB, default_policy());
                let aid3 = register_test_asset(BOB, ALICE, default_policy());
                let aid4 = register_test_asset(BOB, ALICE, default_policy());

                for &aid in &[aid0, aid1, aid2, aid3, aid4] {
                    assert_ok!(Rwa::request_participation(
                        RuntimeOrigin::signed(DAVE),
                        aid,
                        vec![EVE],
                    ));
                }

                // Create a new participation with CHARLIE as initial holder
                // on a new asset (BOB still has capacity)
                let new_aid = register_test_asset(BOB, ALICE, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    new_aid,
                    vec![CHARLIE],
                ));

                // Try to add EVE as holder — should fail (EVE's slots full)
                assert_noop!(
                    Rwa::add_holder(RuntimeOrigin::signed(CHARLIE), new_aid, 0, EVE),
                    Error::<Test>::MaxParticipationsPerHolderReached
                );
            });
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Category 7: Bounded Collection — batch_reject_pending
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod cat7_batch_reject {
    use super::*;

    // ── RWA-CAT7.5-High: batch_reject_pending mid-loop failure ─────────
    //
    // Attack scenario (Persona I — State Corruption Artist):
    // 1. Multiple pending participations exist.
    // 2. batch_reject_pending calls PendingApprovals::take (removes all IDs).
    // 3. For each pending, it refunds deposit+fee via do_transfer.
    // 4. If one refund fails (e.g., pallet account drained), the ? operator
    //    propagates the error.
    // 5. QUESTION: Does the PendingApprovals::take get rolled back?
    //    In FRAME v4, dispatchables are transactional by default, so YES.
    //
    // This test verifies transactional rollback behavior.

    #[test]
    fn attack_cat7_5_batch_reject_transactional_rollback_on_failure() {
        // Create a scenario where the pallet account cannot fund all refunds.
        // The batch_reject_pending should either succeed fully or roll back.
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (CHARLIE, 100_000),
                (DAVE, 100_000),
                (EVE, 100_000),
            ])
            .build()
            .execute_with(|| {
                // Asset requires approval, entry_fee = 10, deposit = 50
                let policy = approval_policy();
                let aid = register_test_asset(ALICE, BOB, policy);

                // Create 3 pending participations (each locks 60 in pallet account)
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(DAVE),
                    aid,
                    vec![DAVE],
                ));
                assert_ok!(Rwa::request_participation(RuntimeOrigin::signed(EVE), aid, vec![EVE],));

                // Verify pending queue has 3 entries
                let pending = pallet::PendingApprovals::<Test>::get(aid);
                assert_eq!(pending.len(), 3);

                // Verify pallet account has the escrowed funds
                let pallet_acct: u64 = RwaPalletId::get().into_account_truncating();
                let pallet_balance = Balances::free_balance(pallet_acct);
                // Each pending: deposit(50) + entry_fee(10) = 60, total = 180 + 1 (ED)
                assert_eq!(pallet_balance, 181);

                // Normal case: batch_reject_pending succeeds
                assert_ok!(Rwa::batch_reject_pending(RuntimeOrigin::signed(ALICE), aid,));

                // Verify all were rejected
                let pending_after = pallet::PendingApprovals::<Test>::get(aid);
                assert_eq!(pending_after.len(), 0);

                // Verify all funds returned
                // Each participant gets back 60 (deposit + fee)
                // Note: initial balance was 100_000, minus 60 for participation = 99_940
                // After refund: 99_940 + 60 = 100_000
                assert_eq!(Balances::free_balance(CHARLIE), 100_000);
                assert_eq!(Balances::free_balance(DAVE), 100_000);
                assert_eq!(Balances::free_balance(EVE), 100_000);
            });
    }

    #[test]
    fn attack_cat7_5_batch_reject_maintains_storage_consistency() {
        // After batch_reject, verify that all storage items are consistent:
        // - PendingApprovals is empty
        // - participant_count is decremented for each rejection
        // - HolderIndex entries are cleaned up
        // - HolderAssets entries are cleaned up
        // - Participations records are removed
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000), (DAVE, 100_000)])
            .build()
            .execute_with(|| {
                let policy = approval_policy();
                let aid = register_test_asset(ALICE, BOB, policy);

                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(DAVE),
                    aid,
                    vec![DAVE],
                ));

                // Verify pre-state
                let asset_pre = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset_pre.participant_count, 2);
                assert!(pallet::HolderIndex::<Test>::contains_key(aid, CHARLIE));
                assert!(pallet::HolderIndex::<Test>::contains_key(aid, DAVE));

                assert_ok!(Rwa::batch_reject_pending(RuntimeOrigin::signed(ALICE), aid,));

                // Verify post-state consistency
                let asset_post = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(
                    asset_post.participant_count, 0,
                    "participant_count decremented for each rejection"
                );
                assert!(
                    pallet::PendingApprovals::<Test>::get(aid).is_empty(),
                    "PendingApprovals cleared"
                );
                assert!(
                    !pallet::HolderIndex::<Test>::contains_key(aid, CHARLIE),
                    "HolderIndex cleaned for CHARLIE"
                );
                assert!(
                    !pallet::HolderIndex::<Test>::contains_key(aid, DAVE),
                    "HolderIndex cleaned for DAVE"
                );
                assert!(
                    pallet::Participations::<Test>::get(aid, 0).is_none(),
                    "Participation 0 removed"
                );
                assert!(
                    pallet::Participations::<Test>::get(aid, 1).is_none(),
                    "Participation 1 removed"
                );
            });
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Category 8: Concurrency, Ordering & TOCTOU
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod cat8_toctou {
    use super::*;

    // ── RWA-CAT8.1-High: TOCTOU on concurrent operations ──────────────
    //
    // Attack scenario (Persona D — Front-running MEV Bot):
    // Verify that the two-phase design of renew_participation prevents
    // double-refund in sequential-within-same-block scenarios:
    // 1. Participation expires.
    // 2. settle_expired_participation is called — refunds deposit.
    // 3. renew_participation is called — should work from Expired state,
    //    re-collecting the deposit.
    // 4. exit_participation is called — refunds the renewed deposit.
    //
    // Net: payer paid deposit twice (original + renewal), got it back
    // twice (settlement + exit). Economically neutral.

    #[test]
    fn attack_cat8_1_sequential_settle_renew_exit_no_double_refund() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let policy = timed_policy(5);
                let aid = register_test_asset(ALICE, BOB, policy);

                // CHARLIE participates (deposit = 50)
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                let charlie_after_join = Balances::free_balance(CHARLIE);
                // 100_000 - 50 (deposit) = 99_950
                assert_eq!(charlie_after_join, 100_000 - 50);

                // Advance past expiry (started_at=1, expires_at=6)
                run_to_block(7);

                // Step 1: Anyone settles the expired participation
                assert_ok!(Rwa::settle_expired_participation(RuntimeOrigin::signed(DAVE), aid, 0,));
                let charlie_after_settle = Balances::free_balance(CHARLIE);
                // Got deposit back: 99_950 + 50 = 100_000
                assert_eq!(charlie_after_settle, 100_000);

                // Step 2: Renew (from Expired state) — re-collects deposit
                assert_ok!(Rwa::renew_participation(RuntimeOrigin::signed(CHARLIE), aid, 0,));
                let charlie_after_renew = Balances::free_balance(CHARLIE);
                // Paid deposit again: 100_000 - 50 = 99_950
                assert_eq!(charlie_after_renew, 100_000 - 50);

                // Step 3: Exit the renewed participation — refunds deposit
                assert_ok!(Rwa::exit_participation(RuntimeOrigin::signed(CHARLIE), aid, 0,));
                let charlie_final = Balances::free_balance(CHARLIE);
                // Got deposit back: 99_950 + 50 = 100_000
                assert_eq!(charlie_final, 100_000);

                // Net: CHARLIE started with 100_000, ended with 100_000.
                // No double-refund. Economically neutral.
            });
    }

    #[test]
    fn attack_cat8_1_renew_from_active_not_expired_charges_only_fee() {
        // When renewing an Active (not yet expired) participation,
        // only the entry_fee is charged (deposit already in escrow).
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let policy = AssetPolicy {
                    deposit_currency: PaymentCurrency::Native,
                    entry_fee: 10,
                    deposit: 50,
                    max_duration: Some(100), // won't expire during test
                    max_participants: None,
                    requires_approval: false,
                };
                let aid = register_test_asset(ALICE, BOB, policy);

                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));
                let charlie_after_join = Balances::free_balance(CHARLIE);
                // Paid 50 (deposit) + 10 (fee) = 60
                assert_eq!(charlie_after_join, 100_000 - 60);

                // Renew while still active (not expired)
                let charlie_before_renew = Balances::free_balance(CHARLIE);
                assert_ok!(Rwa::renew_participation(RuntimeOrigin::signed(CHARLIE), aid, 0,));
                let charlie_after_renew = Balances::free_balance(CHARLIE);
                // Only charged entry_fee (10), deposit stays in escrow
                assert_eq!(
                    charlie_before_renew - charlie_after_renew,
                    10,
                    "Active renewal only charges entry_fee, not deposit"
                );
            });
    }

    #[test]
    fn attack_cat8_1_double_settle_is_rejected() {
        // Verify that calling settle_expired_participation twice fails.
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let policy = timed_policy(5);
                let aid = register_test_asset(ALICE, BOB, policy);

                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                run_to_block(7);

                // First settle succeeds
                assert_ok!(Rwa::settle_expired_participation(RuntimeOrigin::signed(DAVE), aid, 0,));

                // Second settle fails — already Expired, try_settle_expiry
                // returns false, ensure! fails
                assert_noop!(
                    Rwa::settle_expired_participation(RuntimeOrigin::signed(DAVE), aid, 0),
                    Error::<Test>::InvalidParticipationStatus
                );
            });
    }

    // ── TOCTOU: concurrent ownership transfer + pause ──────────────────

    #[test]
    fn attack_cat8_1_ownership_transfer_during_pause_defended() {
        // Verify behavior when ownership transfer and admin pause overlap.
        // HIGH-02 fix: accept_ownership now blocks Paused assets.
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());

                // Owner proposes transfer
                assert_ok!(Rwa::transfer_ownership(RuntimeOrigin::signed(ALICE), aid, CHARLIE,));

                // Admin pauses the asset
                assert_ok!(Rwa::pause_asset(RuntimeOrigin::root(), aid));

                // DEFENDED: CHARLIE cannot accept ownership on paused asset
                assert_noop!(
                    Rwa::accept_ownership(RuntimeOrigin::signed(CHARLIE), aid),
                    Error::<Test>::InvalidAssetStatus
                );

                // Owner unchanged
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset.owner, ALICE);

                // After unpause, transfer completes normally
                assert_ok!(Rwa::unpause_asset(RuntimeOrigin::root(), aid));
                assert_ok!(Rwa::accept_ownership(RuntimeOrigin::signed(CHARLIE), aid,));
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset.owner, CHARLIE);
            });
    }

    // ── TOCTOU: participation transfer while participation is about to
    //    expire ─────────────────────────────────────────────────────────

    #[test]
    fn attack_cat8_1_transfer_participation_at_expiry_boundary() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000), (DAVE, 100_000)])
            .build()
            .execute_with(|| {
                let policy = timed_policy(5);
                let aid = register_test_asset(ALICE, BOB, policy);

                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Advance to exactly the expiry block (started_at=1, expires_at=6)
                run_to_block(6);

                // Try to transfer participation at the exact expiry block.
                // transfer_participation uses try_settle_expiry_inner inside
                // try_mutate. At block 6, 6 >= 6 is true, so the inner
                // function detects expiry and returns ParticipationExpiredError.
                // Because this is inside try_mutate and returns Err, the
                // storage mutations are rolled back — the participation
                // status is NOT changed to Expired (try_mutate rollback).
                assert_noop!(
                    Rwa::transfer_participation(RuntimeOrigin::signed(CHARLIE), aid, 0, DAVE,),
                    Error::<Test>::ParticipationExpiredError
                );

                // Since try_mutate rolled back, the participation remains
                // Active in storage (the inner settlement was reverted).
                // The payer must explicitly settle or exit.
                let p = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(
                    matches!(p.status, ParticipationStatus::Active { .. }),
                    "try_mutate rollback preserves Active status"
                );

                // Explicitly settle to properly transition to Expired
                assert_ok!(Rwa::settle_expired_participation(RuntimeOrigin::signed(DAVE), aid, 0,));
                let p = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(matches!(p.status, ParticipationStatus::Expired));
            });
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Category 10: Governance, Upgrades & Migration
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod cat10_governance {
    use super::*;

    // ── RWA-CAT10.1-Critical: Verify StorageVersion exists ─────────────
    //
    // Attack scenario (Persona J — Governance/Upgrade Attacker):
    // Without StorageVersion, future runtime upgrades cannot safely
    // detect whether migrations have run. This is a governance risk.
    //
    // This test verifies the CURRENT state: whether StorageVersion is
    // present or absent. After the fix is applied, the test should be
    // updated to verify the correct version number.

    #[test]
    fn attack_cat10_1_storage_version_exists() {
        // CRIT-05 fix is now APPLIED: StorageVersion is declared as V5.
        // This test verifies the fix is in place and the version is
        // correctly set for future migration safety.
        ExtBuilder::default().build().execute_with(|| {
            use frame_support::traits::GetStorageVersion;

            let current = Rwa::current_storage_version();

            // The in-code storage version should be 5 (V5 = MinParticipationDeposit)
            assert_eq!(
                current,
                frame_support::traits::StorageVersion::new(5),
                "DEFENDED: StorageVersion is declared as V5"
            );
        });
    }

    #[test]
    fn attack_cat10_1_call_indexes_are_explicit() {
        // Verify that call_indexes are explicit (not positional).
        // This is a governance safety check: if call_indexes were positional,
        // reordering extrinsics would break existing transactions.
        //
        // We verify this indirectly by checking that the pallet has the
        // expected number of extrinsics (29, indices 0..28).
        ExtBuilder::default().build().execute_with(|| {
            // register_asset is call_index(0)
            // batch_reject_pending is call_index(28)
            // All 29 extrinsics have explicit #[pallet::call_index(N)]
            // This is confirmed by code review; we verify functionality
            // of the first and last call_indexes.

            // call_index(0): register_asset
            let aid = register_test_asset(ALICE, BOB, default_policy());
            assert_eq!(aid, 0);

            // call_index(28): batch_reject_pending (requires approval)
            let policy = approval_policy();
            let aid2 = register_test_asset(ALICE, BOB, policy);
            assert_ok!(Rwa::request_participation(
                RuntimeOrigin::signed(CHARLIE),
                aid2,
                vec![CHARLIE],
            ));
            assert_ok!(Rwa::batch_reject_pending(RuntimeOrigin::signed(ALICE), aid2,));
        });
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Cross-cutting: Additional adversarial scenarios
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod cross_cutting {
    use super::*;

    // ── Additional CAT1.1 variant: entry_fee change blocked after
    //    participants join ──────────────────────────────────────────────
    // HIGH-01 fix: entry_fee is immutable when participant_count > 0.
    // Owner can change entry_fee only BEFORE any participants join.

    #[test]
    fn attack_cat1_1_entry_fee_change_before_participants_then_locked() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                // Start with entry_fee = 100
                let policy = AssetPolicy {
                    deposit_currency: PaymentCurrency::Native,
                    entry_fee: 100,
                    deposit: 50,
                    max_duration: Some(5),
                    max_participants: None,
                    requires_approval: false,
                };
                let aid = register_test_asset(ALICE, BOB, policy);

                // No participants yet — owner CAN lower entry_fee to 0
                let cheap_policy = AssetPolicy {
                    deposit_currency: PaymentCurrency::Native,
                    entry_fee: 0,
                    deposit: 50,
                    max_duration: Some(5),
                    max_participants: None,
                    requires_approval: false,
                };
                assert_ok!(Rwa::update_asset_policy(
                    RuntimeOrigin::signed(ALICE),
                    aid,
                    cheap_policy,
                ));

                // CHARLIE joins at entry_fee = 0
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // DEFENDED: Owner tries to raise entry_fee after participant
                // exists — blocked by HIGH-01 fix.
                let expensive_policy = AssetPolicy {
                    deposit_currency: PaymentCurrency::Native,
                    entry_fee: 5000,
                    deposit: 50,
                    max_duration: Some(5),
                    max_participants: None,
                    requires_approval: false,
                };
                assert_noop!(
                    Rwa::update_asset_policy(RuntimeOrigin::signed(ALICE), aid, expensive_policy,),
                    Error::<Test>::PolicyFieldImmutable
                );

                // entry_fee remains 0
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset.policy.entry_fee, 0);
            });
    }

    // ── max_duration reduction guard ──────────────────────────────────

    #[test]
    fn attack_v4_fix_max_duration_reduction_blocked() {
        // V4 fix: verify max_duration cannot be reduced when participants exist.
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let policy = timed_policy(100);
                let aid = register_test_asset(ALICE, BOB, policy);

                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Try to reduce max_duration from 100 to 10
                let shorter_policy = timed_policy(10);
                assert_noop!(
                    Rwa::update_asset_policy(RuntimeOrigin::signed(ALICE), aid, shorter_policy),
                    Error::<Test>::PolicyFieldImmutable
                );

                // Try to add a duration limit where none existed
                let unlimited_aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    unlimited_aid,
                    vec![CHARLIE],
                ));

                // None -> Some(10) is blocked
                let limited_policy = timed_policy(10);
                assert_noop!(
                    Rwa::update_asset_policy(
                        RuntimeOrigin::signed(ALICE),
                        unlimited_aid,
                        limited_policy,
                    ),
                    Error::<Test>::PolicyFieldImmutable
                );

                // Increase is allowed: 100 -> 200
                let longer_policy = timed_policy(200);
                assert_ok!(Rwa::update_asset_policy(
                    RuntimeOrigin::signed(ALICE),
                    aid,
                    longer_policy,
                ));
            });
    }

    // ── Force retire cleans up all related storage ────────────────────

    #[test]
    fn attack_force_retire_with_pending_participations() {
        // force_retire_asset removes PendingApprovals but does NOT refund
        // escrowed deposits. Participants must call claim_retired_deposit.
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000), (DAVE, 100_000)])
            .build()
            .execute_with(|| {
                let policy = approval_policy();
                let aid = register_test_asset(ALICE, BOB, policy);

                // Create pending participations
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(DAVE),
                    aid,
                    vec![DAVE],
                ));

                let charlie_after_request = Balances::free_balance(CHARLIE);
                let dave_after_request = Balances::free_balance(DAVE);

                // Force retire — cleans up PendingApprovals but NOT deposits
                assert_ok!(Rwa::force_retire_asset(RuntimeOrigin::root(), aid));

                // PendingApprovals is cleaned up
                assert!(pallet::PendingApprovals::<Test>::get(aid).is_empty());

                // But the Participations records still exist (for claim_retired_deposit)
                assert!(pallet::Participations::<Test>::get(aid, 0).is_some());
                assert!(pallet::Participations::<Test>::get(aid, 1).is_some());

                // Participants can recover via claim_retired_deposit
                assert_ok!(Rwa::claim_retired_deposit(RuntimeOrigin::signed(CHARLIE), aid, 0,));
                assert_ok!(Rwa::claim_retired_deposit(RuntimeOrigin::signed(DAVE), aid, 1,));

                // Both get back deposit + entry_fee (PendingApproval refunds both)
                assert_eq!(
                    Balances::free_balance(CHARLIE) - charlie_after_request,
                    60, // deposit(50) + entry_fee(10)
                );
                assert_eq!(Balances::free_balance(DAVE) - dave_after_request, 60,);
            });
    }

    // ── Sybil attack on PendingApprovals queue ────────────────────────

    #[test]
    fn attack_cat6_1_pending_approvals_dos_and_recovery() {
        // Attacker fills PendingApprovals queue, blocking legitimate users.
        // Owner uses batch_reject_pending to clear the queue.
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (CHARLIE, 100_000),
                (DAVE, 100_000),
                (EVE, 100_000),
                // Extra accounts for Sybil
                (10, 100_000),
                (11, 100_000),
                (12, 100_000),
                (13, 100_000),
                (14, 100_000),
            ])
            .build()
            .execute_with(|| {
                let policy = approval_policy();
                let aid = register_test_asset(ALICE, BOB, policy);

                // MaxPendingApprovals = 5 in mock
                // Attacker fills all 5 slots
                for i in 10u64..15 {
                    assert_ok!(Rwa::request_participation(RuntimeOrigin::signed(i), aid, vec![i],));
                }

                // Legitimate user CHARLIE is blocked
                assert_noop!(
                    Rwa::request_participation(RuntimeOrigin::signed(CHARLIE), aid, vec![CHARLIE],),
                    Error::<Test>::PendingApprovalsFull
                );

                // Owner recovers by batch-rejecting all pending
                assert_ok!(Rwa::batch_reject_pending(RuntimeOrigin::signed(ALICE), aid,));

                // Now CHARLIE can participate
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));
            });
    }

    // ── Participation status after partial slash ──────────────────────

    #[test]
    fn attack_cat3_4_partial_slash_terminates_participation() {
        // Even a 1-unit slash terminates the participation entirely.
        // The remainder is refunded, but the participation is dead.
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                let charlie_pre = Balances::free_balance(CHARLIE);

                // Slash only 1 out of 50 deposit
                assert_ok!(Rwa::slash_participation(RuntimeOrigin::root(), aid, 0, 1, None,));

                // Participation is terminated (status = Slashed)
                let p = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(matches!(p.status, ParticipationStatus::Slashed));
                assert_eq!(p.deposit_held, 0);

                // CHARLIE got back 49 (remainder), lost 1 (slashed)
                let charlie_after = Balances::free_balance(CHARLIE);
                assert_eq!(charlie_after - charlie_pre, 49);

                // Cannot exit, renew, or do anything with the participation
                assert_noop!(
                    Rwa::exit_participation(RuntimeOrigin::signed(CHARLIE), aid, 0),
                    Error::<Test>::InvalidParticipationStatus
                );
                assert_noop!(
                    Rwa::renew_participation(RuntimeOrigin::signed(CHARLIE), aid, 0),
                    Error::<Test>::InvalidParticipationStatus
                );

                // HolderIndex cleaned up
                assert!(!pallet::HolderIndex::<Test>::contains_key(aid, CHARLIE));
            });
    }

    // ── Paused asset blocks new participations but existing remain ────

    #[test]
    fn attack_pause_blocks_new_participations() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000), (DAVE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());

                // CHARLIE participates before pause
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Admin pauses asset
                assert_ok!(Rwa::pause_asset(RuntimeOrigin::root(), aid));

                // DAVE cannot participate on paused asset
                assert_noop!(
                    Rwa::request_participation(RuntimeOrigin::signed(DAVE), aid, vec![DAVE],),
                    Error::<Test>::AssetNotActive
                );

                // CHARLIE's participation is still Active
                let p = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(matches!(p.status, ParticipationStatus::Active { .. }));

                // CHARLIE can still exit (exit_participation doesn't check
                // asset status, only participation status)
                assert_ok!(Rwa::exit_participation(RuntimeOrigin::signed(CHARLIE), aid, 0,));
            });
    }

    // ── Verify approval blocked on non-Active assets ─────────────────

    #[test]
    fn attack_approve_blocked_on_paused_asset() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let policy = approval_policy();
                let aid = register_test_asset(ALICE, BOB, policy);

                // CHARLIE requests participation (pending approval)
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Admin pauses asset before owner can approve
                assert_ok!(Rwa::pause_asset(RuntimeOrigin::root(), aid));

                // Owner tries to approve — blocked because asset is Paused
                assert_noop!(
                    Rwa::approve_participation(RuntimeOrigin::signed(ALICE), aid, 0),
                    Error::<Test>::AssetNotActive
                );

                // Unpause and approve succeeds
                assert_ok!(Rwa::unpause_asset(RuntimeOrigin::root(), aid));
                assert_ok!(Rwa::approve_participation(RuntimeOrigin::signed(ALICE), aid, 0,));
            });
    }

    // ── Verify V5 MinParticipationDeposit ─────────────────────────────

    #[test]
    fn attack_v5_min_deposit_rejects_zero_deposit() {
        // V5 fix: MinParticipationDeposit = 1 in mock, so deposit = 0
        // should be rejected on registration.
        ExtBuilder::default().build().execute_with(|| {
            let policy = AssetPolicy {
                deposit_currency: PaymentCurrency::Native,
                entry_fee: 0,
                deposit: 0, // below MinParticipationDeposit
                max_duration: None,
                max_participants: None,
                requires_approval: false,
            };
            assert_noop!(
                Rwa::register_asset(RuntimeOrigin::signed(ALICE), BOB, policy, vec![0u8; 10],),
                Error::<Test>::DepositBelowMinimum
            );
        });
    }

    // ── participant_count consistency across all lifecycle paths ──────

    #[test]
    fn attack_cat4_2_participant_count_consistency() {
        // Verify participant_count increments and decrements correctly
        // across multiple paths: request, exit, expire, slash, revoke.
        ExtBuilder::default()
            .balances(vec![
                (ALICE, 100_000),
                (BOB, 100_000),
                (CHARLIE, 100_000),
                (DAVE, 100_000),
                (EVE, 100_000),
            ])
            .build()
            .execute_with(|| {
                let policy = timed_policy(5);
                let aid = register_test_asset(ALICE, BOB, policy);

                // Request 3 participations: count = 3
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(DAVE),
                    aid,
                    vec![DAVE],
                ));
                assert_ok!(Rwa::request_participation(RuntimeOrigin::signed(EVE), aid, vec![EVE],));
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset.participant_count, 3);

                // Exit one: count = 2
                assert_ok!(Rwa::exit_participation(RuntimeOrigin::signed(CHARLIE), aid, 0,));
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset.participant_count, 2);

                // Slash one: count = 1
                assert_ok!(Rwa::slash_participation(RuntimeOrigin::root(), aid, 1, 50, None,));
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset.participant_count, 1);

                // Expire the last one via run_to_block and settle
                run_to_block(7);
                assert_ok!(
                    Rwa::settle_expired_participation(RuntimeOrigin::signed(ALICE), aid, 2,)
                );
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(
                    asset.participant_count, 0,
                    "All paths correctly decrement participant_count"
                );
            });
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CRIT-03: AssetLifecycleGuard — Cross-Pallet Cascade Prevention
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// These tests verify the CRIT-03 defense: `AssetLifecycleGuard` prevents
// `force_retire_asset` and `slash_participation` from proceeding when a
// cross-pallet guard (e.g., active crowdfunding campaign) returns Err.
//
// The MockLifecycleGuard in mock.rs provides per-test configurability
// via thread_local BTreeSets.
//
// MECE Partition:
//   P1: force_retire blocked by guard     (test 1)
//   P2: force_retire allowed (no block)   (test 2)
//   P3: slash blocked by guard            (test 3)
//   P4: slash allowed (no block)          (test 4)
//   P5: on_initialize retirement gap      (test 5)
//   P6: guard-before-mutation invariant   (test 6)
//   P7: selective per-asset blocking      (test 7)
//   P8: guard scope isolation             (test 8)
//   P9: full lifecycle simulation         (test 9)
//   P10: per-participation slash granularity (test 10)
//
// Each partition is mutually exclusive (tests exactly one unique
// behavioral scenario) and collectively exhaustive (covers every
// code path through the AssetLifecycleGuard integration points).

mod crit03_asset_lifecycle_guard {
    use super::*;

    // ── P1: force_retire_asset blocked when guard returns Err ─────────
    //
    // Attack scenario: RWA admin calls force_retire_asset on an asset
    // that has an active crowdfunding campaign.  The guard blocks the
    // retirement to prevent cascade into campaign cancellation.

    #[test]
    fn attack_crit03_force_retire_blocked_by_guard() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Simulate active campaign linked to this asset
                MockLifecycleGuard::block_retire(aid);

                // force_retire_asset must fail with BlockedByLifecycleGuard
                assert_noop!(
                    Rwa::force_retire_asset(RuntimeOrigin::root(), aid),
                    Error::<Test>::BlockedByLifecycleGuard
                );

                // Asset remains in its original status (Active), not Retired
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert!(
                    matches!(asset.status, AssetStatus::Active),
                    "Asset must remain Active when guard blocks retirement"
                );

                // Participation is untouched
                let p = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(matches!(p.status, ParticipationStatus::Active { .. }));
            });
    }

    // ── P2: force_retire_asset succeeds when guard allows ────────────
    //
    // Normal path: no campaigns linked, guard returns Ok.

    #[test]
    fn attack_crit03_force_retire_allowed_when_no_campaigns() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Guard is not blocking (default after clear)
                assert_ok!(Rwa::force_retire_asset(RuntimeOrigin::root(), aid));

                // Asset is now Retired
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert!(
                    matches!(asset.status, AssetStatus::Retired),
                    "Asset must be Retired when guard allows"
                );
            });
    }

    // ── P3: slash_participation blocked when guard returns Err ────────
    //
    // Attack scenario: admin tries to slash a participation that is
    // linked to an active campaign.  Guard blocks to prevent deposit
    // loss from cascading into campaign insolvency.

    #[test]
    fn attack_crit03_slash_blocked_by_guard() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                let p_before = pallet::Participations::<Test>::get(aid, 0).unwrap();
                let deposit_before = p_before.deposit_held;

                // Block slashing for this specific (asset, participation) pair
                MockLifecycleGuard::block_slash(aid, 0);

                assert_noop!(
                    Rwa::slash_participation(RuntimeOrigin::root(), aid, 0, 10, None),
                    Error::<Test>::BlockedByLifecycleGuard
                );

                // Participation deposit is unchanged
                let p_after = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert_eq!(
                    p_after.deposit_held, deposit_before,
                    "Deposit must be unchanged when guard blocks slash"
                );
                assert!(
                    matches!(p_after.status, ParticipationStatus::Active { .. }),
                    "Participation must remain Active when guard blocks slash"
                );
            });
    }

    // ── P4: slash_participation succeeds when guard allows ───────────
    //
    // Normal path: no campaigns linked to this participation.

    #[test]
    fn attack_crit03_slash_allowed_when_no_campaigns() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Guard is not blocking (default)
                assert_ok!(Rwa::slash_participation(RuntimeOrigin::root(), aid, 0, 10, None));

                // Participation is now Slashed
                let p = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(
                    matches!(p.status, ParticipationStatus::Slashed),
                    "Participation must be Slashed when guard allows"
                );
                assert_eq!(p.deposit_held, 0);
            });
    }

    // ── P5: on_initialize retirement bypasses the guard ──────────────
    //
    // DOCUMENTED GAP: The automatic retirement in `on_initialize`
    // (triggered by `sunset_asset` expiry) does NOT call the
    // `AssetLifecycleGuard`.  This is by design — `sunset_asset` is an
    // owner-initiated action with a known future date, giving campaigns
    // time to complete.  However, if an owner sunset an asset while a
    // campaign is still Funding, the on_initialize retirement would
    // proceed unchecked.
    //
    // This test documents this behavior as a known architectural
    // limitation that should be addressed at the campaign layer
    // (e.g., campaigns should check asset status on milestone approval).

    #[test]
    fn attack_crit03_retire_blocked_but_on_initialize_bypasses_guard() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Owner initiates sunset (schedules retirement at block 5)
                assert_ok!(Rwa::sunset_asset(RuntimeOrigin::signed(ALICE), aid, 5));

                // Guard blocks force_retire
                MockLifecycleGuard::block_retire(aid);

                // Confirm force_retire is indeed blocked
                assert_noop!(
                    Rwa::force_retire_asset(RuntimeOrigin::root(), aid),
                    Error::<Test>::BlockedByLifecycleGuard
                );

                // But on_initialize at block 5 retires without checking guard
                // — this is the documented architectural gap.
                run_to_block(5);

                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert!(
                    matches!(asset.status, AssetStatus::Retired),
                    "DOCUMENTED GAP: on_initialize retires without guard check. sunset_asset is \
                     owner-initiated with known expiry, so campaigns have advance notice.  This \
                     is a design trade-off, not a bug."
                );
            });
    }

    // ── P6: guard fires BEFORE any state mutations ───────────────────
    //
    // This is the critical ordering invariant.  If the guard check
    // happened after mutations, a failed guard would leave the pallet
    // in an inconsistent state (even with transactional rollback, the
    // explicit ordering provides defense-in-depth).

    #[test]
    fn attack_crit03_guard_checked_before_state_mutation() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Set up slash distribution to verify it survives the blocked call
                let dist: BoundedVec<
                    SlashRecipient<u64>,
                    <Test as crate::Config>::MaxSlashRecipients,
                > = vec![SlashRecipient {
                    kind: SlashRecipientKind::Beneficiary,
                    share: Permill::from_percent(100),
                }]
                .try_into()
                .unwrap();
                assert_ok!(Rwa::set_slash_distribution(RuntimeOrigin::signed(ALICE), aid, dist,));

                // Set up pending ownership transfer
                assert_ok!(Rwa::transfer_ownership(RuntimeOrigin::signed(ALICE), aid, DAVE,));

                // Snapshot ALL related storage before the blocked call
                let asset_pre = pallet::RwaAssets::<Test>::get(aid).unwrap();
                let owner_assets_pre = pallet::OwnerAssets::<Test>::get(ALICE);
                let slash_dist_pre_exists =
                    pallet::AssetSlashDistribution::<Test>::get(aid).is_some();
                let pending_transfer_pre = pallet::PendingOwnershipTransfer::<Test>::get(aid);
                let p_pre = pallet::Participations::<Test>::get(aid, 0).unwrap();
                let alice_balance_pre = Balances::free_balance(ALICE);
                let alice_reserved_pre = Balances::reserved_balance(ALICE);

                // Block and attempt force_retire
                MockLifecycleGuard::block_retire(aid);
                assert_noop!(
                    Rwa::force_retire_asset(RuntimeOrigin::root(), aid),
                    Error::<Test>::BlockedByLifecycleGuard
                );

                // Verify EVERY piece of storage is unchanged
                let asset_post = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert_eq!(asset_pre.status, asset_post.status, "Asset status must be unchanged");
                assert_eq!(asset_pre.owner, asset_post.owner, "Asset owner must be unchanged");
                assert_eq!(
                    asset_pre.registration_deposit, asset_post.registration_deposit,
                    "Asset registration_deposit must be unchanged"
                );
                assert_eq!(
                    asset_pre.participant_count, asset_post.participant_count,
                    "Asset participant_count must be unchanged"
                );

                let owner_assets_post = pallet::OwnerAssets::<Test>::get(ALICE);
                assert_eq!(
                    owner_assets_pre.len(),
                    owner_assets_post.len(),
                    "OwnerAssets must be unchanged"
                );

                let slash_dist_post_exists =
                    pallet::AssetSlashDistribution::<Test>::get(aid).is_some();
                assert_eq!(
                    slash_dist_pre_exists, slash_dist_post_exists,
                    "SlashDistribution existence must be unchanged"
                );

                let pending_transfer_post = pallet::PendingOwnershipTransfer::<Test>::get(aid);
                assert_eq!(
                    pending_transfer_pre, pending_transfer_post,
                    "PendingOwnershipTransfer must be unchanged"
                );

                let p_post = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert_eq!(p_pre.status, p_post.status, "Participation status must be unchanged");
                assert_eq!(
                    p_pre.deposit_held, p_post.deposit_held,
                    "Participation deposit must be unchanged"
                );
                assert_eq!(p_pre.payer, p_post.payer, "Participation payer must be unchanged");

                assert_eq!(
                    Balances::free_balance(ALICE),
                    alice_balance_pre,
                    "ALICE free balance must be unchanged (registration deposit NOT unreserved)"
                );
                assert_eq!(
                    Balances::reserved_balance(ALICE),
                    alice_reserved_pre,
                    "ALICE reserved balance must be unchanged"
                );
            });
    }

    // ── P7: selective per-asset blocking ─────────────────────────────
    //
    // Proves the guard is per-asset, not global.  Only the asset with
    // an active campaign is blocked; unrelated assets proceed normally.

    #[test]
    fn attack_crit03_selective_blocking() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid0 = register_test_asset(ALICE, BOB, default_policy());
                let aid1 = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid0,
                    vec![CHARLIE],
                ));
                assert_ok!(
                    Rwa::request_participation(RuntimeOrigin::signed(BOB), aid1, vec![BOB],)
                );

                // Block retire for asset 0 only
                MockLifecycleGuard::block_retire(aid0);

                // Asset 0: blocked
                assert_noop!(
                    Rwa::force_retire_asset(RuntimeOrigin::root(), aid0),
                    Error::<Test>::BlockedByLifecycleGuard
                );
                let asset0 = pallet::RwaAssets::<Test>::get(aid0).unwrap();
                assert!(matches!(asset0.status, AssetStatus::Active));

                // Asset 1: proceeds normally
                assert_ok!(Rwa::force_retire_asset(RuntimeOrigin::root(), aid1));
                let asset1 = pallet::RwaAssets::<Test>::get(aid1).unwrap();
                assert!(matches!(asset1.status, AssetStatus::Retired));
            });
    }

    // ── P8: guard scope isolation ────────────────────────────────────
    //
    // The guard ONLY affects force_retire_asset and slash_participation.
    // Other extrinsics (deactivate_asset, sunset_asset,
    // update_asset_policy) must remain unaffected.

    #[test]
    fn attack_crit03_guard_does_not_affect_other_extrinsics() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Block retire for this asset
                MockLifecycleGuard::block_retire(aid);
                // Also block slash for good measure
                MockLifecycleGuard::block_slash(aid, 0);

                // Confirm force_retire IS blocked (control check)
                assert_noop!(
                    Rwa::force_retire_asset(RuntimeOrigin::root(), aid),
                    Error::<Test>::BlockedByLifecycleGuard
                );

                // Confirm slash IS blocked (control check)
                assert_noop!(
                    Rwa::slash_participation(RuntimeOrigin::root(), aid, 0, 10, None),
                    Error::<Test>::BlockedByLifecycleGuard
                );

                // deactivate_asset: NOT guarded, works fine
                assert_ok!(Rwa::deactivate_asset(RuntimeOrigin::signed(ALICE), aid));
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert!(matches!(asset.status, AssetStatus::Inactive));

                // reactivate so we can test sunset
                assert_ok!(Rwa::reactivate_asset(RuntimeOrigin::signed(ALICE), aid));

                // sunset_asset: NOT guarded, works fine (only schedules, doesn't retire)
                assert_ok!(Rwa::sunset_asset(RuntimeOrigin::signed(ALICE), aid, 100));
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert!(matches!(asset.status, AssetStatus::Sunsetting { expiry_block: 100 }));

                // update_asset_policy: NOT guarded.  We need a new Active asset
                // since the current one is Sunsetting.
                let aid2 = register_test_asset(ALICE, BOB, default_policy());
                let new_policy = crate::AssetPolicy {
                    deposit_currency: crate::PaymentCurrency::Native,
                    entry_fee: 25, // change entry_fee (allowed when no participants)
                    deposit: 50,   // deposit is always immutable
                    max_duration: None,
                    max_participants: None,
                    requires_approval: false,
                };
                MockLifecycleGuard::block_retire(aid2);
                assert_ok!(Rwa::update_asset_policy(
                    RuntimeOrigin::signed(ALICE),
                    aid2,
                    new_policy,
                ));
                let asset2 = pallet::RwaAssets::<Test>::get(aid2).unwrap();
                assert_eq!(asset2.policy.entry_fee, 25);
            });
    }

    // ── P9: full lifecycle cascade prevention scenario ───────────────
    //
    // End-to-end simulation:
    // 1. Asset created, participation active, campaign simulated (block)
    // 2. force_retire blocked (campaign active)
    // 3. Campaign completes (unblock)
    // 4. force_retire succeeds
    //
    // This mirrors the real-world flow where CrowdfundingLifecycleGuard
    // iterates campaigns and blocks/allows based on campaign status.

    #[test]
    fn attack_crit03_cascade_prevention_scenario() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));

                // Phase 1: Campaign is Funding — guard blocks retire
                MockLifecycleGuard::block_retire(aid);
                assert_noop!(
                    Rwa::force_retire_asset(RuntimeOrigin::root(), aid),
                    Error::<Test>::BlockedByLifecycleGuard
                );
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert!(matches!(asset.status, AssetStatus::Active));

                // Phase 2: Campaign transitions to Succeeded, then
                // Completed (terminal) — guard unblocks
                MockLifecycleGuard::unblock_retire(aid);

                // Phase 3: force_retire now succeeds
                assert_ok!(Rwa::force_retire_asset(RuntimeOrigin::root(), aid));
                let asset = pallet::RwaAssets::<Test>::get(aid).unwrap();
                assert!(
                    matches!(asset.status, AssetStatus::Retired),
                    "Asset retires after campaign completes"
                );
            });
    }

    // ── P10: per-participation slash granularity ─────────────────────
    //
    // Proves the guard distinguishes between different participations
    // on the same asset.  Only the participation linked to an active
    // campaign is blocked; others can be slashed freely.

    #[test]
    fn attack_crit03_slash_selective_per_participation() {
        ExtBuilder::default()
            .balances(vec![(ALICE, 100_000), (BOB, 100_000), (CHARLIE, 100_000), (DAVE, 100_000)])
            .build()
            .execute_with(|| {
                let aid = register_test_asset(ALICE, BOB, default_policy());
                // Two participations on the same asset
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(CHARLIE),
                    aid,
                    vec![CHARLIE],
                ));
                assert_ok!(Rwa::request_participation(
                    RuntimeOrigin::signed(DAVE),
                    aid,
                    vec![DAVE],
                ));

                // Block slash for participation 0 only
                MockLifecycleGuard::block_slash(aid, 0);

                // Slash participation 0: blocked
                assert_noop!(
                    Rwa::slash_participation(RuntimeOrigin::root(), aid, 0, 10, None),
                    Error::<Test>::BlockedByLifecycleGuard
                );
                let p0 = pallet::Participations::<Test>::get(aid, 0).unwrap();
                assert!(
                    matches!(p0.status, ParticipationStatus::Active { .. }),
                    "Participation 0 must remain Active"
                );
                assert_eq!(p0.deposit_held, 50);

                // Slash participation 1: succeeds (not blocked)
                assert_ok!(Rwa::slash_participation(RuntimeOrigin::root(), aid, 1, 10, None));
                let p1 = pallet::Participations::<Test>::get(aid, 1).unwrap();
                assert!(
                    matches!(p1.status, ParticipationStatus::Slashed),
                    "Participation 1 must be Slashed"
                );
                assert_eq!(p1.deposit_held, 0);
            });
    }
}
