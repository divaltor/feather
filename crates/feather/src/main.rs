mod logging;
mod modpack;
// mod network;
mod cli;


use std::{fs, process::Command};

use anyhow::Result;
use cli::Commands;
use modpack::{MinecraftProfile, Setupable};
use directories::UserDirs;
use subprocess::{Popen, PopenConfig};

fn main() -> Result<()> {
    logging::init()?;

    let user_dir = UserDirs::new().unwrap();

    let home_dir = user_dir.home_dir();

    let cli = cli::parse();
    
    match cli.command {
        Commands::Import(args) => {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            let profile = MinecraftProfile::try_import(&args.file)?;
            
            let minecraft_dir = home_dir.join(&args.working_dir);
            
            fs::create_dir_all(&minecraft_dir)?;
            
            let setup_context = runtime.block_on(profile.setup(minecraft_dir))?;
            
            let output = Command::new(&setup_context.java_executable).args(["-version"]).output()?;

            log::info!("stdout: {}, stderr: {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
            
            let mut minecraft_cmd = Popen::create(&[setup_context.java_executable.to_str().unwrap(), "-Xmx2G", "-jar", setup_context.minecraft_jar.to_str().unwrap(), "nogui"], PopenConfig::default())?;
            
            minecraft_cmd.communicate(None)?;

            log::info!("stdout: {}, stderr: {}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        }
    }

    Ok(())
}
