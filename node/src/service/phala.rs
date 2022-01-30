use cumulus_client_consensus_aura::{
    build_aura_consensus, BuildAuraConsensusParams, SlotProportion,
};
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_network::build_block_announce_validator;
use cumulus_client_service::{
    prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;

use crate::rpc;
pub use parachains_common::{AccountId, Balance, Block, Hash, Header, Index as Nonce};
use sc_executor::NativeElseWasmExecutor;

use sc_client_api::ExecutorProvider;
use sc_network::NetworkService;
use sc_service::{
    Configuration, PruningMode, Role, TFullBackend, TFullClient, TaskManager,
};
use sc_telemetry::TelemetryHandle;
use sp_api::ConstructRuntimeApi;
use sp_consensus::SlotData;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use std::sync::Arc;
use substrate_prometheus_endpoint::Registry;

pub struct RuntimeExecutor;

impl sc_executor::NativeExecutionDispatch for RuntimeExecutor {
    type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        phala_parachain_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        phala_parachain_runtime::native_version()
    }
}

pub fn build_import_queue(
    client: Arc<
        TFullClient<
            Block,
            phala_parachain_runtime::RuntimeApi,
            NativeElseWasmExecutor<RuntimeExecutor>,
        >,
    >,
    config: &Configuration,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<
    sc_consensus::DefaultImportQueue<
        Block,
        TFullClient<
            Block,
            phala_parachain_runtime::RuntimeApi,
            NativeElseWasmExecutor<RuntimeExecutor>,
        >,
    >,
    sc_service::Error,
> {
    let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

    cumulus_client_consensus_aura::import_queue::<
        sp_consensus_aura::sr25519::AuthorityPair,
        _,
        _,
        _,
        _,
        _,
        _,
    >(cumulus_client_consensus_aura::ImportQueueParams {
        block_import: client.clone(),
        client: client.clone(),
        create_inherent_data_providers: move |_, _| async move {
            let time = sp_timestamp::InherentDataProvider::from_system_time();

            let slot =
                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
                    *time,
                    slot_duration.slot_duration(),
                );

            Ok((time, slot))
        },
        registry: config.prometheus_registry().clone(),
        can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
        spawner: &task_manager.spawn_essential_handle(),
        telemetry,
    })
        .map_err(Into::into)
}

pub async fn start_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    id: ParaId,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<
        TFullClient<
            Block,
            phala_parachain_runtime::RuntimeApi,
            NativeElseWasmExecutor<RuntimeExecutor>,
        >,
    >,
)> {
    start_node_impl::<phala_parachain_runtime::RuntimeApi, RuntimeExecutor, _, _, _>(
        parachain_config,
        polkadot_config,
        id,
        |_| Ok(Default::default()),
        build_import_queue,
        |client,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_node,
         transaction_pool,
         sync_oracle,
         keystore,
         force_authoring| {
            let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

            let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
                task_manager.spawn_handle(),
                client.clone(),
                transaction_pool,
                prometheus_registry.clone(),
                telemetry.clone(),
            );

            let relay_chain_backend = relay_chain_node.backend.clone();
            let relay_chain_client = relay_chain_node.client.clone();
            Ok(build_aura_consensus::<
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
            >(BuildAuraConsensusParams {
                proposer_factory,
                create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
                    let parachain_inherent =
                        cumulus_primitives_parachain_inherent::ParachainInherentData::create_at_with_client(
                            relay_parent,
                            &relay_chain_client,
                            &*relay_chain_backend,
                            &validation_data,
                            id,
                        );
                    async move {
                        let time = sp_timestamp::InherentDataProvider::from_system_time();

                        let slot =
                            sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
                                *time,
                                slot_duration.slot_duration(),
                            );

                        let parachain_inherent = parachain_inherent.ok_or_else(|| {
                            Box::<dyn std::error::Error + Send + Sync>::from(
                                "Failed to create parachain inherent",
                            )
                        })?;
                        Ok((time, slot, parachain_inherent))
                    }
                },
                block_import: client.clone(),
                relay_chain_client: relay_chain_node.client.clone(),
                relay_chain_backend: relay_chain_node.backend.clone(),
                para_client: client.clone(),
                backoff_authoring_blocks: Option::<()>::None,
                sync_oracle,
                keystore,
                force_authoring,
                slot_duration,
                // We got around 500ms for proposing
                block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
                // And a maximum of 750ms if slots are skipped
                max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
                telemetry,
            }))
        },
    )
        .await
}

// TODO: Remove when Phala integrated Phala pallets
/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_node_impl<RuntimeApi, Executor, RB, BIQ, BIC>(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    id: ParaId,
    _rpc_ext_builder: RB,
    build_import_queue: BIQ,
    build_consensus: BIC,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
)>
    where
        RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
        + Send
        + Sync
        + 'static,
        RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
        + sp_api::Metadata<Block>
        + sp_session::SessionKeys<Block>
        + sp_api::ApiExt<
            Block,
            StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
        > + sp_offchain::OffchainWorkerApi<Block>
        + sp_block_builder::BlockBuilder<Block>
        + cumulus_primitives_core::CollectCollationInfo<Block>
        + pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
        + substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
        sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
        Executor: sc_executor::NativeExecutionDispatch + 'static,
        RB: Fn(
            Arc<TFullClient<Block, RuntimeApi, Executor>>,
        ) -> Result<jsonrpc_core::IoHandler<sc_rpc::Metadata>, sc_service::Error>
        + Send
        + 'static,
        BIQ: FnOnce(
            Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
            &Configuration,
            Option<TelemetryHandle>,
            &TaskManager,
        ) -> Result<
            sc_consensus::DefaultImportQueue<
                Block,
                TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
            >,
            sc_service::Error,
        > + 'static,
        BIC: FnOnce(
            Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
            Option<&Registry>,
            Option<TelemetryHandle>,
            &TaskManager,
            Arc<dyn RelayChainInterface>,
            Arc<
                sc_transaction_pool::FullPool<
                    Block,
                    TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
                >,
            >,
            Arc<NetworkService<Block, Hash>>,
            SyncCryptoStorePtr,
            bool,
        ) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
    if matches!(parachain_config.role, Role::Light) {
        return Err("Light client not supported!".into())
    }

    let parachain_config = prepare_node_config(parachain_config);

    let params = new_partial::<RuntimeApi, Executor, BIQ>(&parachain_config, build_import_queue)?;
    let (mut telemetry, telemetry_worker_handle) = params.other;

    let client = params.client.clone();
    let backend = params.backend.clone();
    let mut task_manager = params.task_manager;

    let (relay_chain_interface, collator_key) =
        build_relay_chain_interface(polkadot_config, telemetry_worker_handle, &mut task_manager)
            .map_err(|e| match e {
                polkadot_service::Error::Sub(x) => x,
                s => format!("{}", s).into(),
            })?;

    let block_announce_validator = BlockAnnounceValidator::new(relay_chain_interface.clone(), id);

    let force_authoring = parachain_config.force_authoring;
    let validator = parachain_config.role.is_authority();
    let prometheus_registry = parachain_config.prometheus_registry().cloned();
    let transaction_pool = params.transaction_pool.clone();
    let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);
    let (network, system_rpc_tx, start_network) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &parachain_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue: import_queue.clone(),
            block_announce_validator_builder: Some(Box::new(|_| {
                Box::new(block_announce_validator)
            })),
            warp_sync: None,
        })?;

    let rpc_extensions_builder = {
        let client = client.clone();
        let transaction_pool = transaction_pool.clone();

        Box::new(move |deny_unsafe, _| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: transaction_pool.clone(),
                deny_unsafe,
            };

            Ok(crate::rpc::create_full(deps))
        })
    };

    sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        rpc_extensions_builder,
        client: client.clone(),
        transaction_pool: transaction_pool.clone(),
        task_manager: &mut task_manager,
        config: parachain_config,
        keystore: params.keystore_container.sync_keystore(),
        backend: backend.clone(),
        network: network.clone(),
        system_rpc_tx,
        telemetry: telemetry.as_mut(),
    })?;

    let announce_block = {
        let network = network.clone();
        Arc::new(move |hash, data| network.announce_block(hash, data))
    };

    let relay_chain_slot_duration = Duration::from_secs(6);

    if validator {
        let parachain_consensus = build_consensus(
            client.clone(),
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
            para_id: id,
            block_status: client.clone(),
            announce_block,
            client: client.clone(),
            task_manager: &mut task_manager,
            relay_chain_interface,
            spawner,
            parachain_consensus,
            import_queue,
            collator_key,
            relay_chain_slot_duration,
        };

        start_collator(params).await?;
    } else {
        let params = StartFullNodeParams {
            client: client.clone(),
            announce_block,
            task_manager: &mut task_manager,
            para_id: id,
            relay_chain_interface,
            relay_chain_slot_duration,
            import_queue,
        };

        start_full_node(params)?;
    }

    start_network.start_network();

    Ok((task_manager, client))
}
