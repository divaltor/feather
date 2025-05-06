// mod printer;
 
use clap::{Parser, Subcommand, Args};

#[derive(Parser)]
#[command(name = "feather")]
#[command(about = "Lightweight (as feather) Minecraft version manager and modpack installer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args)]
pub struct ImportArgs {
    /// Path to the modpack file to import
    #[arg(value_name = "FILE")]
    pub file: String,

    /// Path to the working directory
    #[arg(long, default_value = ".minecraft")]
    pub working_dir: String,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(name = "from")]
    Import(ImportArgs),
}

pub fn parse() -> Cli {
    Cli::parse()
}