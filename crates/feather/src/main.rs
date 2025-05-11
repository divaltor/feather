mod logging;
mod modpack;
mod cli;
mod action;

use std::{fs, path::PathBuf, sync::LazyLock};

use anyhow::Result;
use cli::Commands;
use modpack::{MinecraftProfile, Setupable};


static JAVA_CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("/opt/feather/java"));
static HOME_DIR: LazyLock<PathBuf> = LazyLock::new(|| PathBuf::from("/opt/feather"));

fn main() -> Result<()> {
    logging::init()?;

    let cli = cli::parse();
    
    match cli.command {
        Commands::Init(args) => {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            let profile = MinecraftProfile::try_import(&args.file)?;
            
            log::info!("Imported profile: {}", profile.snapshot());

            // let minecraft_dir = HOME_DIR.join(&profile);
            
            // fs::create_dir_all(&minecraft_dir)?;

            // let setup_context = runtime.block_on(profile.setup(minecraft_dir, &JAVA_CACHE_DIR))?;
        }
    }

    Ok(())
}
