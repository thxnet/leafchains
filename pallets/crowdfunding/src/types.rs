use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
    pallet_prelude::*, BoundedVec, CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound,
};
use scale_info::TypeInfo;
use sp_runtime::{DispatchResult, RuntimeDebug};

/// Hook for verifying that a campaign creator holds an active RWA license
/// (participation) before campaign creation and fund claiming.
///
/// Follows the same pattern as `pallet_rwa::ParticipationFilter`.
pub trait LicenseVerifier<AccountId, BlockNumber> {
    /// Verify that `who` holds an active license/participation for the given
    /// RWA asset and participation IDs.  Also checks that the underlying
    /// asset is in an Active state (V1 fix).
    fn ensure_active_license(
        rwa_asset_id: u32,
        participation_id: u32,
        who: &AccountId,
    ) -> DispatchResult;

    /// Check whether a license is still active (used for ongoing campaign
    /// checks at claim time and by the permissionless
    /// `report_license_revoked`).  Also checks asset status (V1 fix).
    fn is_license_active(_rwa_asset_id: u32, _participation_id: u32) -> bool { true }

    /// Returns the block at which the license expires, or `None` if the
    /// license has no expiry (unlimited duration).  Used by
    /// `create_campaign` to ensure the campaign deadline does not exceed
    /// the license expiry (V2 fix).
    fn license_expiry(_rwa_asset_id: u32, _participation_id: u32) -> Option<BlockNumber> { None }
}

/// Blanket no-op: no license required.
impl<AccountId, BlockNumber> LicenseVerifier<AccountId, BlockNumber> for () {
    fn ensure_active_license(_: u32, _: u32, _: &AccountId) -> DispatchResult { Ok(()) }
}

/// Payment currency: native token or a specific fungible asset.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum PaymentCurrency<AssetId> {
    #[codec(index = 0)]
    Native,
    #[codec(index = 1)]
    Asset(AssetId),
}

/// A single milestone definition.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Milestone {
    pub release_bps: u16,
    pub description_hash: [u8; 32],
}

/// Funding model for a campaign.
#[derive(
    CloneNoBound,
    PartialEqNoBound,
    EqNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxMilestones))]
pub enum FundingModel<Balance: Clone + PartialEq + Eq + sp_std::fmt::Debug, MaxMilestones: Get<u32>>
{
    #[codec(index = 0)]
    AllOrNothing { goal: Balance },
    #[codec(index = 1)]
    KeepWhatYouRaise { soft_cap: Option<Balance> },
    #[codec(index = 2)]
    MilestoneBased { goal: Balance, milestones: BoundedVec<Milestone, MaxMilestones> },
}

/// Per-campaign configuration.
#[derive(
    CloneNoBound,
    PartialEqNoBound,
    EqNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxMilestones))]
pub struct CampaignConfig<
    Balance: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    BlockNumber: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    AssetId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    MaxMilestones: Get<u32>,
> {
    pub funding_model: FundingModel<Balance, MaxMilestones>,
    pub funding_currency: PaymentCurrency<AssetId>,
    pub deadline: BlockNumber,
    pub hard_cap: Option<Balance>,
    pub min_investment: Option<Balance>,
    pub max_investment_per_investor: Option<Balance>,
    /// SHA-256 digest or IPFS CIDv1 hash of the off-chain campaign metadata.
    pub metadata_hash: [u8; 32],
    pub early_withdrawal_penalty_bps: Option<u16>,
}

/// Eligibility rule for investors.
#[derive(
    CloneNoBound,
    PartialEqNoBound,
    EqNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxNftSets, MaxNftsPerSet))]
pub enum EligibilityRule<
    AssetId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    Balance: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    CollectionId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    ItemId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    MaxNftSets: Get<u32>,
    MaxNftsPerSet: Get<u32>,
> {
    #[codec(index = 0)]
    NativeBalance { min_balance: Balance },
    #[codec(index = 1)]
    AssetBalance { asset_id: AssetId, min_balance: Balance },
    #[codec(index = 2)]
    NftOwnership {
        required_sets: BoundedVec<BoundedVec<(CollectionId, ItemId), MaxNftsPerSet>, MaxNftSets>,
    },
    #[codec(index = 3)]
    AccountWhitelist,
}

/// Campaign lifecycle status.
#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum CampaignStatus {
    #[codec(index = 0)]
    Funding,
    #[codec(index = 1)]
    Succeeded,
    #[codec(index = 2)]
    Failed,
    #[codec(index = 3)]
    MilestonePhase,
    #[codec(index = 4)]
    Completed,
    #[codec(index = 5)]
    Cancelled,
    #[codec(index = 6)]
    Paused,
}

/// Status of a single milestone.
#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum MilestoneStatus {
    #[codec(index = 0)]
    Pending,
    #[codec(index = 1)]
    Submitted,
    #[codec(index = 2)]
    Approved,
    #[codec(index = 3)]
    Rejected,
    #[codec(index = 4)]
    Claimed,
}

/// Full campaign record.
#[derive(
    CloneNoBound,
    PartialEqNoBound,
    EqNoBound,
    Encode,
    Decode,
    RuntimeDebugNoBound,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(MaxMilestones, MaxEligibilityRules, MaxNftSets, MaxNftsPerSet))]
pub struct Campaign<
    AccountId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    Balance: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    BlockNumber: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    AssetId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    CollectionId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    ItemId: Clone + PartialEq + Eq + sp_std::fmt::Debug,
    MaxMilestones: Get<u32>,
    MaxEligibilityRules: Get<u32>,
    MaxNftSets: Get<u32>,
    MaxNftsPerSet: Get<u32>,
> {
    pub creator: AccountId,
    pub status: CampaignStatus,
    pub config: CampaignConfig<Balance, BlockNumber, AssetId, MaxMilestones>,
    pub eligibility_rules: BoundedVec<
        EligibilityRule<AssetId, Balance, CollectionId, ItemId, MaxNftSets, MaxNftsPerSet>,
        MaxEligibilityRules,
    >,
    pub total_raised: Balance,
    pub total_disbursed: Balance,
    pub investor_count: u32,
    pub creation_deposit: Balance,
    pub created_at: BlockNumber,
    /// Set when the campaign is paused; used to extend the deadline on resume
    /// so the effective funding window is not shortened by the pause duration.
    pub paused_at: Option<BlockNumber>,
    /// RWA asset ID linked to this campaign's license (`None` = no license
    /// required).
    pub rwa_asset_id: Option<u32>,
    /// Participation ID within the RWA asset (`None` = no license required).
    pub participation_id: Option<u32>,
    /// CAT-3.9-C-I: protocol fee in basis points, locked at campaign creation.
    /// Prevents retroactive fee changes from affecting existing campaigns.
    pub protocol_fee_bps: u16,
}

/// Individual investor's record for a campaign.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Investment<Balance> {
    pub total_invested: Balance,
    pub total_withdrawn: Balance,
}

/// Preview of a withdrawal (returned by runtime API).
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawalPreview<Balance> {
    pub gross_amount: Balance,
    pub penalty: Balance,
    pub net_amount: Balance,
    pub penalty_bps: u16,
}

/// Summary of a campaign (returned by runtime API).
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct CampaignSummary<Balance, BlockNumber> {
    pub status: CampaignStatus,
    pub total_raised: Balance,
    pub goal: Option<Balance>,
    pub hard_cap: Option<Balance>,
    pub investor_count: u32,
    pub remaining_blocks: Option<BlockNumber>,
    /// Funding percentage in parts-per-million (0 = 0%, 1_000_000 = 100%).
    pub funding_percentage_ppm: u32,
    pub milestones_completed: Option<u8>,
    pub milestones_total: Option<u8>,
    pub rwa_asset_id: Option<u32>,
    pub participation_id: Option<u32>,
}

/// Error type for check_eligibility runtime API.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum EligibilityError {
    #[codec(index = 0)]
    CampaignNotFound,
    #[codec(index = 1)]
    InsufficientNativeBalance,
    #[codec(index = 2)]
    InsufficientAssetBalance,
    #[codec(index = 3)]
    NftOwnershipNotMet,
    #[codec(index = 4)]
    NotWhitelisted,
}
