#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use pallet_crowdfunding::{CampaignSummary, EligibilityError, Investment, WithdrawalPreview};
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait CrowdfundingApi<AccountId, Balance, BlockNumber, AssetId>
    where
        AccountId: Codec,
        Balance: Codec,
        BlockNumber: Codec,
        AssetId: Codec,
    {
        fn check_eligibility(
            campaign_id: u32,
            who: AccountId,
        ) -> Result<(), EligibilityError>;

        fn preview_withdrawal(
            campaign_id: u32,
            investor: AccountId,
            amount: Balance,
        ) -> Option<WithdrawalPreview<Balance>>;

        fn campaign_summary(campaign_id: u32) -> Option<CampaignSummary<Balance, BlockNumber>>;

        fn campaigns_by_creator(creator: AccountId) -> Vec<u32>;

        fn campaigns_by_investor(investor: AccountId) -> Vec<u32>;

        fn get_investment(campaign_id: u32, investor: AccountId) -> Option<Investment<Balance>>;

        fn get_protocol_config() -> (u16, AccountId);
    }
}
