use std::{sync::Arc, time::Duration};

use cumulus_client_cli::CollatorOptions;
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_service::{
    build_relay_chain_interface, prepare_node_config, start_collator, start_full_node,
    StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainInterface};

pub use parachains_common::{AccountId, Balance, Block, Hash, Header, Index as Nonce};

use sc_consensus::ImportQueue;
use sc_network::NetworkService;
use sc_network_common::service::NetworkBlock;
use sc_service::{
    Configuration, TFullBackend, TaskManager,
};
use sc_telemetry::TelemetryHandle;
use sp_api::ConstructRuntimeApi;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use substrate_prometheus_endpoint::Registry;

use shell_parachain_runtime::RuntimeApi;
use crate::service::{ParachainBlockImport, ParachainClient, ParachainBackend};

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
    sc_consensus::DefaultImportQueue<Block, ParachainClient<RuntimeApi>>,
    sc_service::Error,
>
    where
        RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
        RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<ParachainBackend, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>,
        sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
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
async fn start_node_impl<RuntimeApi, RB, BIQ, BIC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    para_id: ParaId,
    rpc_ext_builder: RB,
    build_import_queue: BIQ,
    build_consensus: BIC,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient<RuntimeApi>>)>
    where
        RuntimeApi: ConstructRuntimeApi<Block, ParachainClient<RuntimeApi>> + Send + Sync + 'static,
        RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<ParachainBackend, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>,
        sc_client_api::StateBackendFor<ParachainBackend, Block>: sp_api::StateBackend<BlakeTwo256>,
        RB: Fn(Arc<ParachainClient<RuntimeApi>>) -> Result<jsonrpsee::RpcModule<()>, sc_service::Error>
        + 'static,
        BIQ: FnOnce(
            Arc<ParachainClient<RuntimeApi>>,
            ParachainBlockImport<RuntimeApi>,
            &Configuration,
            Option<TelemetryHandle>,
            &TaskManager,
        ) -> Result<
            sc_consensus::DefaultImportQueue<Block, ParachainClient<RuntimeApi>>,
            sc_service::Error,
        >,
        BIC: FnOnce(
            Arc<ParachainClient<RuntimeApi>>,
            ParachainBlockImport<RuntimeApi>,
            Option<&Registry>,
            Option<TelemetryHandle>,
            &TaskManager,
            Arc<dyn RelayChainInterface>,
            Arc<sc_transaction_pool::FullPool<Block, ParachainClient<RuntimeApi>>>,
            Arc<NetworkService<Block, Hash>>,
            SyncCryptoStorePtr,
            bool,
        ) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
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
        .map_err(|e| match e {
            RelayChainError::ServiceError(polkadot_service::Error::Sub(x)) => x,
            s => s.to_string().into(),
        })?;

    let block_announce_validator =
        BlockAnnounceValidator::new(relay_chain_interface.clone(), para_id);

    let force_authoring = parachain_config.force_authoring;
    let validator = parachain_config.role.is_authority();
    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let import_queue_service = params.import_queue.service();

    let (network, system_rpc_tx, tx_handler_controller, start_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue: params.import_queue,
            block_announce_validator_builder: Some(Box::new(|_| {
                Box::new(block_announce_validator)
            })),
            warp_sync: None,
        })?;

    let rpc_client = client.clone();
    let rpc_builder = Box::new(move |_, _| rpc_ext_builder(rpc_client.clone()));

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.sync_keystore(),
        backend: backend.clone(),
        network: network.clone(),
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
        let network = network.clone();
        Arc::new(move |hash, data| network.announce_block(hash, data))
    };

    let relay_chain_slot_duration = Duration::from_secs(6);

    if validator {
        let parachain_consensus = build_consensus(
            client.clone(),
            block_import,
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|t| t.handle()),
            &task_manager,
            relay_chain_interface.clone(),
            transaction_pool,
            network,
            params.keystore_container.sync_keystore(),
            force_authoring,
        )?;

        let spawner = task_manager.spawn_handle();

        let params = StartCollatorParams {
            para_id,
            block_status: client.clone(),
            announce_block,
            client: client.clone(),
            task_manager: &mut task_manager,
            relay_chain_interface,
            spawner,
            parachain_consensus,
            import_queue: import_queue_service,
            collator_key: collator_key.expect("Command line arguments do not allow this. qed"),
            relay_chain_slot_duration,
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            task_manager: &mut task_manager,
            para_id,
            relay_chain_interface,
            relay_chain_slot_duration,
            import_queue: import_queue_service,
        };

        start_full_node(params)?;
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
        para_id,
        |_| Ok(jsonrpsee::RpcModule::new(())),
        parachain_build_import_queue,
        |client,
         block_import,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         _,
         _,
         _| {
            let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
                task_manager.spawn_handle(),
                client,
                transaction_pool,
                prometheus_registry,
                telemetry,
            );

            Ok(cumulus_client_consensus_relay_chain::build_relay_chain_consensus(
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
            ))
        },
        hwbench,
    )
        .await
}
