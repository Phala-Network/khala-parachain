use std::{sync::Arc, time::Duration};

use jsonrpsee::RpcModule;

use cumulus_client_cli::CollatorOptions;
#[allow(deprecated)]
use cumulus_client_service::old_consensus;
use cumulus_client_service::{
    build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
    BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};

pub use parachains_common::{AccountId, Balance, Block, Hash, Header, Nonce};

use sc_consensus::ImportQueue;
use sc_network::{config::FullNetworkConfiguration, NetworkBlock};
use sc_network_sync::SyncingService;
use sc_service::{
    Configuration, TaskManager,
};
use sc_telemetry::TelemetryHandle;
use sp_api::ConstructRuntimeApi;
use sp_keystore::KeystorePtr;
use substrate_prometheus_endpoint::Registry;

use polkadot_primitives::CollatorPair;

use shell_parachain_runtime::RuntimeApi;
use crate::service::{ParachainBlockImport, ParachainClient};

pub struct RuntimeExecutor;

impl sc_executor::NativeExecutionDispatch for RuntimeExecutor {
    type ExtendHostFunctions = ();

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        shell_parachain_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        shell_parachain_runtime::native_version()
    }
}

/// Build the import queue for the shell runtime.
pub fn parachain_build_import_queue<RuntimeApi>(
    client: Arc<ParachainClient<RuntimeApi>>,
    block_import: ParachainBlockImport<RuntimeApi>,
    config: &Configuration,
    _: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<
    sc_consensus::DefaultImportQueue<Block>,
    sc_service::Error,
>
    where
        RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
        RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
            + sp_api::Metadata<Block>
            + sp_session::SessionKeys<Block>
            + sp_api::ApiExt<Block>
            + sp_offchain::OffchainWorkerApi<Block>
            + sp_block_builder::BlockBuilder<Block>
            + sp_offchain::OffchainWorkerApi<Block>
            + sp_block_builder::BlockBuilder<Block>,
{
    cumulus_client_consensus_relay_chain::import_queue(
        client.clone(),
        block_import,
        |_, _| async { Ok(()) },
        &task_manager.spawn_essential_handle(),
        config.prometheus_registry(),
    )
        .map_err(Into::into)
}

/// Start a shell node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api for shell nodes.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RuntimeApi, RB, BIQ, SC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    sybil_resistance_level: CollatorSybilResistance,
    para_id: ParaId,
    rpc_ext_builder: RB,
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
        + cumulus_primitives_core::CollectCollationInfo<Block>,
        RB: Fn(Arc<ParachainClient<RuntimeApi>>) -> Result<jsonrpsee::RpcModule<()>, sc_service::Error>
        + 'static,
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

    let params = crate::service::new_partial::<RuntimeApi, BIQ>(&parachain_config, build_import_queue)?;
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
        })
        .await?;

    let rpc_client = client.clone();
    let rpc_builder = Box::new(move |_, _| rpc_ext_builder(rpc_client.clone()));

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

pub async fn start_parachain_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    para_id: ParaId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient<RuntimeApi>>)> {
    start_node_impl::<RuntimeApi, _, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        CollatorSybilResistance::Unresistant, // free-for-all consensus
        para_id,
        |_| Ok(RpcModule::new(())),
        parachain_build_import_queue,
        |client,
         block_import,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         _sync_oracle,
         _keystore,
         _relay_chain_slot_duration,
         para_id,
         collator_key,
         overseer_handle,
         announce_block| {
            let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
                task_manager.spawn_handle(),
                client.clone(),
                transaction_pool,
                prometheus_registry,
                telemetry,
            );

            let free_for_all = cumulus_client_consensus_relay_chain::build_relay_chain_consensus(
                cumulus_client_consensus_relay_chain::BuildRelayChainConsensusParams {
                    para_id,
                    proposer_factory,
                    block_import,
                    relay_chain_interface: relay_chain_interface.clone(),
                    create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
                        let relay_chain_interface = relay_chain_interface.clone();
                        async move {
                            let parachain_inherent =
                                cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
                                    relay_parent,
                                    &relay_chain_interface,
                                    &validation_data,
                                    para_id,
                                ).await;
                            let parachain_inherent = parachain_inherent.ok_or_else(|| {
                                Box::<dyn std::error::Error + Send + Sync>::from(
                                    "Failed to create parachain inherent",
                                )
                            })?;
                            Ok(parachain_inherent)
                        }
                    },
                },
            );

            let spawner = task_manager.spawn_handle();

            // Required for free-for-all consensus
            #[allow(deprecated)]
            old_consensus::start_collator_sync(old_consensus::StartCollatorParams {
                para_id,
                block_status: client.clone(),
                announce_block,
                overseer_handle,
                spawner,
                key: collator_key,
                parachain_consensus: free_for_all,
                runtime_api: client.clone(),
            });

            Ok(())
        },
        hwbench,
    )
        .await
}
