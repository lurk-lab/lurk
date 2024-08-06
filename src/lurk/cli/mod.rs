mod config;
mod io_proof;
mod meta;
mod paths;
pub mod repl;
mod zdag;

use anyhow::Result;
use config::{set_config, Config};
use repl::Repl;

pub fn run() -> Result<()> {
    set_config(Config::default());
    Repl::new().run()
}
