use std::{path::PathBuf, str::FromStr};

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "feather")]
#[command(about = "Lightweight (as feather) Minecraft version manager and modpack installer")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Default, Debug)]
pub enum JavaSelection {
    #[default]
    Auto,
    System,
    Custom(PathBuf),
}

impl FromStr for JavaSelection {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "auto" => JavaSelection::Auto,
            "system" => JavaSelection::System,
            path => JavaSelection::Custom(PathBuf::from(path)),
        })
    }
}

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Path to the modpack file to import
    #[arg(value_name = "FILE")]
    pub file: String,

    /// Path to the working directory
    #[arg(long, default_value = ".minecraft")]
    pub working_dir: String,

    /// Path to the custom Java executable.
    /// Can be "auto", "system", or a path to a Java executable.
    /// If provided by the environment variable JAVA_HOME, it will be used instead.
    /// [possible values: auto, system, <path>]
    #[arg(long, value_enum, env = "JAVA_HOME", default_value = "auto")]
    pub java: JavaSelection,

    /// Provide custom Java arguments to execute the server with
    #[arg(long, default_values_t = [
        "-XX:+UseG1GC".to_string(),
        "-XX:+ParallelRefProcEnabled".to_string(),
        "-XX:MaxGCPauseMillis=200".to_string(),
        "-XX:+UnlockExperimentalVMOptions".to_string(),
        "-XX:+DisableExplicitGC".to_string(),
        "-XX:+AlwaysPreTouch".to_string(),
        "-XX:G1HeapRegionSize=16M".to_string(),
        "-XX:G1NewSizePercent=30".to_string(),
        "-XX:G1MaxNewSizePercent=40".to_string(),
        "-XX:G1HeapWastePercent=5".to_string(),
        "-XX:G1MixedGCCountTarget=4".to_string(),
        "-XX:InitiatingHeapOccupancyPercent=15".to_string(),
        "-XX:G1MixedGCLiveThresholdPercent=90".to_string(),
        "-XX:G1RSetUpdatingPauseTimePercent=5".to_string(),
        "-XX:SurvivorRatio=32".to_string(),
        "-XX:+PerfDisableSharedMem".to_string(),
        "-XX:MaxTenuringThreshold=1".to_string(),
        "-Dusing.aikars.flags=https://mcflags.emc.gs".to_string(),
        "-Daikars.new.flags=true".to_string()
    ])]
    pub java_args: Vec<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(name = "init", about = "Initialize a new Feather server")]
    Init(InitArgs),
}

pub fn parse() -> Cli {
    Cli::parse()
}
