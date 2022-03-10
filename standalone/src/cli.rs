use clap::Parser;
use std::path::PathBuf;

/// Sub-commands supported by the collator.
#[derive(Debug, Parser)]
pub enum Subcommand {
    /// Export the metadata.
    #[clap(name = "export-metadata")]
    ExportMetadata(ExportMetadataCommand),

    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),

    /// The custom benchmark subcommmand benchmarking runtime pallets.
    #[clap(name = "benchmark", about = "Benchmark runtime pallets.")]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),
}

/// Command for exporting the metadata.
#[derive(Debug, Parser)]
pub struct ExportMetadataCommand {
    /// Output file name or stdout if unspecified.
    #[clap(parse(from_os_str))]
    pub output: Option<PathBuf>,

    /// Write output in binary. Default is to write in hex.
    #[clap(short, long)]
    pub raw: bool,
}

#[derive(Debug, Parser)]
pub struct RunCmd {
    #[clap(flatten)]
    pub base: sc_cli::RunCmd,
}

impl std::ops::Deref for RunCmd {
    type Target = sc_cli::RunCmd;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[derive(Debug, Parser)]
#[clap(
    propagate_version = true,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
pub struct Cli {
    #[clap(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[clap(flatten)]
    pub run: RunCmd,
}
