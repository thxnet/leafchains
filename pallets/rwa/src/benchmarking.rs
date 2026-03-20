#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks};
use frame_support::{assert_ok, pallet_prelude::Get, traits::Currency, BoundedVec};
use frame_system::RawOrigin;
use sp_runtime::{traits::Bounded, Permill, Saturating};
use sp_std::{vec, vec::Vec};

use super::*;
use crate::pallet::{self as pallet_rwa, BalanceOf};

const SEED: u32 = 0;

/// Ensure the pallet sub-account has enough existential deposit so that
/// KeepAlive transfers out of it succeed.
fn fund_pallet_account<T: Config>() {
    let pallet_acct = Pallet::<T>::pallet_account();
    let ed = T::NativeCurrency::minimum_balance();
    // Only fund if below threshold to avoid double-funding
    if T::NativeCurrency::free_balance(&pallet_acct) < ed.saturating_mul(10u32.into()) {
        T::NativeCurrency::make_free_balance_be(&pallet_acct, ed.saturating_mul(1_000u32.into()));
    }
}

fn create_funded_account<T: Config>(name: &'static str, idx: u32) -> T::AccountId {
    let acct: T::AccountId = account(name, idx, SEED);
    // Fund generously: must cover AssetRegistrationDeposit + participation deposits
    // + existential deposit. 1_000_000 * MinParticipationDeposit gives enough
    // headroom without overflowing total issuance.
    let amount = T::MinParticipationDeposit::get().saturating_mul(1_000_000u32.into());
    T::NativeCurrency::make_free_balance_be(&acct, amount);
    acct
}

fn default_policy<T: Config>() -> AssetPolicy<BalanceOf<T>, T::BlockNumber, T::AssetId> {
    // Use MinParticipationDeposit to ensure deposit is always valid for the
    // runtime.
    let min_deposit = T::MinParticipationDeposit::get();
    AssetPolicy {
        deposit_currency: PaymentCurrency::Native,
        entry_fee: BalanceOf::<T>::from(0u32),
        deposit: min_deposit,
        max_duration: None,
        max_participants: None,
        requires_approval: false,
    }
}

fn setup_asset<T: Config>(owner: &T::AccountId, beneficiary: &T::AccountId, m: u32) -> u32 {
    fund_pallet_account::<T>();
    let metadata = vec![0u8; m as usize];
    assert_ok!(Pallet::<T>::register_asset(
        RawOrigin::Signed(owner.clone()).into(),
        beneficiary.clone(),
        default_policy::<T>(),
        metadata,
    ));
    pallet_rwa::NextRwaAssetId::<T>::get().saturating_sub(1)
}

fn setup_participation<T: Config>(
    asset_id: u32,
    payer: &T::AccountId,
    holders: Vec<T::AccountId>,
) -> u32 {
    assert_ok!(Pallet::<T>::request_participation(
        RawOrigin::Signed(payer.clone()).into(),
        asset_id,
        holders,
    ));
    pallet_rwa::NextParticipationId::<T>::get(asset_id).saturating_sub(1)
}

benchmarks! {
    register_asset {
        let m in 0 .. T::MaxMetadataLen::get();
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let metadata = vec![0u8; m as usize];
    }: _(RawOrigin::Signed(caller), beneficiary, default_policy::<T>(), metadata)
    verify {
        assert_eq!(pallet_rwa::NextRwaAssetId::<T>::get(), 1);
    }

    update_asset_policy {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let new_policy = default_policy::<T>();
    }: _(RawOrigin::Signed(caller), aid, new_policy)

    deactivate_asset {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
    }: _(RawOrigin::Signed(caller), aid)

    reactivate_asset {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        assert_ok!(Pallet::<T>::deactivate_asset(
            RawOrigin::Signed(caller.clone()).into(),
            aid,
        ));
    }: _(RawOrigin::Signed(caller), aid)

    sunset_asset {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let expiry = frame_system::Pallet::<T>::block_number()
            + T::BlockNumber::from(100u32);
    }: _(RawOrigin::Signed(caller), aid, expiry)

    force_retire_asset {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
    }: _(RawOrigin::Root, aid)

    retire_asset {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let expiry = frame_system::Pallet::<T>::block_number()
            + T::BlockNumber::from(1u32);
        assert_ok!(Pallet::<T>::sunset_asset(
            RawOrigin::Signed(caller.clone()).into(),
            aid,
            expiry,
        ));
        frame_system::Pallet::<T>::set_block_number(expiry);
        let anyone = create_funded_account::<T>("anyone", 0);
    }: _(RawOrigin::Signed(anyone), aid)

    request_participation {
        let h in 1 .. T::MaxGroupSize::get();
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holders: Vec<T::AccountId> = (0..h)
            .map(|i| create_funded_account::<T>("holder", i))
            .collect();
    }: _(RawOrigin::Signed(payer), aid, holders)

    approve_participation {
        fund_pallet_account::<T>();
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let mut policy = default_policy::<T>();
        policy.requires_approval = true;
        let metadata = vec![0u8; 10];
        assert_ok!(Pallet::<T>::register_asset(
            RawOrigin::Signed(caller.clone()).into(),
            beneficiary.clone(),
            policy,
            metadata,
        ));
        let aid = pallet_rwa::NextRwaAssetId::<T>::get().saturating_sub(1);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder]);
    }: _(RawOrigin::Signed(caller), aid, 0)

    reject_participation {
        fund_pallet_account::<T>();
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let mut policy = default_policy::<T>();
        policy.requires_approval = true;
        let metadata = vec![0u8; 10];
        assert_ok!(Pallet::<T>::register_asset(
            RawOrigin::Signed(caller.clone()).into(),
            beneficiary.clone(),
            policy,
            metadata,
        ));
        let aid = pallet_rwa::NextRwaAssetId::<T>::get().saturating_sub(1);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder]);
    }: _(RawOrigin::Signed(caller), aid, 0)

    exit_participation {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder.clone()]);
    }: _(RawOrigin::Signed(payer), aid, 0)

    renew_participation {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder.clone()]);
    }: _(RawOrigin::Signed(payer), aid, 0)

    settle_expired_participation {
        fund_pallet_account::<T>();
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let mut policy = default_policy::<T>();
        policy.max_duration = Some(T::BlockNumber::from(1u32));
        let metadata = vec![0u8; 10];
        assert_ok!(Pallet::<T>::register_asset(
            RawOrigin::Signed(caller.clone()).into(),
            beneficiary.clone(),
            policy,
            metadata,
        ));
        let aid = pallet_rwa::NextRwaAssetId::<T>::get().saturating_sub(1);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder.clone()]);
        let expiry = frame_system::Pallet::<T>::block_number()
            + T::BlockNumber::from(2u32);
        frame_system::Pallet::<T>::set_block_number(expiry);
        let anyone = create_funded_account::<T>("anyone", 0);
    }: _(RawOrigin::Signed(anyone), aid, 0)

    claim_retired_deposit {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder.clone()]);
        assert_ok!(Pallet::<T>::force_retire_asset(RawOrigin::Root.into(), aid));
        let anyone = create_funded_account::<T>("anyone", 0);
    }: _(RawOrigin::Signed(anyone), aid, 0)

    add_holder {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder.clone()]);
        let new_holder = create_funded_account::<T>("new_holder", 0);
    }: _(RawOrigin::Signed(payer), aid, 0, new_holder)

    remove_holder {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holder1 = create_funded_account::<T>("holder1", 0);
        let holder2 = create_funded_account::<T>("holder2", 1);
        setup_participation::<T>(aid, &payer, vec![holder1.clone(), holder2.clone()]);
    }: _(RawOrigin::Signed(payer), aid, 0, holder2)

    leave_participation {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holder1 = create_funded_account::<T>("holder1", 0);
        let holder2 = create_funded_account::<T>("holder2", 1);
        setup_participation::<T>(aid, &payer, vec![holder1.clone(), holder2.clone()]);
    }: _(RawOrigin::Signed(holder2), aid, 0)

    set_slash_distribution {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let dist: BoundedVec<SlashRecipientOf<T>, T::MaxSlashRecipients> = vec![
            SlashRecipient {
                kind: SlashRecipientKind::Beneficiary,
                share: Permill::one(),
            },
        ].try_into().unwrap();
    }: _(RawOrigin::Signed(caller), aid, dist)

    slash_participation {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder.clone()]);
        // Slash half the deposit (must be <= deposit amount)
        let slash_amount = T::MinParticipationDeposit::get() / 2u32.into();
    }: _(RawOrigin::Root, aid, 0, slash_amount, None)

    revoke_participation {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder.clone()]);
    }: _(RawOrigin::Root, aid, 0)

    transfer_ownership {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let new_owner = create_funded_account::<T>("new_owner", 0);
    }: _(RawOrigin::Signed(caller), aid, new_owner)

    accept_ownership {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let new_owner = create_funded_account::<T>("new_owner", 0);
        assert_ok!(Pallet::<T>::transfer_ownership(
            RawOrigin::Signed(caller.clone()).into(),
            aid,
            new_owner.clone(),
        ));
    }: _(RawOrigin::Signed(new_owner), aid)

    cancel_ownership_transfer {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let new_owner = create_funded_account::<T>("new_owner", 0);
        assert_ok!(Pallet::<T>::transfer_ownership(
            RawOrigin::Signed(caller.clone()).into(),
            aid,
            new_owner,
        ));
    }: _(RawOrigin::Signed(caller), aid)

    update_beneficiary {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let new_beneficiary = create_funded_account::<T>("new_ben", 0);
    }: _(RawOrigin::Signed(caller), aid, new_beneficiary)

    update_metadata {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let new_metadata = vec![1u8; T::MaxMetadataLen::get() as usize];
    }: _(RawOrigin::Signed(caller), aid, new_metadata)

    transfer_participation {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        let payer = create_funded_account::<T>("payer", 0);
        let holder = create_funded_account::<T>("holder", 0);
        setup_participation::<T>(aid, &payer, vec![holder.clone()]);
        let new_payer = create_funded_account::<T>("new_payer", 0);
    }: _(RawOrigin::Signed(payer), aid, 0, new_payer)

    pause_asset {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
    }: _(RawOrigin::Root, aid)

    unpause_asset {
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let aid = setup_asset::<T>(&caller, &beneficiary, 10);
        assert_ok!(Pallet::<T>::pause_asset(RawOrigin::Root.into(), aid));
    }: _(RawOrigin::Root, aid)

    batch_reject_pending {
        let n in 1 .. T::MaxPendingApprovals::get();
        fund_pallet_account::<T>();
        let caller = create_funded_account::<T>("caller", 0);
        let beneficiary = create_funded_account::<T>("beneficiary", 0);
        let mut policy = default_policy::<T>();
        policy.requires_approval = true;
        let metadata = vec![0u8; 10];
        assert_ok!(Pallet::<T>::register_asset(
            RawOrigin::Signed(caller.clone()).into(),
            beneficiary.clone(),
            policy,
            metadata,
        ));
        let aid = pallet_rwa::NextRwaAssetId::<T>::get().saturating_sub(1);
        for i in 0..n {
            let payer = create_funded_account::<T>("payer", i);
            let holder = create_funded_account::<T>("batch_holder", i);
            setup_participation::<T>(aid, &payer, vec![holder]);
        }
        // Verify we have the expected number of pending approvals
        assert_eq!(pallet_rwa::PendingApprovals::<T>::get(aid).len() as u32, n);
    }: _(RawOrigin::Signed(caller), aid)
    verify {
        assert_eq!(pallet_rwa::PendingApprovals::<T>::get(aid).len(), 0);
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::ExtBuilder::default().build(), crate::mock::Test);
}
