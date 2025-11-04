use frame_support::{assert_noop, assert_ok};

use crate::{mock::*, DisputeStatus, Error, Escrows, Event};

#[test]
fn register_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        let uri = b"ipfs://QmTest123".to_vec();
        let metadata = vec![(b"name".to_vec(), b"TestAgent".to_vec())];

        assert_ok!(TrustlessAgent::register_agent(RuntimeOrigin::signed(1), uri.clone(), metadata));

        // Check event
        System::assert_last_event(
            Event::AgentRegistered { agent_id: 0, owner: 1, registration_uri: uri }.into(),
        );

        // Check storage
        assert!(TrustlessAgent::agents(0).is_some());
        assert_eq!(TrustlessAgent::agents(0).unwrap().owner, 1);
    });
}

#[test]
fn register_agent_requires_deposit() {
    new_test_ext().execute_with(|| {
        // Try to register with insufficient balance (account 4 doesn't exist)
        let uri = b"ipfs://QmTest123".to_vec();
        let metadata = vec![];

        assert_noop!(
            TrustlessAgent::register_agent(RuntimeOrigin::signed(4), uri, metadata),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn update_metadata_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        let uri = b"ipfs://QmTest123".to_vec();
        assert_ok!(TrustlessAgent::register_agent(RuntimeOrigin::signed(1), uri, vec![]));

        // Update metadata
        let key = b"name".to_vec();
        let value = b"UpdatedAgent".to_vec();
        assert_ok!(TrustlessAgent::update_metadata(
            RuntimeOrigin::signed(1),
            0,
            key.clone(),
            Some(value)
        ));

        // Check event
        System::assert_last_event(Event::AgentMetadataUpdated { agent_id: 0, key }.into());
    });
}

#[test]
fn update_metadata_requires_ownership() {
    new_test_ext().execute_with(|| {
        // Register an agent
        let uri = b"ipfs://QmTest123".to_vec();
        assert_ok!(TrustlessAgent::register_agent(RuntimeOrigin::signed(1), uri, vec![]));

        // Try to update metadata as non-owner
        let key = b"name".to_vec();
        let value = b"UpdatedAgent".to_vec();
        assert_noop!(
            TrustlessAgent::update_metadata(RuntimeOrigin::signed(2), 0, key, Some(value)),
            Error::<Test>::NotAgentOwner
        );
    });
}

#[test]
fn transfer_agent_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        let uri = b"ipfs://QmTest123".to_vec();
        assert_ok!(TrustlessAgent::register_agent(RuntimeOrigin::signed(1), uri, vec![]));

        // Transfer agent
        assert_ok!(TrustlessAgent::transfer_agent(RuntimeOrigin::signed(1), 0, 2));

        // Check event
        System::assert_last_event(
            Event::AgentTransferred { agent_id: 0, old_owner: 1, new_owner: 2 }.into(),
        );

        // Check storage
        assert_eq!(TrustlessAgent::agents(0).unwrap().owner, 2);
    });
}

#[test]
fn give_feedback_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        let uri = b"ipfs://QmTest123".to_vec();
        assert_ok!(TrustlessAgent::register_agent(RuntimeOrigin::signed(1), uri, vec![]));

        // Authorize feedback from account 2
        assert_ok!(TrustlessAgent::authorize_feedback(
            RuntimeOrigin::signed(1), // agent owner
            0,                        // agent_id
            2,                        // client to authorize
            10,                       // index_limit
            100000                    // expiry_blocks
        ));

        // Give feedback
        let score = 85;
        let tags = vec![b"helpful".to_vec()];
        let file_uri = b"ipfs://QmFeedback123".to_vec();
        let content_hash = sp_core::H256::default();

        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            score,
            tags,
            file_uri,
            content_hash
        ));

        // Check event - last event should be ReputationUpdated after FeedbackGiven
        System::assert_last_event(
            Event::ReputationUpdated { agent_id: 0, overall_score: 5100 }.into(),
        );

        // Check storage
        assert!(TrustlessAgent::feedbacks(0).is_some());
        assert_eq!(TrustlessAgent::feedbacks(0).unwrap().score, score);
    });
}

#[test]
fn give_feedback_validates_score() {
    new_test_ext().execute_with(|| {
        // Register an agent
        let uri = b"ipfs://QmTest123".to_vec();
        assert_ok!(TrustlessAgent::register_agent(RuntimeOrigin::signed(1), uri, vec![]));

        // Authorize feedback from account 2
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 100000));

        // Try to give feedback with invalid score
        let score = 101; // Invalid: > 100
        let tags = vec![];
        let file_uri = b"ipfs://QmFeedback123".to_vec();
        let content_hash = sp_core::H256::default();

        assert_noop!(
            TrustlessAgent::give_feedback(
                RuntimeOrigin::signed(2),
                0,
                score,
                tags,
                file_uri,
                content_hash
            ),
            Error::<Test>::InvalidScore
        );
    });
}

#[test]
fn revoke_feedback_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        let uri = b"ipfs://QmTest123".to_vec();
        assert_ok!(TrustlessAgent::register_agent(RuntimeOrigin::signed(1), uri, vec![]));

        // Authorize feedback from account 2
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 100000));

        // Give feedback
        let score = 85;
        let tags = vec![];
        let file_uri = b"ipfs://QmFeedback123".to_vec();
        let content_hash = sp_core::H256::default();

        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            score,
            tags,
            file_uri,
            content_hash
        ));

        // Revoke feedback
        assert_ok!(TrustlessAgent::revoke_feedback(RuntimeOrigin::signed(2), 0));

        // Check event - last event should be ReputationUpdated after FeedbackRevoked
        System::assert_last_event(
            Event::ReputationUpdated { agent_id: 0, overall_score: 0 }.into(),
        );

        // Check storage
        assert!(TrustlessAgent::feedbacks(0).unwrap().revoked);
    });
}

#[test]
fn register_validator_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register validator
        let stake = 1000;
        assert_ok!(TrustlessAgent::register_validator(RuntimeOrigin::signed(1), stake));

        // Check event
        System::assert_last_event(Event::ValidatorRegistered { validator: 1, stake }.into());

        // Check storage
        assert!(TrustlessAgent::validators(1).is_some());
        assert_eq!(TrustlessAgent::validators(1).unwrap().stake, stake);
    });
}

#[test]
fn register_validator_requires_min_stake() {
    new_test_ext().execute_with(|| {
        // Try to register validator with insufficient stake
        let stake = 500; // Less than MinStake (1000)
        assert_noop!(
            TrustlessAgent::register_validator(RuntimeOrigin::signed(1), stake),
            Error::<Test>::InsufficientStake
        );
    });
}

#[test]
fn request_validation_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        let uri = b"ipfs://QmTest123".to_vec();
        assert_ok!(TrustlessAgent::register_agent(RuntimeOrigin::signed(1), uri, vec![]));

        // Request validation with reward
        let reward = 100;
        assert_ok!(TrustlessAgent::request_validation(RuntimeOrigin::signed(2), 0, reward));

        // Check event
        System::assert_last_event(
            Event::ValidationRequested { request_id: 0, agent_id: 0, requester: 2 }.into(),
        );

        // Check storage
        assert!(TrustlessAgent::validation_requests(0).is_some());
    });
}

#[test]
fn submit_validation_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        let uri = b"ipfs://QmTest123".to_vec();
        assert_ok!(TrustlessAgent::register_agent(RuntimeOrigin::signed(1), uri, vec![]));

        // Register validator
        let stake = 1000;
        assert_ok!(TrustlessAgent::register_validator(RuntimeOrigin::signed(2), stake));

        // Request validation with reward
        let reward = 100;
        assert_ok!(TrustlessAgent::request_validation(RuntimeOrigin::signed(1), 0, reward));

        // Submit validation
        let score = 90;
        let evidence_uri = b"ipfs://QmEvidence123".to_vec();
        let content_hash = sp_core::H256::default();
        let tags = vec![b"verified".to_vec()];

        assert_ok!(TrustlessAgent::submit_validation(
            RuntimeOrigin::signed(2),
            0,
            score,
            evidence_uri,
            content_hash,
            tags
        ));

        // Check event
        System::assert_last_event(
            Event::ValidationSubmitted { request_id: 0, validator: 2, score }.into(),
        );

        // Check storage
        assert!(TrustlessAgent::validation_responses(0, 2).is_some());
    });
}

#[test]
fn escrow_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));

        // Create escrow (client 2 for agent 0)
        let amount = 500;
        let timeout = 10;
        assert_ok!(TrustlessAgent::create_escrow(
            RuntimeOrigin::signed(2),
            0,
            amount,
            timeout,
            None
        ));
        System::assert_last_event(
            Event::EscrowCreated { escrow_id: 0, client: 2, agent_id: 0, amount, timeout }.into(),
        );
        assert_eq!(Balances::reserved_balance(2), amount);

        // Advance to auto-complete time (100800 blocks from creation)
        System::set_block_number(1 + 100800);

        // Agent (owner 1) claims escrow after auto-complete
        assert_ok!(TrustlessAgent::claim_escrow(RuntimeOrigin::signed(1), 0));
        // Check that both EscrowAutoCompleted and EscrowClaimed events were emitted
        System::assert_last_event(Event::EscrowClaimed { escrow_id: 0, agent_id: 0 }.into());
        assert_eq!(Balances::reserved_balance(2), 0);
        assert_eq!(Balances::free_balance(1), 10000 + amount - 100); // Initial + amount - agent deposit
        assert!(!<Escrows<Test>>::contains_key(0));
    });
}

#[test]
fn cancel_escrow_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));

        // Create escrow
        let amount = 500;
        let timeout = 10;
        assert_ok!(TrustlessAgent::create_escrow(
            RuntimeOrigin::signed(2),
            0,
            amount,
            timeout,
            None
        ));
        assert_eq!(Balances::reserved_balance(2), amount);

        // Try to cancel before timeout
        assert_noop!(
            TrustlessAgent::cancel_escrow(RuntimeOrigin::signed(2), 0),
            Error::<Test>::EscrowNotTimedOut
        );

        // Advance blocks past timeout
        System::set_block_number(11);

        // Cancel escrow
        assert_ok!(TrustlessAgent::cancel_escrow(RuntimeOrigin::signed(2), 0));
        System::assert_last_event(Event::EscrowCancelled { escrow_id: 0, client: 2 }.into());
        assert_eq!(Balances::reserved_balance(2), 0);
        assert!(!<Escrows<Test>>::contains_key(0));
    });
}

// ============================================================================
// New Tests: Authorization, Rate Limiting, Escrow Enhancements
// ============================================================================

#[test]
fn authorize_feedback_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));

        // Agent owner authorizes client to give feedback
        let index_limit = 10u32;
        let expiry = 1000u64;
        assert_ok!(TrustlessAgent::authorize_feedback(
            RuntimeOrigin::signed(1),
            0, // agent_id
            2, // client
            index_limit,
            expiry
        ));

        // Verify authorization ID was stored
        assert!(TrustlessAgent::agent_client_authorizations(0, 2).is_some());
        let auth_id = TrustlessAgent::agent_client_authorizations(0, 2).unwrap();

        // Verify authorization data exists
        use crate::FeedbackAuthorizations;
        let auth = FeedbackAuthorizations::<Test>::get(auth_id).unwrap();
        assert_eq!(auth.index_limit, index_limit);
        assert!(!auth.revoked);
    });
}

#[test]
fn give_feedback_requires_authorization() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register an agent
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));

        // Try to give feedback without authorization - should fail
        assert_noop!(
            TrustlessAgent::give_feedback(
                RuntimeOrigin::signed(2),
                0,
                85,
                vec![],
                b"ipfs://feedback".to_vec(),
                [0u8; 32].into()
            ),
            Error::<Test>::NoValidAuthorization
        );

        // Authorize client
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 1000));

        // Now feedback should work
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            85,
            vec![],
            b"ipfs://feedback".to_vec(),
            [0u8; 32].into()
        ));
    });
}

#[test]
fn feedback_rate_limit_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and authorize client with long expiry
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(
            RuntimeOrigin::signed(1),
            0,
            2,
            10,
            200000 // Long expiry to avoid expiration during test (must be > 100800 blocks)
        ));

        // Give first feedback
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            85,
            vec![],
            b"ipfs://feedback1".to_vec(),
            [0u8; 32].into()
        ));

        // Try to give second feedback immediately - should fail (rate limit)
        assert_noop!(
            TrustlessAgent::give_feedback(
                RuntimeOrigin::signed(2),
                0,
                90,
                vec![],
                b"ipfs://feedback2".to_vec(),
                [1u8; 32].into()
            ),
            Error::<Test>::FeedbackRateLimitExceeded
        );

        // Advance past rate limit period (7 days = 100800 blocks in test)
        System::set_block_number(100801);

        // Now second feedback should work
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            90,
            vec![],
            b"ipfs://feedback2".to_vec(),
            [1u8; 32].into()
        ));
    });
}

#[test]
fn authorization_index_limit_enforced() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and authorize client with limit of 2
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(
            RuntimeOrigin::signed(1),
            0,
            2,
            2,      // index_limit = 2
            300000  // Long expiry to survive multiple rate limit periods (must be > 201601 blocks)
        ));

        // First feedback
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            85,
            vec![],
            b"ipfs://feedback1".to_vec(),
            [0u8; 32].into()
        ));

        // Advance past rate limit
        System::set_block_number(100801);

        // Second feedback
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            90,
            vec![],
            b"ipfs://feedback2".to_vec(),
            [1u8; 32].into()
        ));

        // Advance past rate limit again
        System::set_block_number(201601);

        // Third feedback should fail (exceeds index_limit of 2)
        assert_noop!(
            TrustlessAgent::give_feedback(
                RuntimeOrigin::signed(2),
                0,
                95,
                vec![],
                b"ipfs://feedback3".to_vec(),
                [2u8; 32].into()
            ),
            Error::<Test>::AuthorizationIndexLimitExceeded
        );
    });
}

#[test]
fn revoke_authorization_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and authorize client
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 1000));

        // Revoke authorization
        assert_ok!(TrustlessAgent::revoke_authorization(RuntimeOrigin::signed(1), 0, 2));

        // Verify revoked flag is set
        use crate::FeedbackAuthorizations;
        let auth_id = TrustlessAgent::agent_client_authorizations(0, 2).unwrap();
        let auth = FeedbackAuthorizations::<Test>::get(auth_id).unwrap();
        assert!(auth.revoked);

        // Try to give feedback with revoked authorization - should fail with
        // AuthorizationRevoked
        assert_noop!(
            TrustlessAgent::give_feedback(
                RuntimeOrigin::signed(2),
                0,
                85,
                vec![],
                b"ipfs://feedback".to_vec(),
                [0u8; 32].into()
            ),
            Error::<Test>::AuthorizationRevoked
        );
    });
}

#[test]
fn escrow_auto_complete_timing() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));

        // Create escrow
        assert_ok!(TrustlessAgent::create_escrow(RuntimeOrigin::signed(2), 0, 500, 100, None));

        let escrow = TrustlessAgent::escrows(0).unwrap();
        // auto_complete_at should be created_at (1) + 7 days (100800 blocks)
        // DAYS = 24 * 60 * 10 = 14400, so 7 * DAYS = 100800
        assert_eq!(escrow.auto_complete_at, 100801);

        // Try to claim before auto_complete_at - should fail
        System::set_block_number(100800);
        assert_noop!(
            TrustlessAgent::claim_escrow(RuntimeOrigin::signed(1), 0),
            Error::<Test>::EscrowNotAutoCompleted
        );

        // Advance to exactly auto_complete_at
        System::set_block_number(100801);

        // Now claim should work
        assert_ok!(TrustlessAgent::claim_escrow(RuntimeOrigin::signed(1), 0));
    });
}

#[test]
fn dispute_escrow_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and create escrow
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::create_escrow(RuntimeOrigin::signed(2), 0, 500, 100, None));

        // Client disputes escrow
        assert_ok!(TrustlessAgent::dispute_escrow(RuntimeOrigin::signed(2), 0));

        // Check escrow status changed to Disputed
        let escrow = TrustlessAgent::escrows(0).unwrap();
        assert!(matches!(escrow.status, crate::EscrowStatus::Disputed));

        // Agent should not be able to claim disputed escrow
        System::set_block_number(50401); // Past auto-complete
        assert_noop!(
            TrustlessAgent::claim_escrow(RuntimeOrigin::signed(1), 0),
            Error::<Test>::EscrowDisputed
        );
    });
}

#[test]
fn resolve_escrow_dispute_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and create escrow
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::create_escrow(RuntimeOrigin::signed(2), 0, 500, 100, None));

        // Client disputes
        assert_ok!(TrustlessAgent::dispute_escrow(RuntimeOrigin::signed(2), 0));

        let client_balance_before = Balances::free_balance(2);

        // Resolve in favor of client (refund)
        assert_ok!(TrustlessAgent::resolve_escrow_dispute(RuntimeOrigin::root(), 0, true));

        // Check funds returned to client
        let client_balance_after = Balances::free_balance(2);
        assert_eq!(client_balance_after, client_balance_before + 500);

        // Escrow should be removed
        assert!(!TrustlessAgent::escrows(0).is_some());
    });
}

#[test]
fn reputation_time_decay_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and authorize client
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 1000000));

        // Give first feedback at block 1 with score 100
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            100,
            vec![],
            b"ipfs://feedback1".to_vec(),
            [0u8; 32].into()
        ));

        // Get initial reputation (100% weight for recent feedback)
        use crate::AgentReputations;
        let reputation_fresh = AgentReputations::<Test>::get(0).unwrap();
        let _fresh_score = reputation_fresh.feedback_score;

        // Advance to 91 days later (91 * 7200 = 655200 blocks)
        // Feedback should now have 40% weight
        System::set_block_number(655201);

        // Give second feedback with score 50 to trigger reputation recalculation
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 3, 10, 1000000));
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(3),
            0,
            50,
            vec![],
            b"ipfs://feedback2".to_vec(),
            [1u8; 32].into()
        ));

        // Get updated reputation
        let reputation_decayed = AgentReputations::<Test>::get(0).unwrap();
        let decayed_score = reputation_decayed.feedback_score;

        // The average should be weighted: (100*40 + 50*100) / (40+100) = 9000/140 ≈
        // 64.28 This should be lower than simple average of (100+50)/2 = 75
        // Verify time decay had an effect (old feedback weighted less)
        assert!(decayed_score < 7500, "Time decay should reduce impact of old feedback");
        assert!(decayed_score > 5000, "But shouldn't completely ignore old feedback");
    });
}

// ============================================================================
// Additional Tests for 100% Coverage
// ============================================================================

#[test]
fn append_response_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and authorize client
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 1000));

        // Give feedback
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            85,
            vec![],
            b"ipfs://feedback".to_vec(),
            [0u8; 32].into()
        ));

        // Agent owner appends response
        let response_uri = b"ipfs://response123".to_vec();
        let response_hash = sp_core::H256::default();

        assert_ok!(TrustlessAgent::append_response(
            RuntimeOrigin::signed(1),
            0,
            response_uri.clone(),
            response_hash
        ));

        // Check event
        System::assert_last_event(Event::ResponseAppended { feedback_id: 0, responder: 1 }.into());
    });
}

#[test]
fn append_response_requires_agent_owner() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and give feedback
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 1000));
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            85,
            vec![],
            b"ipfs://feedback".to_vec(),
            [0u8; 32].into()
        ));

        // Try to append response as non-owner - should fail
        assert_noop!(
            TrustlessAgent::append_response(
                RuntimeOrigin::signed(3),
                0,
                b"ipfs://response".to_vec(),
                [0u8; 32].into()
            ),
            Error::<Test>::NotAgentOwner
        );
    });
}

#[test]
fn unregister_validator_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register validator
        let stake = 1000;
        assert_ok!(TrustlessAgent::register_validator(RuntimeOrigin::signed(1), stake));
        assert_eq!(Balances::reserved_balance(1), stake);

        // Unregister validator
        assert_ok!(TrustlessAgent::unregister_validator(RuntimeOrigin::signed(1)));

        // Check event
        System::assert_last_event(Event::ValidatorUnregistered { validator: 1 }.into());

        // Check storage updated
        assert!(TrustlessAgent::validators(1).is_none());

        // Check balance unreserved
        assert_eq!(Balances::reserved_balance(1), 0);
    });
}

#[test]
fn unregister_validator_not_registered() {
    new_test_ext().execute_with(|| {
        // Try to unregister non-existent validator
        assert_noop!(
            TrustlessAgent::unregister_validator(RuntimeOrigin::signed(1)),
            Error::<Test>::ValidatorNotFound
        );
    });
}

#[test]
fn cancel_validation_request_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and request validation
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::request_validation(RuntimeOrigin::signed(2), 0, 100));

        // Try to cancel before deadline - should fail
        assert_noop!(
            TrustlessAgent::cancel_validation_request(RuntimeOrigin::signed(2), 0),
            Error::<Test>::ValidationDeadlineNotPassed
        );

        // Advance past deadline (ValidationDeadline = 100)
        System::set_block_number(101);

        // Cancel validation request
        assert_ok!(TrustlessAgent::cancel_validation_request(RuntimeOrigin::signed(2), 0));

        // Check event
        System::assert_last_event(
            Event::ValidationRequestCancelled { request_id: 0, requester: 2 }.into(),
        );

        // Check storage
        assert!(TrustlessAgent::validation_requests(0).is_none());
    });
}

#[test]
fn dispute_feedback_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and give feedback
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 1000));
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            85,
            vec![],
            b"ipfs://feedback".to_vec(),
            [0u8; 32].into()
        ));

        // Agent owner disputes feedback
        let reason_uri = b"ipfs://dispute_reason".to_vec();
        let reason_hash = sp_core::H256::default();

        assert_ok!(TrustlessAgent::dispute_feedback(
            RuntimeOrigin::signed(1),
            0,
            reason_uri.clone(),
            reason_hash
        ));

        // Check event
        System::assert_last_event(
            Event::DisputeCreated { dispute_id: 0, feedback_id: 0, disputer: 1 }.into(),
        );

        // Check storage
        assert!(TrustlessAgent::disputes(0).is_some());
        let dispute = TrustlessAgent::disputes(0).unwrap();
        assert_eq!(dispute.disputer, 1);
    });
}

#[test]
fn dispute_feedback_requires_agent_owner() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Register agent and give feedback
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 1000));
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            85,
            vec![],
            b"ipfs://feedback".to_vec(),
            [0u8; 32].into()
        ));

        // Try to dispute as non-owner - should fail
        assert_noop!(
            TrustlessAgent::dispute_feedback(
                RuntimeOrigin::signed(3),
                0,
                b"ipfs://reason".to_vec(),
                [0u8; 32].into()
            ),
            Error::<Test>::NotAgentOwner
        );
    });
}

#[test]
fn resolve_dispute_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Setup: agent, feedback, and dispute
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 1000));
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            85,
            vec![],
            b"ipfs://feedback".to_vec(),
            [0u8; 32].into()
        ));
        assert_ok!(TrustlessAgent::dispute_feedback(
            RuntimeOrigin::signed(1),
            0,
            b"ipfs://reason".to_vec(),
            [0u8; 32].into()
        ));

        // Resolve in favor of disputer
        assert_ok!(TrustlessAgent::resolve_dispute(
            RuntimeOrigin::root(),
            0,
            DisputeStatus::ResolvedForDisputer
        ));

        // Check event
        System::assert_last_event(
            Event::DisputeResolved { dispute_id: 0, status: DisputeStatus::ResolvedForDisputer }
                .into(),
        );

        // Check dispute status updated
        let dispute = TrustlessAgent::disputes(0).unwrap();
        assert_eq!(dispute.status, DisputeStatus::ResolvedForDisputer);
    });
}

#[test]
fn resolve_dispute_against_disputer() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Setup: agent, feedback, and dispute
        assert_ok!(TrustlessAgent::register_agent(
            RuntimeOrigin::signed(1),
            b"uri".to_vec(),
            vec![]
        ));
        assert_ok!(TrustlessAgent::authorize_feedback(RuntimeOrigin::signed(1), 0, 2, 10, 1000));
        assert_ok!(TrustlessAgent::give_feedback(
            RuntimeOrigin::signed(2),
            0,
            85,
            vec![],
            b"ipfs://feedback".to_vec(),
            [0u8; 32].into()
        ));
        assert_ok!(TrustlessAgent::dispute_feedback(
            RuntimeOrigin::signed(1),
            0,
            b"ipfs://reason".to_vec(),
            [0u8; 32].into()
        ));

        // Resolve in favor of feedback provider (against disputer)
        assert_ok!(TrustlessAgent::resolve_dispute(
            RuntimeOrigin::root(),
            0,
            DisputeStatus::ResolvedAgainstDisputer
        ));

        // Check dispute status
        let dispute = TrustlessAgent::disputes(0).unwrap();
        assert_eq!(dispute.status, DisputeStatus::ResolvedAgainstDisputer);
    });
}

#[test]
fn add_validator_to_whitelist_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let validator = account_id::<u64>(42);

        // Add validator to whitelist (root only)
        assert_ok!(TrustlessAgent::add_validator_to_whitelist(RuntimeOrigin::root(), validator));

        // Check event
        System::assert_last_event(Event::ValidatorWhitelisted { validator }.into());

        // Check storage
        assert!(TrustlessAgent::validator_whitelist(validator));
    });
}

#[test]
fn add_validator_to_whitelist_requires_root() {
    new_test_ext().execute_with(|| {
        // Try to add validator as non-root - should fail
        assert_noop!(
            TrustlessAgent::add_validator_to_whitelist(RuntimeOrigin::signed(1), 2),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn remove_validator_from_whitelist_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        let validator = account_id::<u64>(42);

        // Add validator to whitelist first
        assert_ok!(TrustlessAgent::add_validator_to_whitelist(RuntimeOrigin::root(), validator));

        // Remove validator from whitelist
        assert_ok!(TrustlessAgent::remove_validator_from_whitelist(
            RuntimeOrigin::root(),
            validator
        ));

        // Check event
        System::assert_last_event(Event::ValidatorRemovedFromWhitelist { validator }.into());

        // Check storage
        assert!(!TrustlessAgent::validator_whitelist(validator));
    });
}

#[test]
fn remove_validator_from_whitelist_requires_root() {
    new_test_ext().execute_with(|| {
        // Try to remove validator as non-root - should fail
        assert_noop!(
            TrustlessAgent::remove_validator_from_whitelist(RuntimeOrigin::signed(1), 2),
            frame_support::error::BadOrigin
        );
    });
}

// Helper function to create AccountId
fn account_id<T: From<u64>>(id: u64) -> T { id.into() }
