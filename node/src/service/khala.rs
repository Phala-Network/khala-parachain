use std::sync::Arc;

use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::{
    AuraConsensus, BuildAuraConsensusParams, SlotProportion
};
use cumulus_primitives_core::ParaId;

pub use parachains_common::{AccountId, Balance, Block, Hash, Header, Index as Nonce};
use sc_executor::NativeElseWasmExecutor;

use sc_client_api::ExecutorProvider;
use sc_service::{
    Configuration, TFullClient, TaskManager,
};
use sc_telemetry::TelemetryHandle;

use khala_parachain_runtime::RuntimeApi;

pub struct RuntimeExecutor;

impl sc_executor::NativeExecutionDispatch for RuntimeExecutor {
    type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

    fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
        khala_parachain_runtime::api::dispatch(method, data)
    }

    fn native_version() -> sc_executor::NativeVersion {
        khala_parachain_runtime::native_version()
    }
}

/// Build the import queue for the parachain runtime.
#[allow(clippy::type_complexity)]
pub fn parachain_build_import_queue(
    client: Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<RuntimeExecutor>>>,
    config: &Configuration,
    telemetry: Option<TelemetryHandle>,
    task_manager: &TaskManager,
) -> Result<
    sc_consensus::DefaultImportQueue<
        Block,
        TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<RuntimeExecutor>>,
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
                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                    *time,
                    slot_duration,
                );

            Ok((time, slot))
        },
        registry: config.prometheus_registry(),
        can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
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
    id: ParaId,
) -> sc_service::error::Result<(
    TaskManager,
    Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<RuntimeExecutor>>>,
)> {
    crate::service::start_node_impl::<RuntimeApi, RuntimeExecutor, _, _, _>(
        parachain_config,
        polkadot_config,
        collator_options,
        id,
        |_| Ok(Default::default()),
        parachain_build_import_queue,
        |client,
         prometheus_registry,
         telemetry,
         task_manager,
         relay_chain_interface,
         transaction_pool,
         sync_oracle,
         keystore,
         force_authoring| {
            let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

            let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
                task_manager.spawn_handle(),
                client.clone(),
                transaction_pool,
                prometheus_registry,
                telemetry.clone(),
            );

            Ok(AuraConsensus::build::<sp_consensus_aura::sr25519::AuthorityPair, _, _, _, _, _, _>(
                BuildAuraConsensusParams {
                    proposer_factory,
                    create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
                        let relay_chain_interface = relay_chain_interface.clone();
                        async move {
                            let parachain_inherent =
                                cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
                                    relay_parent,
                                    &relay_chain_interface,
                                    &validation_data,
                                    id,
                                ).await;
                            let time = sp_timestamp::InherentDataProvider::from_system_time();

                            let slot =
                                sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                                    *time,
                                    slot_duration,
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
                    para_client: client,
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
                },
            ))
        },
    )
        .await
}
