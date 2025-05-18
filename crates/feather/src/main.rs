mod action;
pub mod cache;
mod cli;
mod logging;
mod modpack;
pub mod plan;

use std::{path::PathBuf, sync::LazyLock};

use anyhow::Result;
use cli::Commands;
use plan::ServerSetupManager;

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "aix"),
    not(target_os = "android"),
    any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "powerpc64"
    )
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

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
                let mut manager = ServerSetupManager::new(
                    &args,
                    &JAVA_CACHE_DIR,
                    &MINECRAFT_SERVERS_DIR,
                ).await?;

                if let Err(e) = manager.install().await {
                    tracing::error!("Server setup failed: {:?}", e);
                    return Err(anyhow::anyhow!("Server setup failed: {:?}", e));
                }

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

                Ok(())
            })?;
        }
    }

    Ok(())
}
