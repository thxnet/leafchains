//! Tests for DAO pallet.
// Test cases for DAO pallet: https://thxlab.jetbrains.space/p/thx-mainnet/documents/a/NKoOP1iBRej

use std::sync::Arc;

use frame_support::{
    assert_err, assert_ok,
    traits::{Currency, Hooks},
};
use sp_keystore::{testing::KeyStore, KeystoreExt};
use sp_runtime::BoundedVec;

use super::*;
use crate::mock::{new_test_ext, Balances, Dao, RuntimeOrigin, System, Test, Timestamp, UNITS};

const ONE_MILLISECOND: u64 = 1000;
const ONE_SECOND: u64 = 1;
const ONE_MINUTE: u64 = 60 * ONE_SECOND;
const ONE_HOUR: u64 = 60 * ONE_MINUTE;

// 1. Successful Topic Raise and Voting
#[test]
fn test_successful_topic_raise_and_voting() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1_000_000 * UNITS;
    let bob = 3;
    let bob_balance = 100 * UNITS;
    let charlie = 4;
    let charlie_balance = 1_000_000 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance), (charlie, charlie_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Proposal for Community Improvement".as_bytes().to_vec(),
            "This proposal aims to improve community engagement.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec![
                "Option A".as_bytes().to_vec(),
                "Option B".as_bytes().to_vec(),
                "Option C".as_bytes().to_vec()
            ],
            2
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![(bob)],
            Some(100)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: 100 * bob_balance,
            }
            .into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![(charlie)],
            Some(10_000)
        ));

        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: charlie,
                weight_per_required_option: 10_000 * charlie_balance,
            }
            .into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let block_number_for_voting = System::block_number() + 1000;
        System::set_block_number(block_number_for_voting);
        <Timestamp as Hooks<u64>>::on_initialize(block_number_for_voting);

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            voting_period_start_in_second * ONE_MILLISECOND
        ));
        assert_eq!(Timestamp::now(), voting_period_start_in_second * ONE_MILLISECOND);

        assert_ok!(Dao::vote_topic(
            RuntimeOrigin::signed(bob),
            topic_id,
            BoundedVec::try_from(vec![0, 1]).unwrap()
        ));
        System::assert_last_event(
            Event::<Test>::TopicVoted {
                id: topic_id,
                voter: bob,
                voted_options: BoundedVec::try_from(vec![0, 1]).unwrap(),
            }
            .into(),
        );

        assert_ok!(Dao::vote_topic(
            RuntimeOrigin::signed(charlie),
            topic_id,
            BoundedVec::try_from(vec![0, 2]).unwrap()
        ));
        System::assert_last_event(
            Event::<Test>::TopicVoted {
                id: topic_id,
                voter: charlie,
                voted_options: BoundedVec::try_from(vec![0, 2]).unwrap(),
            }
            .into(),
        );
    });
}

// 2. Invalid Proposal (raiser has insufficient balance)
#[test]
fn test_invalid_topic_raise() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    let bob = 3;
    let bob_balance = 500_000 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> { balances: vec![(bob, bob_balance)] }
        .assimilate_storage(&mut chain_state)
        .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_err!(
            Dao::raise_topic(
                RuntimeOrigin::signed(bob),
                "Invalid Proposal".as_bytes().to_vec(),
                "This proposal should fail due to insufficient balance.".as_bytes().to_vec(),
                voting_period_start_in_second,
                voting_period_end_in_second,
                vec!["Yes".as_bytes().to_vec(), "No".as_bytes().to_vec(),],
                1
            ),
            Error::<Test, _>::InsufficientBalance
        );
    });
}

// 3. Voting Period Expired
#[test]
fn test_voting_period_expired() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 2_000_000 * UNITS;
    let bob = 3;
    let bob_balance = 150 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Expired Topic".as_bytes().to_vec(),
            "This topic should fail due to an expired voting period.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Disagree".as_bytes().to_vec(), "Agree".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![(bob)],
            Some(150)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: 150 * bob_balance,
            }
            .into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let block_number_for_voting = System::block_number() + 1000;
        System::set_block_number(block_number_for_voting);
        <Timestamp as Hooks<u64>>::on_initialize(block_number_for_voting);

        // vote after the vote is closed
        let voting_time = (voting_period_end_in_second + 1 * ONE_HOUR) * ONE_MILLISECOND;
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), voting_time));
        assert_eq!(Timestamp::now(), voting_time);

        assert_err!(
            Dao::vote_topic(
                RuntimeOrigin::signed(bob),
                topic_id,
                BoundedVec::try_from(vec![0]).unwrap()
            ),
            Error::<Test, _>::VoteClosed
        );
    });
}

// 4. Duplicate Vote
#[test]
fn test_duplicate_vote() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1_500_000 * UNITS;
    let bob = 3;
    let bob_balance = 200 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Double Vote Check".as_bytes().to_vec(),
            "This topic should fail due to a duplicate vote attempt.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Approve".as_bytes().to_vec(), "Reject".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![bob],
            Some(200)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: 200 * bob_balance,
            }
            .into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let block_number_for_voting = System::block_number() + 1000;
        System::set_block_number(block_number_for_voting);
        <Timestamp as Hooks<u64>>::on_initialize(block_number_for_voting);

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            voting_period_start_in_second * ONE_MILLISECOND
        ));
        assert_eq!(Timestamp::now(), voting_period_start_in_second * ONE_MILLISECOND);

        assert_ok!(Dao::vote_topic(
            RuntimeOrigin::signed(bob),
            topic_id,
            BoundedVec::try_from(vec![0]).unwrap()
        ));
        System::assert_last_event(
            Event::<Test>::TopicVoted {
                id: topic_id,
                voter: bob,
                voted_options: BoundedVec::try_from(vec![0]).unwrap(),
            }
            .into(),
        );

        assert_err!(
            Dao::vote_topic(
                RuntimeOrigin::signed(bob),
                topic_id,
                BoundedVec::try_from(vec![0]).unwrap()
            ),
            Error::<Test>::VoterHasVoted,
        );
    });
}

// 5. Successful Voting with Minimum Requirements
#[test]
fn test_successful_voting_with_minimum_requirements() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1_000_000 * UNITS;
    let bob = 3;
    let bob_balance = 100 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Minimum Requirements".as_bytes().to_vec(),
            "This topic tests the minimum requirements for a successful raise.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Yes".as_bytes().to_vec(), "No".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![bob],
            Some(100)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: 100 * bob_balance,
            }
            .into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let block_number_for_voting = System::block_number() + 1000;
        System::set_block_number(block_number_for_voting);
        <Timestamp as Hooks<u64>>::on_initialize(block_number_for_voting);

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            voting_period_start_in_second * ONE_MILLISECOND
        ));
        assert_eq!(Timestamp::now(), voting_period_start_in_second * ONE_MILLISECOND);

        assert_ok!(Dao::vote_topic(
            RuntimeOrigin::signed(bob),
            topic_id,
            BoundedVec::try_from(vec![0]).unwrap()
        ));
        System::assert_last_event(
            Event::<Test>::TopicVoted {
                id: topic_id,
                voter: bob,
                voted_options: BoundedVec::try_from(vec![0]).unwrap(),
            }
            .into(),
        );
    });
}

// 6. Invalid VRT Issuance
#[test]
fn test_invalid_vrt_issuance() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1 * UNITS;
    let bob = 1;
    let bob_balance = 1 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        let topic_id = 0;
        assert_err!(
            Dao::issue_voting_right_token(
                RuntimeOrigin::signed(alice),
                topic_id,
                vec![bob],
                Some(100)
            ),
            Error::<Test, _>::UnknownTopic
        );
    });
}

// 7. Invalid Voting Weight Ratio
#[test]
fn test_invalid_voting_weight_ratio() {
    // TODO: this one is removed on documentation??
}

// 8. Vote with Insufficient Weight
#[test]
fn test_vote_with_insufficient_weight() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1_500_000 * UNITS;
    let bob = 3;
    let bob_balance = 50 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Insufficient Weight".as_bytes().to_vec(),
            "This topic should fail due to insufficient weight for voting.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Approve".as_bytes().to_vec(), "Reject".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![bob],
            Some(50)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: 50 * bob_balance,
            }
            .into(),
        );
    });
}

// 9. Invalid Vote Option
#[test]
fn test_invalid_vote_option() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 2_500_000 * UNITS;
    let bob = 3;
    let bob_balance = 200 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Invalid Vote Option".as_bytes().to_vec(),
            "This topic should fail due to an invalid vote option.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Choice 1".as_bytes().to_vec(), "Choice 2".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![bob],
            Some(200)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: 200 * bob_balance,
            }
            .into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        System::set_block_number(System::block_number() + voting_period_start_in_second);
        <Timestamp as Hooks<u64>>::on_initialize(
            System::block_number() + voting_period_start_in_second,
        );

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            voting_period_start_in_second * ONE_MILLISECOND
        ));
        assert_eq!(Timestamp::now(), voting_period_start_in_second * ONE_MILLISECOND);

        // Valid option are 0 and 1. However, Bob vote an invalid option 2
        assert_err!(
            Dao::vote_topic(
                RuntimeOrigin::signed(bob),
                topic_id,
                BoundedVec::try_from(vec![2]).unwrap()
            ),
            Error::<Test, _>::InvalidOption
        );
    });
}

// 10. Successful Topic Raise with Maximum Options
#[test]
fn test_successful_topic_raise_with_maximum_options() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 3_000_000 * UNITS;
    let bob = 3;
    let bob_balance = 300 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        let options = (1..=1024).map(|i| format!("Option {i}").as_bytes().to_vec()).collect();

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Max Options".as_bytes().to_vec(),
            "This topic tests the maximum number of options.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            options,
            3
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![bob],
            Some(1)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: bob_balance,
            }
            .into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        System::set_block_number(System::block_number() + voting_period_start_in_second);
        <Timestamp as Hooks<u64>>::on_initialize(
            System::block_number() + voting_period_start_in_second,
        );

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            voting_period_start_in_second * ONE_MILLISECOND
        ));
        assert_eq!(Timestamp::now(), voting_period_start_in_second * ONE_MILLISECOND);

        // Bob votes for "Option 1," "Option 2," and "Option 3."
        assert_ok!(Dao::vote_topic(
            RuntimeOrigin::signed(bob),
            topic_id,
            BoundedVec::try_from(vec![0, 1, 2]).unwrap()
        ),);
    });
}

// 11. Duplicate VRT Issuance
#[test]
fn test_duplicate_vrt_issuance() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1_200_000 * UNITS;
    let bob = 3;
    let bob_balance = 100 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Duplicate VRT Issuance".as_bytes().to_vec(),
            "This topic should fail due to duplicate VRT issuance attempt.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Approve".as_bytes().to_vec(), "Reject".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let block_number_for_issue_vrt =
            System::block_number() + voting_period_start_in_second - 30;

        System::set_block_number(block_number_for_issue_vrt);
        <Timestamp as Hooks<u64>>::on_initialize(block_number_for_issue_vrt);

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            (voting_period_start_in_second - 30) * ONE_MILLISECOND
        ));
        assert_eq!(Timestamp::now(), (voting_period_start_in_second - 30) * ONE_MILLISECOND);

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![bob],
            Some(1)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: 1 * bob_balance,
            }
            .into(),
        );

        // issue voting right token for Bob again
        assert_err!(
            Dao::issue_voting_right_token(
                RuntimeOrigin::signed(alice),
                topic_id,
                vec![bob],
                Some(1)
            ),
            Error::<Test, _>::VotingRightTokenIssued
        );
    });
}

// 12. Invalid Vote Period
#[test]
fn test_invalid_vote_period() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1_200_000 * UNITS;
    let bob = 3;
    let bob_balance = 100 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 4 * ONE_HOUR;
    let now = 4 * ONE_HOUR;
    let voting_period_start_in_second = now - 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second - 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_err!(
            Dao::raise_topic(
                RuntimeOrigin::signed(alice),
                "Invalid Vote Period".as_bytes().to_vec(),
                "This topic should fail due to an invalid vote period.".as_bytes().to_vec(),
                voting_period_start_in_second,
                voting_period_end_in_second,
                vec!["Approve".as_bytes().to_vec(), "Reject".as_bytes().to_vec()],
                1
            ),
            Error::<Test, _>::InvalidVotingPeriodEnd
        );
    });
}

// 13.  Successful VRT Issuance with Default Weight Ratio
#[test]
fn test_successful_vrt_issuance_with_default_weight_ratio() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1_500_000 * UNITS;
    let bob = 3;
    let bob_balance = 100 * UNITS;
    let charlie = 4;
    let charlie_balance = 10_000 * UNITS;
    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance), (charlie, charlie_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Default Weight Ratio".as_bytes().to_vec(),
            "This topic tests the default weight ratio for VRT issuance.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Yes".as_bytes().to_vec(), "No".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let block_number_for_issue_vrt =
            System::block_number() + voting_period_start_in_second - 30;

        System::set_block_number(block_number_for_issue_vrt);
        <Timestamp as Hooks<u64>>::on_initialize(block_number_for_issue_vrt);

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            (voting_period_start_in_second - 30) * ONE_MILLISECOND
        ));
        assert_eq!(Timestamp::now(), (voting_period_start_in_second - 30) * ONE_MILLISECOND);

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![bob],
            Some(100)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: 100 * bob_balance,
            }
            .into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![charlie],
            Some(10_000)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: charlie,
                weight_per_required_option: 10_000 * charlie_balance,
            }
            .into(),
        );
    });
}

// 14. Successful VRT Issuance with Custom Weight Ratio
#[test]
fn test_successful_vrt_issuance_with_custom_weight_ratio() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 2_000_000 * UNITS;
    let bob = 3;
    let bob_balance = 100 * UNITS;
    let charlie = 4;
    let charlie_balance = 10_000 * UNITS;
    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance), (charlie, charlie_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Custom Weight Ratio".as_bytes().to_vec(),
            "This topic tests VRT issuance with a custom weight ratio.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Agree".as_bytes().to_vec(), "Disagree".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let block_number_for_issue_vrt =
            System::block_number() + voting_period_start_in_second - 30;

        System::set_block_number(block_number_for_issue_vrt);
        <Timestamp as Hooks<u64>>::on_initialize(block_number_for_issue_vrt);

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            (voting_period_start_in_second - 30) * ONE_MILLISECOND
        ));
        assert_eq!(Timestamp::now(), (voting_period_start_in_second - 30) * ONE_MILLISECOND);

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![bob],
            Some(150)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: bob,
                weight_per_required_option: 150 * bob_balance,
            }
            .into(),
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            vec![charlie],
            Some(10_000)
        ));
        System::assert_has_event(
            Event::<Test>::VotingRightTokenIssued {
                topic_id,
                voter: charlie,
                weight_per_required_option: 10_000 * charlie_balance,
            }
            .into(),
        );
    });
}

// 15. VRT Issuance for Expired Topic
#[test]
fn test_vrt_issuance_for_expired_topic() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1_800_000 * UNITS;
    let bob = 3;
    let bob_balance = 150 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Expired Topic VRT Issuance".as_bytes().to_vec(),
            "This topic should fail due to VRT issuance for an expired topic.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Option A".as_bytes().to_vec(), "Option B".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let block_number_for_issue_vrt =
            System::block_number() + voting_period_start_in_second - 30;

        System::set_block_number(block_number_for_issue_vrt);
        <Timestamp as Hooks<u64>>::on_initialize(block_number_for_issue_vrt);

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            (voting_period_end_in_second + 1 * ONE_HOUR) * ONE_MILLISECOND
        ));
        assert_eq!(
            Timestamp::now(),
            (voting_period_end_in_second + 1 * ONE_HOUR) * ONE_MILLISECOND
        );

        assert_err!(
            Dao::issue_voting_right_token(
                RuntimeOrigin::signed(alice),
                topic_id,
                vec![bob],
                Some(150)
            ),
            Error::<Test, _>::VoteClosed
        );
    });
}

// 16. VRT Issuance for Non-Existing Topic
#[test]
fn test_vrt_issuance_for_non_existing_topic() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 1_800_000 * UNITS;
    let bob = 3;
    let bob_balance = 150 * UNITS;

    pallet_balances::pallet::GenesisConfig::<Test> {
        balances: vec![(alice, alice_balance), (bob, bob_balance)],
    }
    .assimilate_storage(&mut chain_state)
    .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        // NOTE: the topic is not created, it does not exist.
        let topic_id = 39393;
        assert_err!(
            Dao::issue_voting_right_token(
                RuntimeOrigin::signed(alice),
                topic_id,
                vec![bob],
                Some(150)
            ),
            Error::<Test, _>::UnknownTopic
        );
    });
}

// 17. VRT Issuance with Maximum Voters
#[test]
fn test_vrt_issuance_with_maximum_voters() {
    let mut chain_state = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    let alice = 2;
    let alice_balance = 3_000_000 * UNITS;
    let voters = 3..1027;
    let voter_balance = 100 * UNITS;

    let balances = {
        let mut balances = vec![(alice, alice_balance)];
        for voter in voters.clone() {
            balances.push((voter, voter_balance));
        }
        balances
    };

    pallet_balances::pallet::GenesisConfig::<Test> { balances }
        .assimilate_storage(&mut chain_state)
        .unwrap();

    let keystore = KeyStore::new();
    let mut ext = sp_io::TestExternalities::new(chain_state);
    ext.register_extension(KeystoreExt(Arc::new(keystore)));
    ext.execute_with(|| System::set_block_number(6));

    let initial_timestamp = 90 * ONE_SECOND;
    let now = 120 * ONE_SECOND;
    let voting_period_start_in_second = now + 1 * ONE_HOUR;
    let voting_period_end_in_second = voting_period_start_in_second + 3 * ONE_HOUR;

    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46 * ONE_SECOND);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);

        // Raise a new topic
        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(alice),
            "Maximum Voters VRT Issuance".as_bytes().to_vec(),
            "This topic tests VRT issuance with the maximum number of voters.".as_bytes().to_vec(),
            voting_period_start_in_second,
            voting_period_end_in_second,
            vec!["Choice 1".as_bytes().to_vec(), "Choice 2".as_bytes().to_vec()],
            1
        ));

        let topic_id = 0;
        System::assert_last_event(
            Event::<Test>::TopicRaised { id: topic_id, raiser: alice }.into(),
        );

        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        let block_number_for_issue_vrt =
            System::block_number() + voting_period_start_in_second - 30;

        System::set_block_number(block_number_for_issue_vrt);
        <Timestamp as Hooks<u64>>::on_initialize(block_number_for_issue_vrt);

        assert_ok!(Timestamp::set(
            RuntimeOrigin::none(),
            (voting_period_start_in_second + 1 * ONE_HOUR) * ONE_MILLISECOND
        ));
        assert_eq!(
            Timestamp::now(),
            (voting_period_start_in_second + 1 * ONE_HOUR) * ONE_MILLISECOND
        );

        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(alice),
            topic_id,
            voters.clone().collect(),
            None
        ));

        for voter in voters.clone() {
            let weight_per_required_option = Balances::total_balance(&voter);

            System::assert_has_event(
                Event::<Test>::VotingRightTokenIssued {
                    topic_id,
                    voter,
                    weight_per_required_option,
                }
                .into(),
            );
        }
    });
}

#[test]
fn test_voting_flow() {
    let voting_period_start = 1_000_000;
    let voting_period_end = voting_period_start + 1_000_000;
    let initial_timestamp = 90_000;
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46_000);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), initial_timestamp));
        assert_eq!(Timestamp::now(), initial_timestamp);
        let raiser = 2;

        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(raiser),
            "dao title".as_bytes().to_vec(),
            "dao description".as_bytes().to_vec(),
            voting_period_start,
            voting_period_end,
            vec![
                "alpha".as_bytes().to_vec(),
                "bravo".as_bytes().to_vec(),
                "charlie".as_bytes().to_vec()
            ],
            2
        ));

        let topic_id = 0;
        System::assert_last_event(Event::<Test>::TopicRaised { id: topic_id, raiser }.into());

        let voter_count = 25;
        let voters: Vec<_> = (1..=voter_count).collect();
        assert_ok!(Dao::issue_voting_right_token(
            RuntimeOrigin::signed(2),
            topic_id,
            voters.clone(),
            None
        ));
        for voter in voters {
            let weight_per_required_option = Balances::total_balance(&voter);

            System::assert_has_event(
                Event::<Test>::VotingRightTokenIssued {
                    topic_id,
                    voter,
                    weight_per_required_option,
                }
                .into(),
            );
        }
        <Timestamp as Hooks<u64>>::on_finalize(System::block_number());
        System::on_finalize(System::block_number());

        System::set_block_number(System::block_number() + voting_period_start);
        <Timestamp as Hooks<u64>>::on_initialize(System::block_number() + voting_period_start);

        assert_ok!(Timestamp::set(RuntimeOrigin::none(), voting_period_start * ONE_MILLISECOND));
        assert_eq!(Timestamp::now(), voting_period_start * ONE_MILLISECOND);

        let options = BoundedVec::try_from(vec![0, 2]).unwrap();
        for voter in 1..=voter_count {
            assert_ok!(Dao::vote_topic(RuntimeOrigin::signed(voter), topic_id, options.clone()));
            System::assert_last_event(
                Event::<Test>::TopicVoted { id: topic_id, voter, voted_options: options.clone() }
                    .into(),
            );
        }

        let mut votes_result = Dao::get_topic_votes_result_by_id(topic_id);
        votes_result.sort_unstable();
        let weight_per_required_option = Balances::total_balance(&(voter_count - 1));

        assert_eq!(
            votes_result,
            vec![
                TopicVotingResult {
                    index: 0,
                    vote_weight: weight_per_required_option * u128::from(voter_count)
                },
                TopicVotingResult {
                    index: 2,
                    vote_weight: weight_per_required_option * u128::from(voter_count)
                }
            ]
        );
    });
}

#[test]
fn test_raise_topic() {
    new_test_ext().execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46);
        assert_ok!(Timestamp::set(RuntimeOrigin::none(), 69));
        assert_eq!(Timestamp::now(), 69);

        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(2),
            "dao title".as_bytes().to_vec(),
            "dao description".as_bytes().to_vec(),
            100000,
            300000,
            vec![
                "alpha".as_bytes().to_vec(),
                "bravo".as_bytes().to_vec(),
                "charlie".as_bytes().to_vec()
            ],
            2
        ));

        assert_eq!(
            Dao::get_topic_by_id(0),
            Some(TopicDetails {
                raiser: 2,
                title: BoundedVec::try_from("dao title".as_bytes().to_vec()).unwrap(),
                description: BoundedVec::try_from("dao description".as_bytes().to_vec()).unwrap(),
                voting_period_start: 100000,
                voting_period_end: 300000,
                required_answer_number: 2,
                options: BoundedVec::try_from(vec![
                    BoundedVec::try_from("alpha".as_bytes().to_vec()).unwrap(),
                    BoundedVec::try_from("bravo".as_bytes().to_vec()).unwrap(),
                    BoundedVec::try_from("charlie".as_bytes().to_vec()).unwrap(),
                ])
                .unwrap(),
            })
        );

        assert_ok!(Dao::raise_topic(
            RuntimeOrigin::signed(5),
            "dao title".as_bytes().to_vec(),
            "dao description".as_bytes().to_vec(),
            100000,
            300000,
            vec![
                "delta".as_bytes().to_vec(),
                "echo".as_bytes().to_vec(),
                "foxtrot".as_bytes().to_vec(),
            ],
            1
        ));

        assert_eq!(
            Dao::get_topic_by_id(1),
            Some(TopicDetails {
                raiser: 5,
                title: BoundedVec::try_from("dao title".as_bytes().to_vec()).unwrap(),
                description: BoundedVec::try_from("dao description".as_bytes().to_vec()).unwrap(),
                voting_period_start: 100000,
                voting_period_end: 300000,
                required_answer_number: 1,
                options: BoundedVec::try_from(vec![
                    BoundedVec::try_from("delta".as_bytes().to_vec()).unwrap(),
                    BoundedVec::try_from("echo".as_bytes().to_vec()).unwrap(),
                    BoundedVec::try_from("foxtrot".as_bytes().to_vec()).unwrap(),
                ])
                .unwrap(),
            })
        );

        assert_err!(
            Dao::raise_topic(
                RuntimeOrigin::signed(7),
                "dao title".as_bytes().to_vec(),
                "dao description".as_bytes().to_vec(),
                100000,
                300000,
                vec![
                    "delta".as_bytes().to_vec(),
                    "delta".as_bytes().to_vec(),
                    "echo".as_bytes().to_vec(),
                    "foxtrot".as_bytes().to_vec(),
                ],
                1
            ),
            Error::<Test, _>::DuplicatedOption
        );

        assert_err!(
            Dao::raise_topic(
                RuntimeOrigin::signed(7),
                "dao title".as_bytes().to_vec(),
                "dao description".as_bytes().to_vec(),
                100000,
                300000,
                vec![
                    "delta".as_bytes().to_vec(),
                    "echo".as_bytes().to_vec(),
                    "foxtrot".as_bytes().to_vec(),
                ],
                20
            ),
            Error::<Test, _>::InvalidAnswerNumber
        );

        assert_err!(
            Dao::raise_topic(
                RuntimeOrigin::signed(7),
                "dao title".as_bytes().to_vec(),
                "dao description".as_bytes().to_vec(),
                100000,
                300000,
                vec![
                    "delta".as_bytes().to_vec(),
                    "echo".as_bytes().to_vec(),
                    "foxtrot".as_bytes().to_vec(),
                ],
                0
            ),
            Error::<Test, _>::InvalidAnswerNumber
        );
    });
}
