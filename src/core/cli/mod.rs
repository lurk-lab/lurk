pub mod comm_data;
mod config;
mod debug;
mod graphql_client;
pub mod lurk_data;
pub mod meta;
pub mod microchain;
mod paths;
pub mod proofs;
mod rdg;
pub mod repl;
#[cfg(test)]
mod tests;
mod timing;
mod zdag;

use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};
use config::{set_config, Config};
use microchain::MicrochainArgs;
use repl::Repl;

#[derive(Parser, Debug)]
#[clap(version)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Enters Lurk's REPL environment ("repl" can be elided)
    Repl(ReplArgs),
    /// Loads a file, processing forms sequentially ("load" can be elided)
    Load(LoadArgs),
    /// Starts the microchain server
    Microchain(MicrochainArgs),
}

#[derive(Args, Debug)]
struct ReplArgs {
    /// Optional file to be loaded before entering the REPL
    #[clap(long, value_parser)]
    preload: Option<Utf8PathBuf>,

    #[arg(long)]
    lurkscript: bool,

    /// Run the Linera graphql backend.
    #[arg(long)]
    linera: bool,

    /// Which Linera wallet to use.
    #[arg(long)]
    with_wallet: Option<usize>,
}

#[derive(Parser, Debug)]
struct ReplCli {
    #[clap(long, value_parser)]
    preload: Option<Utf8PathBuf>,

    #[arg(long)]
    lurkscript: bool,

    #[arg(long)]
    linera: bool,

    #[arg(long)]
    with_wallet: Option<usize>,
}

#[derive(Args, Debug)]
struct LoadArgs {
    /// The file to be loaded
    #[clap(value_parser)]
    lurk_file: Utf8PathBuf,

    /// Flag to prove the last reduction
    #[arg(long)]
    prove: bool,

    /// Flag to load the file in demo mode
    #[arg(long)]
    demo: bool,

    #[arg(long)]
    linera: bool,

    #[arg(long)]
    with_wallet: Option<usize>,
}

#[derive(Parser, Debug)]
struct LoadCli {
    #[clap(value_parser = parse_filename)]
    lurk_file: Utf8PathBuf,

    #[arg(long)]
    prove: bool,

    #[arg(long)]
    demo: bool,

    #[arg(long)]
    linera: bool,

    #[arg(long)]
    with_wallet: Option<usize>,
}

fn parse_filename(file: &str) -> Result<Utf8PathBuf> {
    if ["help", "microchain"].contains(&file) {
        bail!("Invalid file name");
    }
    Ok(file.into())
}

impl ReplArgs {
    fn into_cli(self) -> ReplCli {
        let Self {
            preload,
            lurkscript,
            linera,
            with_wallet,
        } = self;
        ReplCli {
            preload,
            lurkscript,
            linera,
            with_wallet,
        }
    }
}

impl LoadArgs {
    fn into_cli(self) -> LoadCli {
        let Self {
            lurk_file,
            prove,
            demo,
            linera,
            with_wallet,
        } = self;
        LoadCli {
            lurk_file,
            prove,
            demo,
            linera,
            with_wallet,
        }
    }
}

impl Cli {
    async fn run(self) -> Result<()> {
        match self.command {
            Command::Repl(repl_args) => repl_args.into_cli().run().await,
            Command::Load(load_args) => load_args.into_cli().run().await,
            Command::Microchain(microchain_args) => microchain_args.run(),
        }
    }
}

impl ReplCli {
    async fn run(&self) -> Result<()> {
        let mut repl = Repl::new_native(self.lurkscript, self.linera, self.with_wallet);
        if let Some(lurk_file) = &self.preload {
            repl.load_file(lurk_file, false).await?;
        }
        repl.run().await
    }
}

impl LoadCli {
    async fn run(&self) -> Result<()> {
        let mut repl = Repl::new_native(false, self.linera, self.with_wallet);
        repl.load_file(&self.lurk_file, self.demo).await?;
        if self.prove {
            repl.prove_last_reduction()?;
        }
        Ok(())
    }
}

pub async fn run() -> Result<()> {
    set_config(Config::default());
    if let Ok(cli) = Cli::try_parse() {
        cli.run().await
    } else if let Ok(repl_cli) = ReplCli::try_parse() {
        repl_cli.run().await
    } else if let Ok(load_cli) = LoadCli::try_parse() {
        load_cli.run().await
    } else {
        // force printing help
        Cli::parse();
        Ok(())
    }
}
