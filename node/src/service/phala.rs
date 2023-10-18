use std::{sync::Arc, time::Duration};

use jsonrpsee::RpcModule;

use cumulus_client_cli::CollatorOptions;
use cumulus_client_collator::service::CollatorService;
use cumulus_client_consensus_aura::collators::basic::{
    self as basic_aura, Params as BasicAuraParams,
};
use cumulus_client_consensus_proposer::Proposer;
use cumulus_client_service::CollatorSybilResistance;
use cumulus_primitives_core::ParaId;

pub use parachains_common::Block;

use sc_service::{Configuration, TaskManager};
use sc_telemetry::TelemetryHandle;

use crate::service::{ParachainBlockImport, ParachainClient};
use phala_parachain_runtime::RuntimeApi;

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

/// Build the import queue for the parachain runtime.
#[allow(clippy::type_complexity)]
pub fn parachain_build_import_queue(
    client: Arc<ParachainClient<RuntimeApi>>,
    block_import: ParachainBlockImport<RuntimeApi>,
    config: &Configuration,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>
{
    let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

    cumulus_client_consensus_aura::import_queue::<
        sp_consensus_aura::sr25519::AuthorityPair,
        _,
        _,
        _,
        _,
        _,
    >(cumulus_client_consensus_aura::ImportQueueParams {
        block_import,
        client,
        create_inherent_data_providers: move |_, _| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

            let slot =
                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                    *timestamp,
                    slot_duration,
                );

            Ok((slot, timestamp))
        },
        registry: config.prometheus_registry(),
        spawner: &task_manager.spawn_essential_handle(),
        telemetry,
    })
        .map_err(Into::into)
}

/// Start a parachain node.
pub async fn start_parachain_node(
    parachain_config: Configuration,
    polkadot_config: Configuration,
    collator_options: CollatorOptions,
    para_id: ParaId,
    hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient<RuntimeApi>>)> {
    crate::service::start_node_impl::<RuntimeApi, _, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        CollatorSybilResistance::Resistant, // Aura
        para_id,
        |_| Ok(RpcModule::new(())),
        parachain_build_import_queue,
        |
            client,
            block_import,
            prometheus_registry,
            telemetry,
            task_manager,
            relay_chain_interface,
            transaction_pool,
            sync_oracle,
            keystore,
            relay_chain_slot_duration,
            para_id,
            collator_key,
            overseer_handle,
            announce_block
        | {
            let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

            let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
                task_manager.spawn_handle(),
                client.clone(),
                transaction_pool,
                prometheus_registry,
                telemetry.clone(),
            );
            let proposer = Proposer::new(proposer_factory);

            let collator_service = CollatorService::new(
                client.clone(),
                Arc::new(task_manager.spawn_handle()),
                announce_block,
                client.clone(),
            );

            let params = BasicAuraParams {
                create_inherent_data_providers: move |_, ()| async move { Ok(()) },
                block_import,
                para_client: client,
                relay_client: relay_chain_interface,
                sync_oracle,
                keystore,
                collator_key,
                para_id,
                overseer_handle,
                slot_duration,
                relay_chain_slot_duration,
                proposer,
                collator_service,
                // Very limited proposal time.
                authoring_duration: Duration::from_millis(500),
            };

            let fut = basic_aura::run::<
                Block,
                sp_consensus_aura::sr25519::AuthorityPair,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
            >(params);
            task_manager.spawn_essential_handle().spawn("aura", None, fut);

            Ok(())
        },
        hwbench,
    )
        .await
}
