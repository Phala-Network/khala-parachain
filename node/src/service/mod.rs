// Copyright 2019-2021 Parity Technologies (UK) Ltd.
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
use std::{sync::Arc, time::Duration};

use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_common::ParachainBlockImport as TParachainBlockImport;
use cumulus_client_service::{
    build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
    BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};

use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_consensus::ImportQueue;
use sc_network::{config::FullNetworkConfiguration, NetworkBlock};
use sc_network_sync::SyncingService;
use sc_service::{
    Configuration, PartialComponents, PruningMode, TFullBackend, TFullClient, TaskManager,
};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_keystore::KeystorePtr;
use substrate_prometheus_endpoint::Registry;

use rmrk_traits::primitives::{CollectionId, NftId, PartId};
use rmrk_traits::{
    BaseInfo, CollectionInfo, NftInfo, PartType, PropertyInfo, ResourceInfo, Theme, ThemeProperty,
};
use sp_runtime::{BoundedVec, Permill};

use polkadot_primitives::CollatorPair;

use parachains_common::{rmrk_core, rmrk_equip, uniques};

pub use parachains_common::{AccountId, Balance, Block, Hash, Header, Nonce};

#[cfg(feature = "khala-native")]
pub mod khala;
#[cfg(feature = "phala-native")]
pub mod phala;
#[cfg(feature = "rhala-native")]
pub mod rhala;
#[cfg(feature = "shell-native")]
pub mod shell;
#[cfg(feature = "thala-native")]
pub mod thala;

#[cfg(not(feature = "runtime-benchmarks"))]
type HostFunctions = sp_io::SubstrateHostFunctions;

#[cfg(feature = "runtime-benchmarks")]
type HostFunctions = (
    sp_io::SubstrateHostFunctions,
    frame_benchmarking::benchmarking::HostFunctions,
);

pub(crate) type ParachainClient<RuntimeApi> = TFullClient<Block, RuntimeApi, WasmExecutor<HostFunctions>>;
pub(crate) type ParachainBackend = TFullBackend<Block>;
pub(crate) type ParachainBlockImport<RuntimeApi> = TParachainBlockImport<Block, Arc<ParachainClient<RuntimeApi>>, ParachainBackend>;

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
#[allow(clippy::type_complexity)]
pub fn new_partial<RuntimeApi, BIQ>(
    config: &Configuration,
    build_import_queue: BIQ,
) -> Result<
    PartialComponents<
        ParachainClient<RuntimeApi>,
        ParachainBackend,
        (),
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::FullPool<Block, ParachainClient<RuntimeApi>>,
        (ParachainBlockImport<RuntimeApi>, Option<Telemetry>, Option<TelemetryWorkerHandle>),
    >,
    sc_service::Error,
>
where
    RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>,
    BIQ: FnOnce(
        Arc<ParachainClient<RuntimeApi>>,
        ParachainBlockImport<RuntimeApi>,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<
        sc_consensus::DefaultImportQueue<Block>,
        sc_service::Error,
    >,
{
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let heap_pages = config
        .default_heap_pages
        .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static { extra_pages: h as _ });

    let executor = sc_executor::WasmExecutor::<HostFunctions>::builder()
        .with_execution_method(config.wasm_method)
        .with_max_runtime_instances(config.max_runtime_instances)
        .with_runtime_cache_size(config.runtime_cache_size)
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        .build();

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;
    let client = Arc::new(client);

    let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    sc_storage_monitor::StorageMonitorService::try_spawn(
        sc_storage_monitor::StorageMonitorParams {
            threshold: 1024,
            polling_period: 12,
        },
        config.database.clone(),
        &task_manager.spawn_essential_handle(),
    )
    .map_err(|e| sc_service::Error::Application(e.into()))?;

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let block_import = ParachainBlockImport::new(client.clone(), backend.clone());

    let import_queue = build_import_queue(
        client.clone(),
        block_import.clone(),
        config,
        telemetry.as_ref().map(|telemetry| telemetry.handle()),
        &task_manager,
    )?;

    let params = PartialComponents {
        backend,
        client,
        import_queue,
        keystore_container,
        task_manager,
        transaction_pool,
        select_chain: (),
        other: (block_import, telemetry, telemetry_worker_handle),
    };

    Ok(params)
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RuntimeApi, RB, BIQ, SC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    sybil_resistance_level: CollatorSybilResistance,
    para_id: ParaId,
    _rpc_ext_builder: RB,
    build_import_queue: BIQ,
    start_consensus: SC,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient<RuntimeApi>>)>
where
    RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
    RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
        + pallet_rmrk_rpc_runtime_api::RmrkApi<
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
        >
        + pallet_mq_runtime_api::MqApi<Block>
        + sygma_runtime_api::SygmaBridgeApi<Block>,
    RB: Fn(Arc<ParachainClient<RuntimeApi>>) -> Result<jsonrpsee::RpcModule<()>, sc_service::Error>,
    BIQ: FnOnce(
        Arc<ParachainClient<RuntimeApi>>,
        ParachainBlockImport<RuntimeApi>,
        &Configuration,
        Option<TelemetryHandle>,
        &TaskManager,
    ) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>,
    SC: FnOnce(
        Arc<ParachainClient<RuntimeApi>>,
        ParachainBlockImport<RuntimeApi>,
        Option<&Registry>,
        Option<TelemetryHandle>,
        &TaskManager,
        Arc<dyn RelayChainInterface>,
        Arc<sc_transaction_pool::FullPool<Block, ParachainClient<RuntimeApi>>>,
        Arc<SyncingService<Block>>,
        KeystorePtr,
        Duration,
        ParaId,
        CollatorPair,
        OverseerHandle,
        Arc<dyn Fn(Hash, Option<Vec<u8>>) + Send + Sync>,
    ) -> Result<(), sc_service::Error>,
{
    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial::<RuntimeApi, BIQ>(&parachain_config, build_import_queue)?;
    let (block_import, mut telemetry, telemetry_worker_handle) = params.other;

    let client = params.client.clone();
    let backend = params.backend.clone();

    let mut task_manager = params.task_manager;
    let (relay_chain_interface, collator_key) = build_relay_chain_interface(
        polkadot_config,
        &parachain_config,
        telemetry_worker_handle,
        &mut task_manager,
        collator_options.clone(),
        hwbench.clone(),
    )
    .await
    .map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

    let validator = parachain_config.role.is_authority();
    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let import_queue_service = params.import_queue.service();
    let net_config = FullNetworkConfiguration::new(&parachain_config.network);

    let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
        build_network(BuildNetworkParams {
            parachain_config: &parachain_config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            para_id,
            spawn_handle: task_manager.spawn_handle(),
            relay_chain_interface: relay_chain_interface.clone(),
            import_queue: params.import_queue,
            sybil_resistance_level,
        }).await?;

    let rpc_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();
        let backend = backend.clone();
        let archive_enabled = match &parachain_config.state_pruning {
            Some(m) => match m {
                PruningMode::Constrained(_) => false,
                PruningMode::ArchiveAll | PruningMode::ArchiveCanonical => true,
            },
            None => true,
        };

        Box::new(move |deny_unsafe, _| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                backend: backend.clone(),
                archive_enabled,
                deny_unsafe,
            };

            crate::rpc::create_full(deps).map_err(Into::into)
        })
    };

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.keystore(),
        backend: backend.clone(),
        network: network.clone(),
        sync_service: sync_service.clone(),
        system_rpc_tx,
        tx_handler_controller,
        telemetry: telemetry.as_mut(),
    })?;

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);

        if let Some(ref mut telemetry) = telemetry {
            let telemetry_handle = telemetry.handle();
            task_manager.spawn_handle().spawn(
                "telemetry_hwbench",
                None,
                sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
            );
        }
    }

    let announce_block = {
        let sync_service = sync_service.clone();
        Arc::new(move |hash, data| sync_service.announce_block(hash, data))
    };

    let relay_chain_slot_duration = Duration::from_secs(6);

    let overseer_handle = relay_chain_interface
        .overseer_handle()
        .map_err(|e| sc_service::Error::Application(Box::new(e)))?;

    start_relay_chain_tasks(StartRelayChainTasksParams {
        client: client.clone(),
        announce_block: announce_block.clone(),
        para_id,
        relay_chain_interface: relay_chain_interface.clone(),
        task_manager: &mut task_manager,
        da_recovery_profile: if validator {
            DARecoveryProfile::Collator
        } else {
            DARecoveryProfile::FullNode
        },
        import_queue: import_queue_service,
        relay_chain_slot_duration,
        recovery_handle: Box::new(overseer_handle.clone()),
        sync_service: sync_service.clone(),
    })?;

    if validator {
        start_consensus(
            client.clone(),
            block_import,
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
            sync_service.clone(),
            params.keystore_container.keystore(),
            relay_chain_slot_duration,
            para_id,
            collator_key.expect("Command line arguments do not allow this. qed"),
            overseer_handle,
            announce_block,
        )?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}
