// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use crate::{
    chain_spec,
    cli::{Cli, RuntimeName, Subcommand},
    service::{new_partial, InterlayRuntimeExecutor, KintsugiRuntimeExecutor, TestnetRuntimeExecutor},
};
use frame_benchmarking_cli::BenchmarkCmd;
use primitives::Block;
use sc_cli::{ChainSpec, Result, RuntimeVersion, SubstrateCli};
use sc_service::{Configuration, TaskManager};

use crate::cli::RelayChainCli;
use codec::Encode;
use cumulus_client_service::genesis::generate_genesis_block;
use cumulus_primitives_core::ParaId;
use log::info;
use polkadot_parachain::primitives::AccountIdConversion;
use sc_cli::{CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams, NetworkParams, SharedParams};
use sc_service::config::{BasePath, PrometheusConfig};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::Block as BlockT;
use std::{io::Write, net::SocketAddr, path::PathBuf};

const DEFAULT_PARA_ID: u32 = 2121;

pub trait IdentifyChain {
    fn is_interlay(&self) -> bool;
    fn is_kintsugi(&self) -> bool;
    fn is_testnet(&self) -> bool;
}

impl IdentifyChain for dyn sc_service::ChainSpec {
    fn is_interlay(&self) -> bool {
        self.id().starts_with("interlay")
    }
    fn is_kintsugi(&self) -> bool {
        self.id().starts_with("kintsugi")
    }
    fn is_testnet(&self) -> bool {
        self.id().starts_with("testnet")
    }
}

impl<T: sc_service::ChainSpec + 'static> IdentifyChain for T {
    fn is_interlay(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_interlay(self)
    }
    fn is_kintsugi(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_kintsugi(self)
    }
    fn is_testnet(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_testnet(self)
    }
}

fn load_spec(id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
    Ok(match id {
        "" => Box::new(chain_spec::testnet::local_config(DEFAULT_PARA_ID.into())),
        "dev" => Box::new(chain_spec::testnet::development_config(DEFAULT_PARA_ID.into())),
        "rococo" => Box::new(chain_spec::testnet::rococo_testnet_config(DEFAULT_PARA_ID.into())),
        "rococo-local-2000" => Box::new(chain_spec::testnet::rococo_local_testnet_config(2000.into())),
        "rococo-local-3000" => Box::new(chain_spec::testnet::rococo_local_testnet_config(3000.into())),
        "westend" => Box::new(chain_spec::testnet::westend_testnet_config(DEFAULT_PARA_ID.into())),
        "kintsugi-latest" => Box::new(chain_spec::kintsugi::kintsugi_mainnet_config()),
        "kintsugi" => Box::new(chain_spec::KintsugiChainSpec::from_json_bytes(
            &include_bytes!("../res/kintsugi.json")[..],
        )?),
        "interlay-latest" => Box::new(chain_spec::interlay::interlay_mainnet_config()),
        "interlay" => Box::new(chain_spec::InterlayChainSpec::from_json_bytes(
            &include_bytes!("../res/interlay.json")[..],
        )?),
        "staging-latest" => Box::new(chain_spec::testnet::staging_testnet_config(DEFAULT_PARA_ID.into())),
        "moonbase-alpha" => Box::new(chain_spec::testnet::staging_testnet_config(1002.into())),
        path => {
            let chain_spec = chain_spec::DummyChainSpec::from_json_file(path.into())?;
            if chain_spec.is_interlay() {
                Box::new(chain_spec::InterlayChainSpec::from_json_file(path.into())?)
            } else if chain_spec.is_kintsugi() {
                Box::new(chain_spec::KintsugiChainSpec::from_json_file(path.into())?)
            } else if chain_spec.is_testnet() {
                Box::new(chain_spec::TestnetChainSpec::from_json_file(path.into())?)
            } else {
                Box::new(chain_spec)
            }
        }
    })
}

macro_rules! with_runtime_or_err {
	($chain_spec:expr, { $( $code:tt )* }) => {
		if $chain_spec.is_interlay() {
            #[allow(unused_imports)]
			use { interlay_runtime::RuntimeApi, crate::service::InterlayRuntimeExecutor as Executor };
			$( $code )*

		} else if $chain_spec.is_kintsugi() {
            #[allow(unused_imports)]
			use { kintsugi_runtime::RuntimeApi, crate::service::KintsugiRuntimeExecutor as Executor };
			$( $code )*

		} else {
            #[allow(unused_imports)]
			use { testnet_runtime::RuntimeApi, crate::service::TestnetRuntimeExecutor as Executor };
			$( $code )*

		}
	}
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "interBTC Parachain".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "https://github.com/interlay/interbtc/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2017
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        load_spec(id)
    }

    fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        if chain_spec.is_interlay() {
            &interlay_runtime::VERSION
        } else if chain_spec.is_kintsugi() {
            &kintsugi_runtime::VERSION
        } else {
            &testnet_runtime::VERSION
        }
    }
}

impl SubstrateCli for RelayChainCli {
    fn impl_name() -> String {
        "interBTC Parachain".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        format!(
            "Polkadot collator\n\nThe command-line arguments provided first will be \
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
        "https://github.com/paritytech/cumulus/issues/new".into()
    }

    fn copyright_start_year() -> i32 {
        2017
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        polkadot_cli::Cli::from_iter([RelayChainCli::executable_name().to_string()].iter()).load_spec(id)
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

fn write_to_file_or_stdout(raw: bool, output: &Option<PathBuf>, raw_bytes: Vec<u8>) -> Result<()> {
    let output_buf = if raw {
        raw_bytes
    } else {
        format!("0x{:?}", HexDisplay::from(&raw_bytes)).into_bytes()
    };

    if let Some(output) = output {
        std::fs::write(output, output_buf)?;
    } else {
        std::io::stdout().write_all(&output_buf)?;
    }

    Ok(())
}

macro_rules! construct_async_run {
	(|$components:ident, $cli:ident, $cmd:ident, $config:ident| $( $code:tt )* ) => {{
		let runner = $cli.create_runner($cmd)?;
		if runner.config().chain_spec.is_interlay() {
			runner.async_run(|$config| {
				let $components = new_partial::<interlay_runtime::RuntimeApi, InterlayRuntimeExecutor>(
					&$config,
                    true,
				)?;
				let task_manager = $components.task_manager;
				{ $( $code )* }.map(|v| (v, task_manager))
			})
		} else if runner.config().chain_spec.is_kintsugi() {
			runner.async_run(|$config| {
				let $components = new_partial::<
					kintsugi_runtime::RuntimeApi,
					KintsugiRuntimeExecutor,
				>(
					&$config,
                    true,
				)?;
				let task_manager = $components.task_manager;
				{ $( $code )* }.map(|v| (v, task_manager))
			})
		} else {
			runner.async_run(|$config| {
				let $components = new_partial::<
					testnet_runtime::RuntimeApi,
					TestnetRuntimeExecutor,
				>(
					&$config,
                    true,
				)?;
				let task_manager = $components.task_manager;
				{ $( $code )* }.map(|v| (v, task_manager))
			})
		}
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
            construct_async_run!(|components, cli, cmd, config| Ok(cmd.run(components.client, config.database)))
        }
        Some(Subcommand::ExportState(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| Ok(cmd.run(components.client, config.chain_spec)))
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

                let polkadot_config =
                    SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, config.tokio_handle.clone())
                        .map_err(|err| format!("Relay chain argument error: {}", err))?;

                cmd.run(config, polkadot_config)
            })
        }
        Some(Subcommand::Revert(cmd)) => {
            construct_async_run!(|components, cli, cmd, config| Ok(cmd.run(
                components.client,
                components.backend,
                None
            )))
        }
        Some(Subcommand::Benchmark(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            match cmd {
                BenchmarkCmd::Pallet(cmd) => {
                    if cfg!(feature = "runtime-benchmarks") {
                        if runner.config().chain_spec.is_interlay() {
                            runner.sync_run(|config| cmd.run::<Block, InterlayRuntimeExecutor>(config))
                        } else if runner.config().chain_spec.is_kintsugi() {
                            runner.sync_run(|config| cmd.run::<Block, KintsugiRuntimeExecutor>(config))
                        } else if runner.config().chain_spec.is_testnet() {
                            runner.sync_run(|config| cmd.run::<Block, TestnetRuntimeExecutor>(config))
                        } else {
                            Err("Chain doesn't support benchmarking".into())
                        }
                    } else {
                        Err("Benchmarking wasn't enabled when building the node. \
                You can enable it with `--features runtime-benchmarks`."
                            .into())
                    }
                }
                BenchmarkCmd::Block(cmd) => {
                    if cfg!(feature = "runtime-benchmarks") {
                        let runner = cli.create_runner(cmd)?;
                        let chain_spec = &runner.config().chain_spec;

                        with_runtime_or_err!(chain_spec, {
                            runner.sync_run(|config| {
                                let partials = new_partial::<RuntimeApi, Executor>(&config, false)?;
                                cmd.run(partials.client)
                            })
                        })
                    } else {
                        Err("Benchmarking wasn't enabled when building the node. \
                        You can enable it with `--features runtime-benchmarks`."
                            .into())
                    }
                }
                BenchmarkCmd::Storage(cmd) => {
                    if cfg!(feature = "runtime-benchmarks") {
                        let runner = cli.create_runner(cmd)?;
                        let chain_spec = &runner.config().chain_spec;

                        with_runtime_or_err!(chain_spec, {
                            runner.sync_run(|config| {
                                let partials = new_partial::<RuntimeApi, Executor>(&config, false)?;
                                let db = partials.backend.expose_db();
                                let storage = partials.backend.expose_storage();
                                cmd.run(config, partials.client.clone(), db, storage)
                            })
                        })
                    } else {
                        Err("Benchmarking wasn't enabled when building the node. \
                        You can enable it with `--features runtime-benchmarks`."
                            .into())
                    }
                }
                BenchmarkCmd::Overhead(_) => Err("Unsupported benchmarking command".into()),
            }
        }
        Some(Subcommand::ExportGenesisState(params)) => {
            let mut builder = sc_cli::LoggerBuilder::new("");
            builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
            let _ = builder.init();

            let chain_spec = load_spec(&params.chain.clone().unwrap_or_default())?;
            let state_version = Cli::native_runtime_version(&chain_spec).state_version();
            let block: Block = generate_genesis_block(&chain_spec, state_version)?;
            let raw_header = block.header().encode();
            write_to_file_or_stdout(params.raw, &params.output, raw_header)?;

            Ok(())
        }
        Some(Subcommand::ExportGenesisWasm(params)) => {
            let mut builder = sc_cli::LoggerBuilder::new("");
            builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
            let _ = builder.init();

            let raw_wasm_blob = extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
            write_to_file_or_stdout(params.raw, &params.output, raw_wasm_blob)?;

            Ok(())
        }
        Some(Subcommand::ExportMetadata(params)) => {
            let mut ext = frame_support::BasicExternalities::default();
            sc_executor::with_externalities_safe(&mut ext, move || {
                let raw_meta_blob = match params.runtime {
                    RuntimeName::Interlay => interlay_runtime::Runtime::metadata().into(),
                    RuntimeName::Kintsugi => kintsugi_runtime::Runtime::metadata().into(),
                    RuntimeName::Testnet => testnet_runtime::Runtime::metadata().into(),
                };

                write_to_file_or_stdout(params.raw, &params.output, raw_meta_blob)?;
                Ok::<_, sc_cli::Error>(())
            })
            .map_err(|err| sc_cli::Error::Application(err.into()))??;

            Ok(())
        }
        None => {
            let runner = cli.create_runner(&cli.run.normalize())?;

            runner
                .run_node_until_exit(|config| async move {
                    if cli.instant_seal {
                        start_instant(config).await
                    } else {
                        start_node(cli, config).await
                    }
                })
                .map_err(Into::into)
        }
    }
}

async fn start_instant(config: Configuration) -> sc_service::error::Result<TaskManager> {
    with_runtime_or_err!(config.chain_spec, {
        {
            crate::service::start_instant::<RuntimeApi, Executor>(config)
                .await
                .map(|r| r.0)
                .map_err(Into::into)
        }
    })
}

async fn start_node(cli: Cli, config: Configuration) -> sc_service::error::Result<TaskManager> {
    let para_id = chain_spec::Extensions::try_get(&*config.chain_spec).map(|e| e.para_id);

    let polkadot_cli = RelayChainCli::new(
        &config,
        [RelayChainCli::executable_name().to_string()]
            .iter()
            .chain(cli.relaychain_args.iter()),
    );

    let id = ParaId::from(para_id.unwrap_or(DEFAULT_PARA_ID));

    let parachain_account = AccountIdConversion::<polkadot_primitives::v2::AccountId>::into_account(&id);

    let state_version = Cli::native_runtime_version(&config.chain_spec).state_version();
    let block: Block = generate_genesis_block(&config.chain_spec, state_version).map_err(|e| format!("{:?}", e))?;
    let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

    let tokio_handle = config.tokio_handle.clone();
    let polkadot_config = SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, tokio_handle)
        .map_err(|err| format!("Relay chain argument error: {}", err))?;

    let collator_options = cli.run.collator_options();

    info!("Parachain id: {:?}", id);
    info!("Parachain Account: {}", parachain_account);
    info!("Parachain genesis state: {}", genesis_state);
    info!(
        "Is collating: {}",
        if config.role.is_authority() { "yes" } else { "no" }
    );

    with_runtime_or_err!(config.chain_spec, {
        {
            crate::service::start_node::<RuntimeApi, Executor>(config, polkadot_config, collator_options, id)
                .await
                .map(|r| r.0)
                .map_err(Into::into)
        }
    })
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

    fn prometheus_config(
        &self,
        default_listen_port: u16,
        chain_spec: &Box<dyn ChainSpec>,
    ) -> Result<Option<PrometheusConfig>> {
        self.base.base.prometheus_config(default_listen_port, chain_spec)
    }

    fn init<F>(
        &self,
        _support_url: &String,
        _impl_version: &String,
        _logger_hook: F,
        _config: &sc_service::Configuration,
    ) -> Result<()>
    where
        F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
    {
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
}
