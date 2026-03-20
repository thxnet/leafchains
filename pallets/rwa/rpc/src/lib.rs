use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
};
use pallet_rwa::{CanParticipateError, ParticipationStatus};
use pallet_rwa_runtime_api::RwaApi as RwaRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

#[rpc(client, server)]
pub trait RwaRpc<BlockHash, AccountId, Balance, BlockNumber, AssetId> {
    #[method(name = "rwa_effectiveParticipationStatus")]
    fn effective_participation_status(
        &self,
        asset_id: u32,
        participation_id: u32,
        at: Option<BlockHash>,
    ) -> RpcResult<Option<ParticipationStatus<BlockNumber>>>;

    #[method(name = "rwa_canParticipate")]
    fn can_participate(
        &self,
        asset_id: u32,
        who: AccountId,
        at: Option<BlockHash>,
    ) -> RpcResult<Result<(), CanParticipateError>>;

    #[method(name = "rwa_assetsByOwner")]
    fn assets_by_owner(&self, owner: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<u32>>;

    #[method(name = "rwa_participationsByHolder")]
    fn participations_by_holder(
        &self,
        holder: AccountId,
        at: Option<BlockHash>,
    ) -> RpcResult<Vec<(u32, u32)>>;

    #[method(name = "rwa_activeParticipantCount")]
    fn active_participant_count(&self, asset_id: u32, at: Option<BlockHash>) -> RpcResult<u32>;
}

pub struct RwaRpcHandler<C, Block> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<Block>,
}

impl<C, Block> RwaRpcHandler<C, Block> {
    pub fn new(client: Arc<C>) -> Self { Self { client, _marker: Default::default() } }
}

#[async_trait]
impl<C, Block, AccountId, Balance, BlockNumber, AssetId>
    RwaRpcServer<<Block as BlockT>::Hash, AccountId, Balance, BlockNumber, AssetId>
    for RwaRpcHandler<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: RwaRuntimeApi<Block, AccountId, Balance, BlockNumber, AssetId>,
    AccountId: Codec + Send + Sync + 'static,
    Balance: Codec + Send + Sync + 'static,
    BlockNumber: Codec + Send + Sync + 'static,
    AssetId: Codec + Send + Sync + 'static,
{
    fn effective_participation_status(
        &self,
        asset_id: u32,
        participation_id: u32,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Option<ParticipationStatus<BlockNumber>>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.effective_participation_status(at_hash, asset_id, participation_id)
            .map_err(|e| jsonrpsee::core::Error::Custom(format!("{:?}", e)))
    }

    fn can_participate(
        &self,
        asset_id: u32,
        who: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Result<(), CanParticipateError>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.can_participate(at_hash, asset_id, who)
            .map_err(|e| jsonrpsee::core::Error::Custom(format!("{:?}", e)))
    }

    fn assets_by_owner(
        &self,
        owner: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<u32>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.assets_by_owner(at_hash, owner)
            .map_err(|e| jsonrpsee::core::Error::Custom(format!("{:?}", e)))
    }

    fn participations_by_holder(
        &self,
        holder: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<Vec<(u32, u32)>> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.participations_by_holder(at_hash, holder)
            .map_err(|e| jsonrpsee::core::Error::Custom(format!("{:?}", e)))
    }

    fn active_participant_count(
        &self,
        asset_id: u32,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<u32> {
        let api = self.client.runtime_api();
        let at_hash = at.unwrap_or_else(|| self.client.info().best_hash);
        api.active_participant_count(at_hash, asset_id)
            .map_err(|e| jsonrpsee::core::Error::Custom(format!("{:?}", e)))
    }
}
