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
    cli::{Cli, RelayChainCli, RuntimeName, Subcommand},
    service::{
        new_partial, InterlayRuntimeExecutor, KintsugiRuntimeExecutor, TestnetInterlayRuntimeExecutor,
        TestnetKintsugiRuntimeExecutor,
    },
};
use cumulus_primitives_core::ParaId;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use log::info;
use primitives::Block;
use regex::Regex;
use sc_cli::{
    ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams, NetworkParams, Result,
    RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::{
    config::{BasePath, PrometheusConfig},
    Configuration, TaskManager,
};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::AccountIdConversion;
use std::{io::Write, net::SocketAddr, path::PathBuf};

#[cfg(feature = "runtime-benchmarks")]
use crate::benchmarking::*;

const DEFAULT_PARA_ID: u32 = 2121;

pub trait IdentifyChain {
    fn is_interlay(&self) -> bool;
    fn is_kintsugi(&self) -> bool;
    fn is_interlay_testnet(&self) -> bool;
    fn is_kintsugi_testnet(&self) -> bool;
}

impl IdentifyChain for dyn sc_service::ChainSpec {
    fn is_interlay(&self) -> bool {
        self.id().starts_with("interlay")
    }
    fn is_kintsugi(&self) -> bool {
        // NOTE: naming kept for backwards compatibility
        // changing the ID would require collators to move
        // their database files or resync
        self.id().starts_with("kusama") || self.id() == "kintsugi"
    }
    fn is_interlay_testnet(&self) -> bool {
        self.id().starts_with("testnet-interlay")
    }
    fn is_kintsugi_testnet(&self) -> bool {
        self.id().starts_with("testnet-kintsugi") || self.id().starts_with("staging")
    }
}

impl<T: sc_service::ChainSpec + 'static> IdentifyChain for T {
    fn is_interlay(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_interlay(self)
    }
    fn is_kintsugi(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_kintsugi(self)
    }
    fn is_interlay_testnet(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_interlay_testnet(self)
    }
    fn is_kintsugi_testnet(&self) -> bool {
        <dyn sc_service::ChainSpec>::is_kintsugi_testnet(self)
    }
}

fn load_spec(id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
    Ok(match id {
        "" => Box::new(chain_spec::testnet_kintsugi::local_config(DEFAULT_PARA_ID.into())),
        "dev" => Box::new(chain_spec::testnet_kintsugi::development_config(DEFAULT_PARA_ID.into())),
        "rococo" => Box::new(chain_spec::testnet_kintsugi::rococo_testnet_config(
            chain_spec::kintsugi::PARA_ID.into(),
        )),
        "westend" => Box::new(chain_spec::testnet_kintsugi::westend_testnet_config(
            DEFAULT_PARA_ID.into(),
        )),
        "kintsugi-dev" | "kintsugi-bench" => Box::new(chain_spec::kintsugi::kintsugi_dev_config()),
        "kintsugi-latest" => Box::new(chain_spec::kintsugi::kintsugi_mainnet_config()),
        "kintsugi" => Box::new(chain_spec::KintsugiChainSpec::from_json_bytes(
            &include_bytes!("../res/kintsugi.json")[..],
        )?),
        "interlay-dev" | "interlay-bench" => Box::new(chain_spec::interlay::interlay_dev_config()),
        "interlay-latest" => Box::new(chain_spec::interlay::interlay_mainnet_config()),
        "interlay" => Box::new(chain_spec::InterlayChainSpec::from_json_bytes(
            &include_bytes!("../res/interlay.json")[..],
        )?),
        "interlay-testnet-bench" => Box::new(chain_spec::testnet_interlay::staging_testnet_config(
            DEFAULT_PARA_ID.into(),
            true,
        )),
        "interlay-testnet-latest" => Box::new(chain_spec::testnet_interlay::staging_testnet_config(
            DEFAULT_PARA_ID.into(),
            false,
        )),
        "kintsugi-testnet-bench" => Box::new(chain_spec::testnet_kintsugi::staging_testnet_config(
            DEFAULT_PARA_ID.into(),
            true,
        )),
        "kintsugi-testnet-latest" => Box::new(chain_spec::testnet_kintsugi::staging_testnet_config(
            DEFAULT_PARA_ID.into(),
            false,
        )),
        "kintsugi-staging" => Box::new(chain_spec::testnet_kintsugi::staging_mainnet_config(false)),
        "interlay-staging" => Box::new(chain_spec::testnet_interlay::staging_mainnet_config(false)),
        "moonbase-alpha" => Box::new(chain_spec::testnet_kintsugi::staging_testnet_config(1002.into(), false)),
        path => {
            if let Some(matches) = Regex::new(r"^rococo-local-([0-9]+)$").unwrap().captures(path) {
                let para_id: u32 = matches[1].parse().expect("failed to parse chain id");
                return Ok(Box::new(chain_spec::testnet_kintsugi::rococo_local_testnet_config(
                    para_id.into(),
                )));
            }
            if let Some(matches) = Regex::new(r"^rococo-local-interlay-([0-9]+)$").unwrap().captures(path) {
                let para_id: u32 = matches[1].parse().expect("failed to parse chain id");
                return Ok(Box::new(chain_spec::testnet_interlay::rococo_local_testnet_config(
                    para_id.into(),
                )));
            }

            let chain_spec = chain_spec::DummyChainSpec::from_json_file(path.into())?;
            if chain_spec.is_interlay() {
                Box::new(chain_spec::InterlayChainSpec::from_json_file(path.into())?)
            } else if chain_spec.is_kintsugi() {
                Box::new(chain_spec::KintsugiChainSpec::from_json_file(path.into())?)
            } else if chain_spec.is_kintsugi_testnet() {
                Box::new(chain_spec::KintsugiTestnetChainSpec::from_json_file(path.into())?)
            } else if chain_spec.is_interlay_testnet() {
                Box::new(chain_spec::InterlayTestnetChainSpec::from_json_file(path.into())?)
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

        } else if $chain_spec.is_interlay_testnet() {
            #[allow(unused_imports)]
            use { testnet_interlay_runtime::RuntimeApi, crate::service::TestnetInterlayRuntimeExecutor as Executor };
            $( $code )*

        } else {
            // TODO: return error if not testnet-kintsugi?
            #[allow(unused_imports)]
            use { testnet_kintsugi_runtime::RuntimeApi, crate::service::TestnetKintsugiRuntimeExecutor as Executor };
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
        } else if chain_spec.is_interlay_testnet() {
            &testnet_interlay_runtime::VERSION
        } else {
            // TODO: panic if not testnet-kintsugi?
            &testnet_kintsugi_runtime::VERSION
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
                #[allow(unused_imports)]
                use InterlayRuntimeExecutor as Executor;
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
                #[allow(unused_imports)]
                use KintsugiRuntimeExecutor as Executor;
                { $( $code )* }.map(|v| (v, task_manager))
            })
        } else if runner.config().chain_spec.is_interlay_testnet() {
            runner.async_run(|$config| {
                let $components = new_partial::<
                    interlay_runtime::RuntimeApi,
                    TestnetInterlayRuntimeExecutor,
                >(
                    &$config,
                    true,
                )?;
                let task_manager = $components.task_manager;
                #[allow(unused_imports)]
                use TestnetInterlayRuntimeExecutor as Executor;
                { $( $code )* }.map(|v| (v, task_manager))
            })
        } else {
            runner.async_run(|$config| {
                let $components = new_partial::<
                    testnet_kintsugi_runtime::RuntimeApi,
                    TestnetKintsugiRuntimeExecutor,
                >(
                    &$config,
                    true,
                )?;
                let task_manager = $components.task_manager;
                #[allow(unused_imports)]
                use TestnetKintsugiRuntimeExecutor as Executor;
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
                        } else if runner.config().chain_spec.is_kintsugi_testnet() {
                            runner.sync_run(|config| cmd.run::<Block, TestnetKintsugiRuntimeExecutor>(config))
                        } else if runner.config().chain_spec.is_interlay_testnet() {
                            runner.sync_run(|config| cmd.run::<Block, TestnetInterlayRuntimeExecutor>(config))
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
                #[cfg(not(feature = "runtime-benchmarks"))]
                BenchmarkCmd::Storage(_) => {
                    Err("Storage benchmarking can be enabled with `--features runtime-benchmarks`.".into())
                }
                #[cfg(feature = "runtime-benchmarks")]
                BenchmarkCmd::Storage(cmd) => {
                    if cfg!(feature = "runtime-benchmarks") {
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
                #[cfg(feature = "runtime-benchmarks")]
                BenchmarkCmd::Overhead(cmd) => {
                    let chain_spec = &runner.config().chain_spec;
                    with_runtime_or_err!(chain_spec, {
                        let selected_runtime = SelectedRuntime::from_chain_spec(chain_spec)?;
                        runner.sync_run(|config| {
                            let partials = new_partial::<RuntimeApi, Executor>(&config, false)?;
                            let remark_builder = RemarkBuilder {
                                client: partials.client.clone(),
                                selected_runtime,
                            };
                            cmd.run(
                                config,
                                partials.client,
                                para_benchmark_inherent_data().unwrap(),
                                Vec::new(),
                                &remark_builder,
                            )
                        })
                    })
                }
                #[cfg(not(feature = "runtime-benchmarks"))]
                BenchmarkCmd::Overhead(_) => Err("Benchmarking wasn't enabled when building the node. \
                        You can enable it with `--features runtime-benchmarks`."
                    .into()),
                BenchmarkCmd::Extrinsic(_) => Err("Unsupported benchmarking command".into()),
                BenchmarkCmd::Machine(cmd) => {
                    runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
                }
            }
        }
        Some(Subcommand::ExportGenesisState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            let chain_spec = &runner.config().chain_spec;

            with_runtime_or_err!(chain_spec, {
                return runner.sync_run(|_config| {
                    let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
                    let state_version = Cli::native_runtime_version(&spec).state_version();
                    cmd.run::<Block>(&*spec, state_version)
                });
            })
        }
        Some(Subcommand::ExportGenesisWasm(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|_config| {
                let spec = cli.load_spec(&cmd.shared_params.chain.clone().unwrap_or_default())?;
                cmd.run(&*spec)
            })
        }
        Some(Subcommand::ExportMetadata(params)) => {
            let mut ext = frame_support::BasicExternalities::default();
            sc_executor::with_externalities_safe(&mut ext, move || {
                let raw_meta_blob = match params.runtime {
                    RuntimeName::Interlay => interlay_runtime::Runtime::metadata().into(),
                    RuntimeName::Kintsugi => kintsugi_runtime::Runtime::metadata().into(),
                    RuntimeName::InterlayTestnet => testnet_interlay_runtime::Runtime::metadata().into(),
                    RuntimeName::KintsugiTestnet => testnet_kintsugi_runtime::Runtime::metadata().into(),
                };

                write_to_file_or_stdout(params.raw, &params.output, raw_meta_blob)?;
                Ok::<_, sc_cli::Error>(())
            })
            .map_err(|err| sc_cli::Error::Application(err.into()))??;

            Ok(())
        }
        #[cfg(feature = "try-runtime")]
        Some(Subcommand::TryRuntime(cmd)) => {
            use sc_executor::{sp_wasm_interface::ExtendedHostFunctions, NativeExecutionDispatch};
            use try_runtime_cli::block_building_info::timestamp_with_aura_info;
            let runner = cli.create_runner(cmd)?;
            let chain_spec = &runner.config().chain_spec;

            with_runtime_or_err!(chain_spec, {
                return runner.async_run(|config| {
                    // we don't need any of the components of new_partial, just a runtime, or a task
                    // manager to do `async_run`.
                    let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                    let task_manager = sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                        .map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;

                    let info_provider = timestamp_with_aura_info(6000);
                    Ok((
                        cmd.run::<Block, ExtendedHostFunctions<
                            sp_io::SubstrateHostFunctions,
                            <Executor as NativeExecutionDispatch>::ExtendHostFunctions,
                        >, _>(Some(info_provider)),
                        task_manager,
                    ))
                });
            })
        }
        #[cfg(not(feature = "try-runtime"))]
        Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
                You can enable it with `--features try-runtime`."
            .into()),
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

    let parachain_account = AccountIdConversion::<polkadot_primitives::v2::AccountId>::into_account_truncating(&id);

    let tokio_handle = config.tokio_handle.clone();
    let polkadot_config = SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, tokio_handle)
        .map_err(|err| format!("Relay chain argument error: {}", err))?;

    let collator_options = cli.run.collator_options();

    info!("Parachain id: {:?}", id);
    info!("Parachain account: {}", parachain_account);
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
            .base_path()?
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

    fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
        self.base.base.transaction_pool(is_dev)
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
