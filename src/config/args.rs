use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, value_name = "FILE", default_value = "gateway.yaml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(name = "validate", about = "Validate the configuration file")]
    ValidateConfig,
    #[command(name = "run", about = "Run the server")]
    Run,
}
