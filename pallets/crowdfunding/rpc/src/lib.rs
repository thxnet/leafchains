use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
};
use pallet_crowdfunding::{CampaignSummary, EligibilityError, Investment, WithdrawalPreview};
use pallet_crowdfunding_runtime_api::CrowdfundingApi as CrowdfundingRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

#[rpc(client, server)]
pub trait CrowdfundingRpc<BlockHash, AccountId, Balance, BlockNumber, AssetId> {
    #[method(name = "crowdfunding_checkEligibility")]
    fn check_eligibility(
        &self,
        campaign_id: u32,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> RpcResult<Result<(), EligibilityError>>;

    #[method(name = "crowdfunding_previewWithdrawal")]
    fn preview_withdrawal(
        &self,
        campaign_id: u32,
        investor: AccountId,
        amount: Balance,
        at: Option<BlockHash>,
    ) -> RpcResult<Option<WithdrawalPreview<Balance>>>;

    #[method(name = "crowdfunding_campaignSummary")]
    fn campaign_summary(
        &self,
        campaign_id: u32,
        at: Option<BlockHash>,
    ) -> RpcResult<Option<CampaignSummary<Balance, BlockNumber>>>;

    #[method(name = "crowdfunding_campaignsByCreator")]
    fn campaigns_by_creator(
        &self,
        creator: AccountId,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<u32>>;

    #[method(name = "crowdfunding_campaignsByInvestor")]
    fn campaigns_by_investor(
        &self,
        investor: AccountId,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<u32>>;

    #[method(name = "crowdfunding_getInvestment")]
    fn get_investment(
        &self,
        campaign_id: u32,
        investor: AccountId,
        at: Option<BlockHash>,
    ) -> RpcResult<Option<Investment<Balance>>>;

    #[method(name = "crowdfunding_getProtocolConfig")]
    fn get_protocol_config(&self, at: Option<BlockHash>) -> RpcResult<(u16, AccountId)>;
}

pub struct CrowdfundingRpcHandler<C, Block> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<Block>,
}

impl<C, Block> CrowdfundingRpcHandler<C, Block> {
    pub fn new(client: Arc<C>) -> Self { Self { client, _marker: Default::default() } }
}

/// L-7: map a runtime API error to an RPC error, logging the debug format
/// internally so callers never see internal error details.
fn runtime_api_error(e: impl std::fmt::Debug) -> jsonrpsee::core::Error {
    log::error!(target: "crowdfunding-rpc", "Runtime API error: {:?}", e);
    jsonrpsee::core::Error::Custom("Runtime API call failed".to_string())
}

#[async_trait]
impl<C, Block, AccountId, Balance, BlockNumber, AssetId>
    CrowdfundingRpcServer<<Block as BlockT>::Hash, AccountId, Balance, BlockNumber, AssetId>
    for CrowdfundingRpcHandler<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: CrowdfundingRuntimeApi<Block, AccountId, Balance, BlockNumber, AssetId>,
    AccountId: Codec + Send + Sync + 'static,
    Balance: Codec + Send + Sync + 'static,
    BlockNumber: Codec + Send + Sync + 'static,
    AssetId: Codec + Send + Sync + 'static,
{
    fn check_eligibility(
        &self,
        campaign_id: u32,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Result<(), EligibilityError>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.check_eligibility(at_hash, campaign_id, who).map_err(runtime_api_error)
    }

    fn preview_withdrawal(
        &self,
        campaign_id: u32,
        investor: AccountId,
        amount: Balance,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Option<WithdrawalPreview<Balance>>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.preview_withdrawal(at_hash, campaign_id, investor, amount).map_err(runtime_api_error)
    }

    fn campaign_summary(
        &self,
        campaign_id: u32,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Option<CampaignSummary<Balance, BlockNumber>>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.campaign_summary(at_hash, campaign_id).map_err(runtime_api_error)
    }

    fn campaigns_by_creator(
        &self,
        creator: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u32>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.campaigns_by_creator(at_hash, creator).map_err(runtime_api_error)
    }

    fn campaigns_by_investor(
        &self,
        investor: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u32>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.campaigns_by_investor(at_hash, investor).map_err(runtime_api_error)
    }

    fn get_investment(
        &self,
        campaign_id: u32,
        investor: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Option<Investment<Balance>>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.get_investment(at_hash, campaign_id, investor).map_err(runtime_api_error)
    }

    fn get_protocol_config(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<(u16, AccountId)> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.get_protocol_config(at_hash).map_err(runtime_api_error)
    }
}
