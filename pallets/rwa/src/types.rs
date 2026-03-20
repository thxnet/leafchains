use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{pallet_prelude::*, BoundedVec};
use scale_info::TypeInfo;
use sp_runtime::{DispatchResult, Permill, RuntimeDebug};

/// Hook for KYC / whitelist eligibility checks before participation.
pub trait ParticipationFilter<AccountId> {
    fn ensure_eligible(rwa_asset_id: u32, who: &AccountId) -> DispatchResult;
}

/// Blanket pass-through: no filtering.
impl<AccountId> ParticipationFilter<AccountId> for () {
    fn ensure_eligible(_: u32, _: &AccountId) -> DispatchResult { Ok(()) }
}

/// Hook for checking whether an RWA asset or participation can be
/// retired/slashed.  Implemented at the runtime level to enable
/// cross-pallet awareness (e.g., crowdfunding campaigns linked to
/// this asset).
pub trait AssetLifecycleGuard<AccountId> {
    /// Called before `force_retire_asset`.  Return `Err` to block retirement.
    fn can_retire_asset(_rwa_asset_id: u32) -> DispatchResult { Ok(()) }

    /// Called before `slash_participation`.  Return `Err` to block slashing.
    fn can_slash_participation(_rwa_asset_id: u32, _participation_id: u32) -> DispatchResult {
        Ok(())
    }
}

/// Blanket no-op: no guard.
impl<AccountId> AssetLifecycleGuard<AccountId> for () {}

/// Payment currency: native token or a specific fungible asset.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum PaymentCurrency<AssetId> {
    #[codec(index = 0)]
    Native,
    #[codec(index = 1)]
    Asset(AssetId),
}

/// Asset lifecycle status.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum AssetStatus<BlockNumber> {
    #[codec(index = 0)]
    Active,
    #[codec(index = 1)]
    Inactive,
    #[codec(index = 2)]
    Sunsetting { expiry_block: BlockNumber },
    #[codec(index = 3)]
    Retired,
    #[codec(index = 4)]
    Paused,
}

/// Per-asset participation policy, defined at asset creation.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AssetPolicy<Balance, BlockNumber, AssetId> {
    /// Currency for deposit and entry_fee (native or fungible asset).
    pub deposit_currency: PaymentCurrency<AssetId>,
    /// Non-refundable entry fee transferred to beneficiary on approval.
    pub entry_fee: Balance,
    /// Refundable deposit locked in escrow during participation.
    pub deposit: Balance,
    /// Maximum participation duration in blocks (None = unlimited).
    pub max_duration: Option<BlockNumber>,
    /// Maximum number of active participants (None = unlimited).
    pub max_participants: Option<u32>,
    /// Whether new participations require owner approval.
    pub requires_approval: bool,
}

/// Full RWA asset record.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxMetadataLen))]
pub struct AssetInfo<AccountId, Balance, BlockNumber, AssetId, MaxMetadataLen: Get<u32>> {
    pub owner: AccountId,
    pub beneficiary: AccountId,
    pub status: AssetStatus<BlockNumber>,
    pub policy: AssetPolicy<Balance, BlockNumber, AssetId>,
    pub metadata: BoundedVec<u8, MaxMetadataLen>,
    pub participant_count: u32,
    pub registration_deposit: Balance,
    pub created_at: BlockNumber,
}

/// Participation lifecycle status.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum ParticipationStatus<BlockNumber> {
    #[codec(index = 0)]
    PendingApproval,
    #[codec(index = 1)]
    Active { started_at: BlockNumber, expires_at: Option<BlockNumber> },
    #[codec(index = 2)]
    Expired,
    #[codec(index = 3)]
    Exited,
    #[codec(index = 4)]
    Slashed,
    #[codec(index = 5)]
    Revoked,
}

/// A participation record. Solo (1 holder) or group (multiple holders).
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxGroupSize))]
pub struct Participation<AccountId, Balance, BlockNumber, MaxGroupSize: Get<u32>> {
    pub rwa_asset_id: u32,
    /// The account that paid the deposit + entry_fee (receives refunds).
    pub payer: AccountId,
    /// Accounts with usage rights.
    pub holders: BoundedVec<AccountId, MaxGroupSize>,
    pub status: ParticipationStatus<BlockNumber>,
    pub deposit_held: Balance,
    pub entry_fee_paid: Balance,
}

/// Kind of slash recipient.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum SlashRecipientKind<AccountId> {
    /// The asset's beneficiary (dynamic, resolved at slash time).
    #[codec(index = 0)]
    Beneficiary,
    /// The reporter passed at slash time (fallback to Beneficiary if None).
    #[codec(index = 1)]
    Reporter,
    /// A fixed account (e.g. treasury).
    #[codec(index = 2)]
    Account(AccountId),
    /// Burn (destroy tokens).
    #[codec(index = 3)]
    Burn,
}

/// One entry in a per-asset slash distribution.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct SlashRecipient<AccountId> {
    pub kind: SlashRecipientKind<AccountId>,
    pub share: Permill,
}

/// Error type for the `can_participate` runtime API.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum CanParticipateError {
    #[codec(index = 0)]
    AssetNotFound,
    #[codec(index = 1)]
    AssetNotActive,
    #[codec(index = 2)]
    MaxParticipantsReached,
    #[codec(index = 3)]
    AlreadyParticipating,
    #[codec(index = 4)]
    NotEligible,
}
