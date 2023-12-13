use codec::{Decode, Encode};
use frame_support::{pallet_prelude::*, sp_runtime::Saturating};

use crate::macros::impl_incrementable;

pub trait Incrementable {
    fn increment(&self) -> Self;

    fn initial_value() -> Self;
}

impl_incrementable!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

#[derive(Clone, Encode, Decode, Eq, PartialEqNoBound, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(StringLimit, OptionLimit))]
pub struct TopicDetails<AccountId: PartialEq, StringLimit: Get<u32>, OptionLimit: Get<u32>> {
    pub(super) raiser: AccountId,
    pub(super) title: BoundedVec<u8, StringLimit>,
    pub(super) description: BoundedVec<u8, StringLimit>,
    pub(super) voting_period_start: u64,
    pub(super) voting_period_end: u64,
    pub(super) options: BoundedVec<BoundedVec<u8, StringLimit>, OptionLimit>, // options' names
    pub(super) required_answer_number: u32,
}

#[derive(
    Clone, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub struct TopicVotingResult<OptionIndex, VoteWeight> {
    pub index: OptionIndex,
    pub vote_weight: VoteWeight,
}
