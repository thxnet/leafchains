//! Benchmarking setup for pallet-trustless-agent

use frame_benchmarking::{account, benchmarks, vec, whitelisted_caller};
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use sp_runtime::traits::{Bounded, Hash};

use super::*;
#[allow(unused)]
use crate::Pallet as TrustlessAgent;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

benchmarks! {
    where_clause { where T: Config }

    register_agent {
        let caller: T::AccountId = whitelisted_caller();
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
    }: _(RawOrigin::Signed(caller.clone()), registration_uri, metadata)
    verify {
        assert!(Agents::<T>::contains_key(0u64));
    }

    update_metadata {
        let caller: T::AccountId = whitelisted_caller();
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let key = vec![1u8; 64];
        let value = Some(vec![1u8; 64]);
    }: _(RawOrigin::Signed(caller), agent_id, key.clone(), value)

    transfer_agent {
        let caller: T::AccountId = whitelisted_caller();
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(caller.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let new_owner: T::AccountId = account("new_owner", 0, 0);
        let _ = T::Currency::make_free_balance_be(&new_owner, BalanceOf::<T>::max_value());
    }: _(RawOrigin::Signed(caller), agent_id, new_owner.clone())
    verify {
        let agent = Agents::<T>::get(agent_id).unwrap();
        assert_eq!(agent.owner, new_owner);
    }

    give_feedback {
        let caller: T::AccountId = whitelisted_caller();
        let agent_owner: T::AccountId = account("agent_owner", 0, 0);
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        // Authorize the client to give feedback
        let _ = Pallet::<T>::authorize_feedback(
            RawOrigin::Signed(agent_owner).into(),
            agent_id,
            caller.clone(),
            100u32,
            1000u32.into(),
        );
        let file_uri = vec![1u8; 100];
        let content_hash = T::Hashing::hash_of(&file_uri);
        let tags = vec![vec![1u8; 10]];
    }: _(RawOrigin::Signed(caller), agent_id, 100u8, tags, file_uri, content_hash)
    verify {
        assert!(Feedbacks::<T>::contains_key(0u64));
    }

    revoke_feedback {
        let caller: T::AccountId = whitelisted_caller();
        let agent_owner: T::AccountId = account("agent_owner", 0, 0);
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        // Authorize the client to give feedback
        let _ = Pallet::<T>::authorize_feedback(
            RawOrigin::Signed(agent_owner).into(),
            agent_id,
            caller.clone(),
            100u32,
            1000u32.into(),
        );
        let file_uri = vec![1u8; 100];
        let content_hash = T::Hashing::hash_of(&file_uri);
        let tags = vec![vec![1u8; 10]];
        let _ = Pallet::<T>::give_feedback(
            RawOrigin::Signed(caller.clone()).into(),
            agent_id,
            50u8,
            tags,
            file_uri,
            content_hash,
        );
        let feedback_id = 0u64;
    }: _(RawOrigin::Signed(caller), feedback_id)
    verify {
        let feedback = Feedbacks::<T>::get(feedback_id).unwrap();
        assert!(feedback.revoked);
    }

    append_response {
        let agent_owner: T::AccountId = whitelisted_caller();
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let client: T::AccountId = account("client", 0, 0);
        let _ = T::Currency::make_free_balance_be(&client, BalanceOf::<T>::max_value());
        // Authorize the client to give feedback
        let _ = Pallet::<T>::authorize_feedback(
            RawOrigin::Signed(agent_owner.clone()).into(),
            agent_id,
            client.clone(),
            100u32,
            1000u32.into(),
        );
        let file_uri = vec![1u8; 100];
        let content_hash = T::Hashing::hash_of(&file_uri);
        let tags = vec![vec![1u8; 10]];
        let _ = Pallet::<T>::give_feedback(
            RawOrigin::Signed(client).into(),
            agent_id,
            50u8,
            tags,
            file_uri.clone(),
            content_hash,
        );
        let feedback_id = 0u64;
        let response_uri = vec![1u8; 100];
        let response_hash = T::Hashing::hash_of(&response_uri);
    }: _(RawOrigin::Signed(agent_owner), feedback_id, response_uri, response_hash)
    verify {
        assert_eq!(FeedbackResponses::<T>::get(feedback_id).unwrap().len(), 1);
    }

    register_validator {
        let caller: T::AccountId = whitelisted_caller();
        let stake = T::ValidatorMinStake::get();
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
    }: _(RawOrigin::Signed(caller.clone()), stake)
    verify {
        assert!(Validators::<T>::contains_key(caller));
    }

    unregister_validator {
        let caller: T::AccountId = whitelisted_caller();
        let stake = T::ValidatorMinStake::get();
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_validator(
            RawOrigin::Signed(caller.clone()).into(),
            stake,
        );
    }: _(RawOrigin::Signed(caller.clone()))
    verify {
        assert!(!Validators::<T>::contains_key(caller));
    }

    request_validation {
        let caller: T::AccountId = whitelisted_caller();
        let agent_owner: T::AccountId = account("agent_owner", 0, 0);
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let reward = T::ValidatorMinStake::get();
    }: _(RawOrigin::Signed(caller), agent_id, reward)
    verify {
        assert!(ValidationRequests::<T>::contains_key(0u64));
    }

    submit_validation {
        let validator: T::AccountId = whitelisted_caller();
        let stake = T::ValidatorMinStake::get();
        let _ = T::Currency::make_free_balance_be(&validator, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_validator(
            RawOrigin::Signed(validator.clone()).into(),
            stake,
        );
        let requester: T::AccountId = account("requester", 0, 0);
        let agent_owner: T::AccountId = account("agent_owner", 0, 0);
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let _ = T::Currency::make_free_balance_be(&requester, BalanceOf::<T>::max_value());
        let reward = T::ValidatorMinStake::get();
        let _ = Pallet::<T>::request_validation(
            RawOrigin::Signed(requester).into(),
            agent_id,
            reward,
        );
        let request_id = 0u64;
        let evidence_uri = vec![1u8; 100];
        let content_hash = T::Hashing::hash_of(&evidence_uri);
        let validation_tags = vec![vec![1u8; 10]];
    }: _(RawOrigin::Signed(validator.clone()), request_id, 100u8, evidence_uri, content_hash, validation_tags)
    verify {
        assert!(ValidationResponses::<T>::contains_key(request_id, &validator));
    }

    create_escrow {
        let caller: T::AccountId = whitelisted_caller();
        let agent_owner: T::AccountId = account("agent_owner", 0, 0);
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let amount = T::ValidatorMinStake::get();
        let timeout = 100u32.into();
    }: _(RawOrigin::Signed(caller), agent_id, amount, timeout, None)
    verify {
        assert!(Escrows::<T>::contains_key(0u64));
    }

    claim_escrow {
        let client: T::AccountId = account("client", 0, 0);
        let agent_owner: T::AccountId = whitelisted_caller();
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let initial_balance = BalanceOf::<T>::max_value() / 2u32.into();
        let _ = T::Currency::make_free_balance_be(&agent_owner, initial_balance);
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let _ = T::Currency::make_free_balance_be(&client, BalanceOf::<T>::max_value());
        let amount = T::ValidatorMinStake::get();
        let timeout = 200u32.into();
        // Use custom auto-complete time for benchmark
        let custom_auto_complete = Some(100u32.into());
        let _ = Pallet::<T>::create_escrow(
            RawOrigin::Signed(client).into(),
            agent_id,
            amount,
            timeout,
            custom_auto_complete,
        );
        let escrow_id = 0u64;
        // Fast forward past auto-complete time
        frame_system::Pallet::<T>::set_block_number(150u32.into());
    }: _(RawOrigin::Signed(agent_owner), escrow_id)
    verify {
        assert!(!Escrows::<T>::contains_key(escrow_id));
    }

    cancel_escrow {
        let caller: T::AccountId = whitelisted_caller();
        let agent_owner: T::AccountId = account("agent_owner", 0, 0);
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let amount = T::ValidatorMinStake::get();
        let timeout = 1u32.into();
        let _ = Pallet::<T>::create_escrow(
            RawOrigin::Signed(caller.clone()).into(),
            agent_id,
            amount,
            timeout,
            None,
        );
        let escrow_id = 0u64;
        // Fast forward past timeout
        frame_system::Pallet::<T>::set_block_number(100u32.into());
    }: _(RawOrigin::Signed(caller), escrow_id)
    verify {
        assert!(!Escrows::<T>::contains_key(escrow_id));
    }

    cancel_validation_request {
        let caller: T::AccountId = whitelisted_caller();
        let agent_owner: T::AccountId = account("agent_owner", 0, 0);
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let _ = T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let reward = T::ValidatorMinStake::get();
        let _ = Pallet::<T>::request_validation(
            RawOrigin::Signed(caller.clone()).into(),
            agent_id,
            reward,
        );
        let request_id = 0u64;
        // Fast forward past deadline
        frame_system::Pallet::<T>::set_block_number(
            frame_system::Pallet::<T>::block_number() + T::ValidationDeadline::get() + 1u32.into()
        );
    }: _(RawOrigin::Signed(caller), request_id)
    verify {
        assert!(!ValidationRequests::<T>::contains_key(request_id));
    }

    dispute_feedback {
        let agent_owner: T::AccountId = whitelisted_caller();
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let client: T::AccountId = account("client", 0, 0);
        let _ = T::Currency::make_free_balance_be(&client, BalanceOf::<T>::max_value());
        // Authorize the client to give feedback
        let _ = Pallet::<T>::authorize_feedback(
            RawOrigin::Signed(agent_owner.clone()).into(),
            agent_id,
            client.clone(),
            100u32,
            1000u32.into(),
        );
        let file_uri = vec![1u8; 100];
        let content_hash = T::Hashing::hash_of(&file_uri);
        let tags = vec![vec![1u8; 10]];
        let _ = Pallet::<T>::give_feedback(
            RawOrigin::Signed(client).into(),
            agent_id,
            50u8,
            tags,
            file_uri,
            content_hash,
        );
        let feedback_id = 0u64;
        let reason_uri = vec![1u8; 100];
        let reason_hash = T::Hashing::hash_of(&reason_uri);
    }: _(RawOrigin::Signed(agent_owner), feedback_id, reason_uri, reason_hash)
    verify {
        assert!(Disputes::<T>::contains_key(0u64));
    }

    resolve_dispute {
        let agent_owner: T::AccountId = account("agent_owner", 0, 0);
        let registration_uri = vec![1u8; 100];
        let metadata = vec![];
        let _ = T::Currency::make_free_balance_be(&agent_owner, BalanceOf::<T>::max_value());
        let _ = Pallet::<T>::register_agent(
            RawOrigin::Signed(agent_owner.clone()).into(),
            registration_uri,
            metadata,
        );
        let agent_id = 0u64;
        let client: T::AccountId = account("client", 0, 0);
        let _ = T::Currency::make_free_balance_be(&client, BalanceOf::<T>::max_value());
        // Authorize the client to give feedback
        let _ = Pallet::<T>::authorize_feedback(
            RawOrigin::Signed(agent_owner.clone()).into(),
            agent_id,
            client.clone(),
            100u32,
            1000u32.into(),
        );
        let file_uri = vec![1u8; 100];
        let content_hash = T::Hashing::hash_of(&file_uri);
        let tags = vec![vec![1u8; 10]];
        let _ = Pallet::<T>::give_feedback(
            RawOrigin::Signed(client).into(),
            agent_id,
            50u8,
            tags,
            file_uri,
            content_hash,
        );
        let feedback_id = 0u64;
        let reason_uri = vec![1u8; 100];
        let reason_hash = T::Hashing::hash_of(&reason_uri);
        let _ = Pallet::<T>::dispute_feedback(
            RawOrigin::Signed(agent_owner).into(),
            feedback_id,
            reason_uri,
            reason_hash,
        );
        let dispute_id = 0u64;
    }: _(RawOrigin::Root, dispute_id, DisputeStatus::ResolvedForDisputer)
    verify {
        let dispute = Disputes::<T>::get(dispute_id).unwrap();
        assert_eq!(dispute.status, DisputeStatus::ResolvedForDisputer);
    }

    add_validator_to_whitelist {
        let validator: T::AccountId = account("validator", 0, 0);
    }: _(RawOrigin::Root, validator.clone())
    verify {
        assert!(ValidatorWhitelist::<T>::get(validator));
    }

    remove_validator_from_whitelist {
        let validator: T::AccountId = account("validator", 0, 0);
        ValidatorWhitelist::<T>::insert(validator.clone(), true);
    }: _(RawOrigin::Root, validator.clone())
    verify {
        assert!(!ValidatorWhitelist::<T>::get(validator));
    }

    impl_benchmark_test_suite!(TrustlessAgent, crate::mock::new_test_ext(), crate::mock::Test);
}
