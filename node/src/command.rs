// Copyright (C) 2021 HashForest Technology Pte. Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    chain_spec,
    cli::{Cli, RelayChainCli, Subcommand},
};
use codec::Encode;
use cumulus_client_service::genesis::generate_genesis_block;
use cumulus_primitives_core::ParaId;
use log::info;
use polkadot_parachain::primitives::AccountIdConversion;
use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
    NetworkParams, Result, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::Block as BlockT;
use std::{io::Write, net::SocketAddr};

use crate::service::Block;

fn load_spec(id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
    let (norm_id, para_id) = extract_parachain_id(id);
    info!(
        "Loading spec: {}, custom parachain-id = {:?})",
        norm_id, para_id.unwrap_or(ParaId::new(0)).to_string()
    );
    Ok(match norm_id {
        "khala-dev" => Box::new(chain_spec::khala_development_config(
            para_id.expect("Must specify parachain id"),
        )),
        "khala-local" => Box::new(chain_spec::khala_local_config(
            para_id.expect("Must specify parachain id"),
        )),
        "whala-local" => Box::new(chain_spec::whala_local_config(
            para_id.expect("Must specify parachain id"),
        )),
        "whala" => Box::new(chain_spec::ChainSpec::from_json_bytes(
            &include_bytes!("../res/whala.json")[..],
        )?),
        "khala-staging" => Box::new(chain_spec::khala_staging_config()),
        "khala" => Box::new(chain_spec::ChainSpec::from_json_bytes(
            &include_bytes!("../res/khala.json")[..],
        )?),
        path => Box::new(chain_spec::ChainSpec::from_json_file(
            std::path::PathBuf::from(path),
        )?),
    })
}

/// Extracts the normalized chain id and parachain id from the input chain id
///
/// E.g. "khala-dev-2004" yields ("khala-dev", Some(2004))
fn extract_parachain_id(id: &str) -> (&str, Option<ParaId>) {
    const DEV_PARAM_PREFIX: &str = "khala-dev-";
    const LOCAL_PARAM_PREFIX: &str = "khala-local-";
    const WHALA_PARAM_PREFIX: &str = "whala-local-";

    let (norm_id, para) = if id.starts_with(DEV_PARAM_PREFIX) {
        let suffix = &id[DEV_PARAM_PREFIX.len()..];
        let para_id: u32 = suffix.parse().expect("Invalid parachain-id suffix");
        (&id[..DEV_PARAM_PREFIX.len() - 1], Some(para_id))
    } else if id.starts_with(LOCAL_PARAM_PREFIX) {
        let suffix = &id[LOCAL_PARAM_PREFIX.len()..];
        let para_id: u32 = suffix.parse().expect("Invalid parachain-id suffix");
        (&id[..LOCAL_PARAM_PREFIX.len() - 1], Some(para_id))
    } else if id.starts_with(WHALA_PARAM_PREFIX) {
        let suffix = &id[WHALA_PARAM_PREFIX.len()..];
        let para_id: u32 = suffix.parse().expect("Invalid parachain-id suffix");
        (&id[..WHALA_PARAM_PREFIX.len() - 1], Some(para_id))
    } else if id == "khala-dev" || id == "khala-local" {
        (id, Some(2004u32))
    } else {
        (id, None)
    };
    (norm_id, para.map(Into::into))
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Khala Node".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Khala Node\n\nThe command-line arguments provided first will be \
            passed to the parachain node, while the arguments provided after -- will be passed \
            to the relaychain node.\n\n\
            {} [parachain-args] -- [relaychain-args]",
            Self::executable_name()
        )
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/Phala-Network/khala-parachain/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2018
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        load_spec(id)
    }

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &khala_runtime::VERSION
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "Khala Node".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        "Khala Node\n\nThe command-line arguments provided first will be \
        passed to the parachain node, while the arguments provided after -- will be passed \
        to the relaychain node.\n\n\
        khala [parachain-args] -- [relaychain-args]"
            .into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/Phala-Network/khala-parachain/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2018
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        polkadot_cli::Cli::from_iter([RelayChainCli::executable_name().to_string()].iter())
            .load_spec(id)
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        polkadot_cli::Cli::native_runtime_version(chain_spec)
    }
}

fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
    let mut storage = chain_spec.build_storage()?;

    storage
        .top
        .remove(sp_core::storage::well_known_keys::CODE)
        .ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

use crate::service::{new_partial, KhalaRuntimeExecutor};

macro_rules! construct_async_run {
    (|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
        let runner = $cli.create_runner($cmd)?;
        runner.async_run(|$config| {
            let $components = new_partial::<khala_runtime::RuntimeApi, KhalaRuntimeExecutor, _>(
                &$config,
                crate::service::khala_build_import_queue,
            )?;
            let task_manager = $components.task_manager;
            { $( $code )* }.map(|v| (v, task_manager))
        })
    }}
}

/// Parse command line arguments into service configuration.
pub fn run() -> Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, config.database))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, config.chain_spec))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| {
                Ok(cmd.run(components.client, components.import_queue))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.sync_run(|config| {
                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name().to_string()]
                        .iter()
                        .chain(cli.relaychain_args.iter()),
                );

                let polkadot_config = SubstrateCli::create_configuration(
                    &polkadot_cli,
                    &polkadot_cli,
                    config.task_executor.clone(),
                )
                .map_err(|err| format!("Relay chain argument error: {}", err))?;

                cmd.run(config, polkadot_config)
            })
        }
        Some(Subcommand::Revert(cmd)) => construct_async_run!(|components, cli, cmd, config| {
            Ok(cmd.run(components.client, components.backend))
        }),
        Some(Subcommand::ExportGenesisState(params)) => {
            let mut builder = sc_cli::LoggerBuilder::new("");
            builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
            let _ = builder.init();

            let block: Block =
                generate_genesis_block(&load_spec(&params.chain.clone().unwrap_or_default())?)?;
            let raw_header = block.header().encode();
            let output_buf = if params.raw {
                raw_header
            } else {
                format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
            };

            if let Some(output) = &params.output {
                std::fs::write(output, output_buf)?;
            } else {
                std::io::stdout().write_all(&output_buf)?;
            }

            Ok(())
        }
        Some(Subcommand::ExportGenesisWasm(params)) => {
            let mut builder = sc_cli::LoggerBuilder::new("");
            builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
            let _ = builder.init();

            let raw_wasm_blob =
                extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
            let output_buf = if params.raw {
                raw_wasm_blob
            } else {
                format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
            };

            if let Some(output) = &params.output {
                std::fs::write(output, output_buf)?;
            } else {
                std::io::stdout().write_all(&output_buf)?;
            }

            Ok(())
        }
        Some(Subcommand::Benchmark(cmd)) => {
            if cfg!(feature = "runtime-benchmarks") {
                let runner = cli.create_runner(cmd)?;
                runner.sync_run(|config| cmd.run::<Block, KhalaRuntimeExecutor>(config))
            } else {
                Err("Benchmarking wasn't enabled when building the node. \
                You can enable it with `--features runtime-benchmarks`."
                    .into())
            }
        }
        None => {
            let runner = cli.create_runner(&cli.run.normalize())?;

            runner.run_node_until_exit(|config| async move {
                let para_id =
                    chain_spec::Extensions::try_get(&*config.chain_spec).map(|e| e.para_id);

                let polkadot_cli = RelayChainCli::new(
                    &config,
                    [RelayChainCli::executable_name().to_string()]
                        .iter()
                        .chain(cli.relaychain_args.iter()),
                );

                let id = ParaId::from(
                    cli.run
                        .parachain_id
                        .or(para_id)
                        .expect("Unable to determine parachain id"),
                );

                let parachain_account =
                    AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(&id);

                let block: Block =
                    generate_genesis_block(&config.chain_spec).map_err(|e| format!("{:?}", e))?;
                let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

                let task_executor = config.task_executor.clone();
                let polkadot_config =
                    SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, task_executor)
                        .map_err(|err| format!("Relay chain argument error: {}", err))?;

                info!("Parachain id: {:?}", id);
                info!("Parachain Account: {}", parachain_account);
                info!("Parachain genesis state: {}", genesis_state);
                info!(
                    "Is collating: {}",
                    if config.role.is_authority() {
                        "yes"
                    } else {
                        "no"
                    }
                );

                crate::service::start_khala_node(
                    config,
                    polkadot_config,
                    id
                )
                .await
                .map(|r| r.0)
                .map_err(Into::into)
            })
        }
    }
}

impl DefaultConfigurationValues for RelayChainCli {
    fn p2p_listen_port() -> u16 {
        30334
    }

    fn rpc_ws_listen_port() -> u16 {
        9945
    }

    fn rpc_http_listen_port() -> u16 {
        9934
    }

    fn prometheus_listen_port() -> u16 {
        9616
    }
}

impl CliConfiguration<Self> for RelayChainCli {
    fn shared_params(&self) -> &SharedParams {
        self.base.base.shared_params()
    }

    fn import_params(&self) -> Option<&ImportParams> {
        self.base.base.import_params()
    }

    fn network_params(&self) -> Option<&NetworkParams> {
        self.base.base.network_params()
    }

    fn keystore_params(&self) -> Option<&KeystoreParams> {
        self.base.base.keystore_params()
    }

    fn base_path(&self) -> Result<Option<BasePath>> {
        Ok(self
            .shared_params()
            .base_path()
            .or_else(|| self.base_path.clone().map(Into::into)))
    }

    fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_http(default_listen_port)
    }

    fn rpc_ipc(&self) -> Result<Option<String>> {
        self.base.base.rpc_ipc()
    }

    fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
        self.base.base.rpc_ws(default_listen_port)
    }

    fn prometheus_config(&self, default_listen_port: u16) -> Result<Option<PrometheusConfig>> {
        self.base.base.prometheus_config(default_listen_port)
    }

    fn init<C: SubstrateCli>(&self) -> Result<()> {
        unreachable!("PolkadotCli is never initialized; qed");
    }

    fn chain_id(&self, is_dev: bool) -> Result<String> {
        let chain_id = self.base.base.chain_id(is_dev)?;

        Ok(if chain_id.is_empty() {
            self.chain_id.clone().unwrap_or_default()
        } else {
            chain_id
        })
    }

    fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
        self.base.base.role(is_dev)
    }

    fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool()
    }

    fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
        self.base.base.state_cache_child_ratio()
    }

    fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
        self.base.base.rpc_methods()
    }

    fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
        self.base.base.rpc_ws_max_connections()
    }

    fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
        self.base.base.rpc_cors(is_dev)
    }

    fn telemetry_external_transport(&self) -> Result<Option<sc_service::config::ExtTransport>> {
        self.base.base.telemetry_external_transport()
    }

    fn default_heap_pages(&self) -> Result<Option<u64>> {
        self.base.base.default_heap_pages()
    }

    fn force_authoring(&self) -> Result<bool> {
        self.base.base.force_authoring()
    }

    fn disable_grandpa(&self) -> Result<bool> {
        self.base.base.disable_grandpa()
    }

    fn max_runtime_instances(&self) -> Result<Option<usize>> {
        self.base.base.max_runtime_instances()
    }

    fn announce_block(&self) -> Result<bool> {
        self.base.base.announce_block()
    }

    fn telemetry_endpoints(
        &self,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
        self.base.base.telemetry_endpoints(chain_spec)
    }
}
