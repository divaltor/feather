mod cli;
mod config;
mod installer;
mod java;
mod logging;
mod minecraft;
mod modpack;

use std::{path::PathBuf, sync::LazyLock};

use anyhow::Result;
use cli::Commands;
use installer::MinecraftServerInstaller;
use modpack::MinecraftProfile;

static JAVA_CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("/opt/feather/java"));
static HOME_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("/opt/feather"));
static MINECRAFT_SERVERS_DIR: LazyLock<PathBuf> = LazyLock::new(|| HOME_DIR.join("servers"));

fn main() -> Result<()> {
    logging::init()?;

    let cli = cli::parse();

    match cli.command {
        Commands::Init(args) => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                let profile = MinecraftProfile::try_import(&args.file)?;

                let installer = MinecraftServerInstaller::new(
                    profile,
                    &args,
                    &JAVA_CACHE_DIR,
                    &MINECRAFT_SERVERS_DIR,
                );

                installer.install().await?;

                tracing::info!(
                    "Feather server initialization for modpack '{}' finished successfully.",
                    args.file
                );
                tracing::info!(
                    "Minecraft server instance is being set up. Check logs for specific server directory."
                );
                tracing::info!(
                    "{}\n{}",
                    "After completion, you might need to run the downloaded installer jar (if Fabric/Forge/etc.) ",
                    "inside the server's directory, then use systemd commands (if configured) to manage the server."
                );

                Ok::<(), anyhow::Error>(())
            })?;
        }
    }

    Ok(())
}
