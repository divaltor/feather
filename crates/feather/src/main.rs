mod logging;
mod modpack;
mod cli;

use std::{fs, process::Command};

use anyhow::Result;
use cli::Commands;
use directories::{ProjectDirs, UserDirs};
use modpack::{MinecraftProfile, Setupable};
use subprocess::{Popen, PopenConfig};

fn main() -> Result<()> {
    logging::init()?;

    let project_dirs = ProjectDirs::from("", "", "feather")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    let cache_dir = project_dirs.cache_dir().to_path_buf();
    let user_dir = UserDirs::new().unwrap();
    let home_dir = user_dir.home_dir();

    let cli = cli::parse();
    
    match cli.command {
        Commands::Init(args) => {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            let profile = MinecraftProfile::try_import(&args.file)?;
            let minecraft_dir = home_dir.join(&args.working_dir);
            
            fs::create_dir_all(&minecraft_dir)?;

            let setup_context = runtime.block_on(profile.setup(minecraft_dir, &cache_dir))?;
            
            let output = Command::new(&setup_context.java_executable).args(["-version"]).output()?;

            log::info!("stdout: {}, stderr: {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            
            let mut minecraft_cmd = Popen::create(&[setup_context.java_executable.to_str().unwrap(), "-Xmx4G", "-jar", setup_context.minecraft_jar.to_str().unwrap(), "nogui"], PopenConfig::default())?;
            
            minecraft_cmd.communicate(None)?;

            log::info!("stdout: {}, stderr: {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}
