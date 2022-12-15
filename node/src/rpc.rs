// Copyright 2021 Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Parachain-specific RPCs implementation.

#![warn(missing_docs)]

use std::sync::Arc;

use parachains_common::{
    rmrk_core, rmrk_equip, uniques, AccountId, Balance, Block, Index as Nonce,
};
use rmrk_traits::primitives::{CollectionId, NftId, PartId};
use rmrk_traits::{
    BaseInfo, CollectionInfo, NftInfo, PartType, PropertyInfo, ResourceInfo, Theme, ThemeProperty,
};
use sc_client_api::{backend, AuxStore, Backend, BlockBackend, StorageProvider};
pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};
use sc_transaction_pool_api::TransactionPool;
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_runtime::{BoundedVec, Permill};

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies
pub struct FullDeps<C, B, P> {
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Backend
    pub backend: Arc<B>,
    /// The client enabled archive mode
    pub archive_enabled: bool,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, B, P>(
    deps: FullDeps<C, B, P>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>
        + StorageProvider<Block, B>
        + HeaderBackend<Block>
        + BlockBackend<Block>
        + AuxStore
        + HeaderMetadata<Block, Error = BlockChainError>
        + Send
        + Sync
        + 'static,
    C::Api: frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: BlockBuilder<Block>,
    C::Api:
        sp_api::Metadata<Block> + ApiExt<Block, StateBackend = backend::StateBackendFor<B, Block>>,
    C::Api: pallet_rmrk_rpc_runtime_api::RmrkApi<
        Block,
        AccountId,
        CollectionInfo<
            BoundedVec<u8, uniques::StringLimit>,
            BoundedVec<u8, rmrk_core::CollectionSymbolLimit>,
            AccountId,
        >,
        NftInfo<AccountId, Permill, BoundedVec<u8, uniques::StringLimit>, CollectionId, NftId>,
        ResourceInfo<
            BoundedVec<u8, uniques::StringLimit>,
            BoundedVec<PartId, rmrk_core::PartsLimit>,
        >,
        PropertyInfo<BoundedVec<u8, uniques::KeyLimit>, BoundedVec<u8, uniques::ValueLimit>>,
        BaseInfo<AccountId, BoundedVec<u8, uniques::StringLimit>>,
        PartType<
            BoundedVec<u8, uniques::StringLimit>,
            BoundedVec<CollectionId, rmrk_equip::MaxCollectionsEquippablePerPart>,
        >,
        Theme<
            BoundedVec<u8, uniques::StringLimit>,
            BoundedVec<
                ThemeProperty<BoundedVec<u8, uniques::StringLimit>>,
                rmrk_equip::MaxPropertiesPerTheme,
            >,
        >,
    >,
    C::Api: pallet_mq_runtime_api::MqApi<Block>,
    B: Backend<Block> + 'static,
    P: TransactionPool + Sync + Send + 'static,
{
    use frame_rpc_system::{System, SystemApiServer};
    use pallet_rmrk_rpc::{Rmrk, RmrkApiServer};
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};

    let mut module = RpcExtension::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        backend,
        archive_enabled,
    } = deps;

    module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
    module.merge(TransactionPayment::new(client.clone()).into_rpc())?;
    module.merge(Rmrk::new(client.clone()).into_rpc())?;

    phala_node_rpc_ext::extend_rpc(
        &mut module,
        client.clone(),
        backend.clone(),
        archive_enabled,
        pool.clone(),
    );

    Ok(module)
}

// TODO: Remove when Phala integrated Phala pallets
#[allow(dead_code)]
pub fn create_phala_full<C, B, P>(
    deps: FullDeps<C, B, P>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>
        + HeaderBackend<Block>
        + AuxStore
        + HeaderMetadata<Block, Error = BlockChainError>
        + Send
        + Sync
        + 'static,
    C::Api: frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: BlockBuilder<Block>,
    P: TransactionPool + Sync + Send + 'static,
{
    use frame_rpc_system::{System, SystemApiServer};
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};

    let mut module = RpcExtension::new(());
    let FullDeps {
        client,
        pool,
        deny_unsafe,
        backend: _,
        archive_enabled: _,
    } = deps;

    module.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;
    module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

    Ok(module)
}
