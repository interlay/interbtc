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
    cli::{Cli, Subcommand},
    service as interbtc_service,
};
use interbtc_runtime::Block;
use sc_cli::{ChainSpec, Result, RuntimeVersion, SubstrateCli};
use sc_service::{Configuration, PartialComponents, TaskManager};
use sp_core::hexdisplay::HexDisplay;
use std::io::Write;

fn load_spec(id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
    match id {
        "" => Ok(Box::new(chain_spec::local_config())),
        "dev" => Ok(Box::new(chain_spec::development_config())),
        "beta" => Ok(Box::new(chain_spec::beta_testnet_config())),
        "testnet" => Ok(Box::new(chain_spec::ChainSpec::from_json_bytes(
            &include_bytes!("../res/testnet.json")[..],
        )?)),
        path => Ok(Box::new(chain_spec::ChainSpec::from_json_file(path.into())?)),
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

    fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
        &interbtc_runtime::VERSION
    }
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
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = interbtc_service::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client, task_manager, ..
                } = interbtc_service::new_partial(&config)?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client, task_manager, ..
                } = interbtc_service::new_partial(&config)?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = interbtc_service::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = interbtc_service::new_partial(&config)?;
                Ok((cmd.run(client, backend), task_manager))
            })
        }
        Some(Subcommand::Benchmark(cmd)) => {
            if cfg!(feature = "runtime-benchmarks") {
                let runner = cli.create_runner(cmd)?;

                runner.sync_run(|config| cmd.run::<Block, interbtc_service::Executor>(config))
            } else {
                Err("Benchmarking wasn't enabled when building the node. \
				You can enable it with `--features runtime-benchmarks`."
                    .into())
            }
        }
        Some(Subcommand::ExportMetadata(params)) => {
            let mut ext = frame_support::BasicExternalities::default();
            sc_executor::with_externalities_safe(&mut ext, move || {
                let raw_meta_blob = interbtc_runtime::Runtime::metadata().into();
                let output_buf = if params.raw {
                    raw_meta_blob
                } else {
                    format!("0x{:?}", HexDisplay::from(&raw_meta_blob)).into_bytes()
                };

                if let Some(output) = &params.output {
                    std::fs::write(output, output_buf)?;
                } else {
                    std::io::stdout().write_all(&output_buf)?;
                }

                Ok::<_, sc_cli::Error>(())
            })
            .map_err(|err| sc_cli::Error::Application(err.into()))??;

            Ok(())
        }
        None => {
            let runner = cli.create_runner(&*cli.run)?;

            runner
                .run_node_until_exit(|config| async move { start_node(cli, config).await })
                .map_err(Into::into)
        }
    }
}

async fn start_node(_: Cli, config: Configuration) -> sc_service::error::Result<TaskManager> {
    interbtc_service::new_full(config).map(|(task_manager, _)| task_manager)
}
