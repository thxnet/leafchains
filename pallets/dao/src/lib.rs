#![cfg_attr(not(feature = "std"), no_std)]

mod macros;
mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;
pub use types::{Incrementable, TopicDetails, TopicVotingResult};

#[frame_support::pallet]
pub mod pallet {

    use core::ops::{Add, AddAssign};

    use frame_support::{
        pallet_prelude::*,
        traits::{tokens::currency::Currency, UnixTime},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{AtLeast32Bit, SaturatedConversion};
    use sp_std::vec::Vec;

    use crate::{Incrementable, TopicDetails, TopicVotingResult};

    const ONE_HOUR: u64 = 60 * 60;
    const THREE_HOURS: u64 = 3 * 60 * 60;
    const THREE_MONTHS: u64 = 3 * 730 * ONE_HOUR;

    type BalanceOf<T, I> =
        <<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        /// A new topic was successfully raised
        TopicRaised { id: T::TopicId, raiser: T::AccountId },

        /// A new voting right token was successfully issued
        VotingRightTokenIssued {
            topic_id: T::TopicId,
            voter: T::AccountId,
            weight_per_required_option: T::Vote,
        },

        /// A new topic was successfully voted
        TopicVoted {
            id: T::TopicId,
            voter: T::AccountId,
            voted_options: BoundedVec<T::OptionIndex, T::TopicOptionMaximumNumber>,
        },
    }

    #[pallet::error]
    pub enum Error<T, I = ()> {
        /// Topic Raiser's balance is insufficient.
        InsufficientBalance,

        /// Contains duplicated option
        DuplicatedOption,

        /// Unknown topic.
        UnknownTopic,

        /// A title is too short.
        TitleTooShort,

        /// A title is too long.
        TitleTooLong,

        /// The title is invalid.
        InvalidTitle,

        /// A description is too long.
        DescriptionTooLong,

        /// The description is invalid.
        InvalidDescription,

        /// A option is too long.
        OptionTooLong,

        /// Topic option is too few.
        OptionTooFew,

        /// Topic option is too many.
        OptionTooMany,

        /// The option is invalid.
        InvalidOption,

        /// The start time of voting period is invalid.
        InvalidVotingPeriodStart,

        /// The end time of voting period is invalid.
        InvalidVotingPeriodEnd,

        /// The answer number is invalid.
        InvalidAnswerNumber,

        /// The weight ratio is invalid.
        InvalidWeightRatio,

        /// Voter has voted on the topic.
        VoterHasVoted,

        /// Voter has no voting right to vote on the topic.
        VoterHasNoVotingRight,

        /// Vote is not open yet.
        VoteNotOpen,

        /// Vote is closed.
        VoteClosed,

        /// A vote is put in the same option many times.
        DuplicatedVoting,

        /// A voting right token has been issued to particular voter.
        VotingRightTokenIssued,
    }

    #[pallet::storage]
    pub type TopicCount<T: Config<I>, I: 'static = ()> = StorageValue<_, T::TopicId, OptionQuery>;

    /// Details of a collection.
    #[pallet::storage]
    #[pallet::getter(fn get_topic)]
    pub type TopicCollection<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Blake2_128Concat,
        T::TopicId,
        TopicDetails<T::AccountId, T::StringLimit, T::TopicOptionMaximumNumber>,
    >;

    /// Status of voting.
    #[pallet::storage]
    #[pallet::getter(fn get_vote_result)]
    pub type BallotBox<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::TopicId,
        Blake2_128Concat,
        T::OptionIndex,
        T::Vote,
    >;

    #[pallet::storage]
    pub type VotingRightTokenCollection<T: Config<I>, I: 'static = ()> =
        StorageDoubleMap<_, Blake2_128Concat, T::TopicId, Blake2_128Concat, T::AccountId, T::Vote>;

    #[pallet::pallet]
    pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

    #[pallet::config]
    pub trait Config<I: 'static = ()>: frame_system::Config {
        type UnixTime: UnixTime;
        type Currency: Currency<Self::AccountId>;

        type RuntimeEvent: From<Event<Self, I>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Identifier for the topic
        type TopicId: Member + Parameter + MaxEncodedLen + Copy + Incrementable;
        type Vote: Member
            + Parameter
            + MaxEncodedLen
            + Copy
            + Incrementable
            + Add
            + AddAssign
            + Ord
            + AtLeast32Bit;
        type OptionIndex: Member
            + Parameter
            + MaxEncodedLen
            + Copy
            + Incrementable
            + Ord
            + AtLeast32Bit
            + From<u64>;

        /// The minimum length of topic title on-chain.
        #[pallet::constant]
        type TopicTitleMinimumLength: Get<u32>;

        /// The maximum length of topic title on-chain.
        #[pallet::constant]
        type TopicTitleMaximumLength: Get<u32>;

        /// The minimum length of topic description on-chain.
        #[pallet::constant]
        type TopicDescriptionMinimumLength: Get<u32>;

        /// The maximum length of topic description on-chain.
        #[pallet::constant]
        type TopicDescriptionMaximumLength: Get<u32>;

        /// The minimum length of topic option on-chain.
        #[pallet::constant]
        type TopicOptionMinimumLength: Get<u32>;

        /// The maximum length of topic option description on-chain.
        #[pallet::constant]
        type TopicOptionMaximumLength: Get<u32>;

        /// The maximum number of topic option description on-chain.
        #[pallet::constant]
        type TopicOptionMaximumNumber: Get<u32>;

        /// The maximum length of data stored on-chain.
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// The threshold of topic raiser.
        #[pallet::constant]
        type TopicRaiserBalanceLowerBound: Get<u128>;

        #[pallet::constant]
        type CurrencyUnits: Get<u128>;
    }

    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        #[pallet::call_index(0)]
        #[pallet::weight(0)]
        pub fn raise_topic(
            origin: OriginFor<T>,
            title: Vec<u8>,
            description: Vec<u8>,
            voting_period_start: u64,
            voting_period_end: u64,
            options: Vec<Vec<u8>>,
            required_answer_number: u32,
        ) -> DispatchResult {
            // Make sure the caller is from a signed origin
            let raiser = frame_system::ensure_signed(origin)?;

            ensure!(
                T::Currency::total_balance(&raiser)
                    >= T::TopicRaiserBalanceLowerBound::get().saturated_into::<BalanceOf::<T, I>>(),
                Error::<T, I>::InsufficientBalance
            );

            {
                let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
                ensure!(
                    voting_period_start >= now + ONE_HOUR
                        && voting_period_start <= now + THREE_MONTHS,
                    Error::<T, I>::InvalidVotingPeriodStart
                );
                ensure!(
                    voting_period_end >= voting_period_start + THREE_HOURS
                        && voting_period_end <= voting_period_start + THREE_MONTHS,
                    Error::<T, I>::InvalidVotingPeriodEnd
                );
            }

            check_title::<T, I>(&title)?;
            check_description::<T, I>(&description)?;
            check_options::<T, I>(&options)?;

            ensure!(
                required_answer_number >= 1
                    && required_answer_number as usize <= (options.len() - 1),
                Error::<T, I>::InvalidAnswerNumber
            );

            let topic_id = TopicCount::<T, I>::get().unwrap_or(T::TopicId::initial_value());
            let topic_details: TopicDetails<
                T::AccountId,
                T::StringLimit,
                T::TopicOptionMaximumNumber,
            > = {
                let mut opts = BoundedVec::default();
                for option in options {
                    opts.try_push(
                        BoundedVec::try_from(option).map_err(|_| Error::<T, I>::OptionTooLong)?,
                    )
                    .map_err(|_| Error::<T, I>::OptionTooMany)?;
                }

                TopicDetails {
                    raiser: raiser.clone(),
                    title: BoundedVec::try_from(title).map_err(|_| Error::<T, I>::TitleTooLong)?,
                    description: BoundedVec::try_from(description)
                        .map_err(|_| Error::<T, I>::DescriptionTooLong)?,
                    voting_period_start,
                    voting_period_end,
                    options: opts,
                    required_answer_number,
                }
            };

            TopicCollection::<T, I>::insert(topic_id, topic_details);

            Self::deposit_event(Event::TopicRaised { id: topic_id, raiser });

            let next_topic_id = topic_id.increment();
            TopicCount::<T, I>::set(Some(next_topic_id));

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(0)]
        pub fn issue_voting_right_token(
            origin: OriginFor<T>,
            topic_id: T::TopicId,
            voters: Vec<T::AccountId>,
            weight_ratio_for_voters: Option<u64>,
        ) -> DispatchResult {
            // Make sure the caller is from a signed origin
            let _raiser = frame_system::ensure_signed(origin)?;

            let weight_ratio_for_voters = weight_ratio_for_voters.unwrap_or(1);
            ensure!(
                weight_ratio_for_voters > 0 && weight_ratio_for_voters <= 10_000,
                Error::<T, I>::InvalidWeightRatio
            );

            let topic_details = if let Some(topic_details) = TopicCollection::<T, I>::get(&topic_id)
            {
                topic_details
            } else {
                return Err(Error::<T, I>::UnknownTopic.into());
            };

            {
                let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
                ensure!(now <= topic_details.voting_period_end, Error::<T, I>::VoteClosed);
            }

            for voter in voters {
                ensure!(
                    !VotingRightTokenCollection::<T, I>::contains_key(topic_id, &voter),
                    Error::<T, I>::VotingRightTokenIssued
                );

                let weight_per_required_option = (T::Currency::total_balance(&voter)
                    * weight_ratio_for_voters.saturated_into::<BalanceOf<T, I>>())
                .saturated_into::<u128>()
                .saturated_into::<T::Vote>();

                if weight_per_required_option > T::Vote::initial_value() {
                    VotingRightTokenCollection::<T, I>::insert(
                        topic_id,
                        &voter,
                        weight_per_required_option,
                    );

                    Self::deposit_event(Event::VotingRightTokenIssued {
                        topic_id,
                        voter,
                        weight_per_required_option,
                    });
                }
            }
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(0)]
        pub fn vote_topic(
            origin: OriginFor<T>,
            topic_id: T::TopicId,
            options: BoundedVec<T::OptionIndex, T::TopicOptionMaximumNumber>,
        ) -> DispatchResult {
            let topic_details = if let Some(topic_details) = TopicCollection::<T, I>::get(&topic_id)
            {
                topic_details
            } else {
                return Err(Error::<T, I>::UnknownTopic.into());
            };

            {
                let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
                ensure!(topic_details.voting_period_start <= now, Error::<T, I>::VoteNotOpen);
                ensure!(now <= topic_details.voting_period_end, Error::<T, I>::VoteClosed);
            }

            // Make sure the caller is from a signed origin
            let voter = frame_system::ensure_signed(origin)?;

            let vote_weight = if let Some(vote_weight) =
                VotingRightTokenCollection::<T, I>::get(topic_id, &voter)
            {
                ensure!(vote_weight > T::Vote::initial_value(), Error::<T, I>::VoterHasVoted);
                vote_weight
            } else {
                return Err(Error::<T, I>::VoterHasNoVotingRight.into());
            };

            {
                let mut options = options.clone().into_inner();
                options.sort_unstable();
                let origin_len = options.len();
                options.dedup();
                ensure!(origin_len == options.len(), Error::<T, I>::DuplicatedVoting);
                ensure!(
                    options.len() == topic_details.required_answer_number as usize,
                    Error::<T, I>::InvalidAnswerNumber
                );
                ensure!(
                    options.last().expect("options is not empty")
                        <= &T::OptionIndex::from((topic_details.options.len() as u64) - 1),
                    Error::<T, I>::InvalidOption
                );

                for opt in options.iter() {
                    let mut vote_count = BallotBox::<T, I>::get(topic_id, opt)
                        .unwrap_or_else(T::Vote::initial_value);
                    vote_count += vote_weight;
                    BallotBox::<T, I>::insert(topic_id, opt, vote_count);
                }
            }

            VotingRightTokenCollection::<T, I>::insert(
                topic_id,
                voter.clone(),
                T::Vote::initial_value(),
            );

            Self::deposit_event(Event::<T, I>::TopicVoted {
                id: topic_id,
                voted_options: options,
                voter,
            });

            Ok(())
        }
    }

    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        pub fn get_topic_by_id(
            topic_id: T::TopicId,
        ) -> Option<TopicDetails<T::AccountId, T::StringLimit, T::TopicOptionMaximumNumber>>
        {
            TopicCollection::<T, I>::get(&topic_id)
        }

        pub fn get_topic_votes_result_by_id(
            topic_id: T::TopicId,
        ) -> Vec<TopicVotingResult<T::OptionIndex, T::Vote>> {
            BallotBox::<T, I>::iter_prefix(topic_id)
                .map(|(index, vote_weight)| TopicVotingResult { index, vote_weight })
                .collect()
        }
    }

    fn check_title<T: Config<I>, I: 'static>(title: &[u8]) -> DispatchResult {
        let title_str =
            simdutf8::basic::from_utf8(title).map_err(|_| Error::<T, I>::InvalidTitle)?;
        let len = title_str.chars().count() as u32;
        ensure!(
            len >= T::TopicTitleMinimumLength::get() && len <= T::TopicTitleMaximumLength::get(),
            Error::<T, I>::InvalidTitle
        );
        Ok(())
    }

    fn check_description<T: Config<I>, I: 'static>(desc: &[u8]) -> DispatchResult {
        let desc_str =
            simdutf8::basic::from_utf8(desc).map_err(|_| Error::<T, I>::InvalidDescription)?;
        let len = desc_str.chars().count() as u32;
        ensure!(
            len >= T::TopicDescriptionMinimumLength::get()
                && len <= T::TopicDescriptionMaximumLength::get(),
            Error::<T, I>::InvalidDescription
        );
        Ok(())
    }

    fn check_options<T: Config<I>, I: 'static>(options: &[Vec<u8>]) -> DispatchResult {
        {
            let mut sorted = options.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            ensure!(sorted.len() == options.len(), Error::<T, I>::DuplicatedOption);
        }

        ensure!(options.len() >= 2, Error::<T, I>::OptionTooFew);
        ensure!(
            options.len() as u32 <= T::TopicOptionMaximumNumber::get(),
            Error::<T, I>::OptionTooMany
        );

        for option in options {
            let option_str =
                simdutf8::basic::from_utf8(option).map_err(|_| Error::<T, I>::InvalidOption)?;
            let len = option_str.len() as u32;
            ensure!(
                len >= T::TopicOptionMinimumLength::get()
                    && len <= T::TopicOptionMaximumLength::get(),
                Error::<T, I>::InvalidOption
            );
        }

        Ok(())
    }
}
