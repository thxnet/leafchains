//! Tests for DAO pallet.

use frame_support::{
    assert_err, assert_ok,
    traits::{Currency, Hooks},
};
use sp_runtime::BoundedVec;

use super::*;
use crate::mock::{new_test_ext, Balances, Dao, RuntimeOrigin, System, Test, Timestamp};

const ONE_MILLISECOND: u64 = 1000;

#[test]
fn test_voting_flow() {
    let voting_period_start = 100000;
    let voting_period_end = voting_period_start + 100000;
    let initial_timestamp = 90;
    let mut ext = new_test_ext();
    ext.execute_with(|| {
        pallet_timestamp::Now::<Test>::put(46);
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
            voters.clone()
        ));
        for voter in voters {
            let weight_per_required_option = Balances::total_balance(&voter) / 100;

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
        let weight_per_required_option = Balances::total_balance(&(voter_count - 1)) / 100;

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
