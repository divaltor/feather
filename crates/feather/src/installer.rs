use crate::{
    cli::InitArgs, config::ConfigGenerator, java::JavaInstaller, minecraft::MinecraftInstaller,
    modpack::MinecraftProfile,
};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct MinecraftServerInstaller {
    profile: MinecraftProfile,
    java_cache_dir: PathBuf,
    server_dir: PathBuf,
    java_args: Vec<String>,
}

impl MinecraftServerInstaller {
    pub fn new(
        profile: MinecraftProfile,
        args: &InitArgs,
        java_cache_dir: &Path,
        minecraft_servers_dir: &Path,
    ) -> Self {
        let server_dir = minecraft_servers_dir.join(profile.hash());

        Self {
            profile,
            java_cache_dir: java_cache_dir.to_path_buf(),
            server_dir,
            java_args: args.java_args.clone(),
        }
    }

    pub async fn install(&self) -> Result<()> {
        tracing::info!("Starting Minecraft server installation...");

        self.create_directories()
            .context("Failed to create directories")?;

        self.create_user()
            .context("Failed to create feather user")?;

        let java_executable = self
            .install_java()
            .await
            .context("Failed to install Java")?;

        self.install_minecraft_server()
            .await
            .context("Failed to install Minecraft server")?;

        self.create_config_files(&java_executable)
            .context("Failed to create configuration files")?;

        self.setup_systemd()
            .context("Failed to setup systemd service")?;

        tracing::info!("Minecraft server installation completed successfully");
        Ok(())
    }

    fn create_directories(&self) -> Result<()> {
        tracing::info!("Creating directories...");

        std::fs::create_dir_all(&self.java_cache_dir).with_context(|| {
            format!(
                "Failed to create Java cache directory: {}",
                self.java_cache_dir.display()
            )
        })?;

        std::fs::create_dir_all(&self.server_dir).with_context(|| {
            format!(
                "Failed to create server directory: {}",
                self.server_dir.display()
            )
        })?;

        Ok(())
    }

    fn create_user(&self) -> Result<()> {
        tracing::info!("Creating feather user...");

        let output = std::process::Command::new("id")
            .arg("feather")
            .output()
            .context("Failed to check if feather user exists")?;

        if !output.status.success() {
            tracing::info!("Creating feather user...");
            let status = std::process::Command::new("sudo")
                .args(["useradd", "-r", "-s", "/bin/false", "feather"])
                .status()
                .context("Failed to create feather user")?;

            if !status.success() {
                return Err(anyhow::anyhow!("Failed to create feather user"));
            }
        } else {
            tracing::info!("Feather user already exists");
        }

        Ok(())
    }

    async fn install_java(&self) -> Result<PathBuf> {
        tracing::info!("Installing Java...");

        let java_installer = JavaInstaller::new(&self.java_cache_dir);
        let java_version = java_installer.determine_java_version(&self.profile.version);

        java_installer.install(java_version).await
    }

    async fn install_minecraft_server(&self) -> Result<()> {
        tracing::info!("Installing Minecraft server...");

        let minecraft_installer = MinecraftInstaller::new(&self.server_dir);
        minecraft_installer.install(&self.profile).await
    }

    fn create_config_files(&self, java_executable: &Path) -> Result<()> {
        tracing::info!("Creating configuration files...");

        let config_generator = ConfigGenerator::new(&self.server_dir);

        config_generator.create_eula_file()?;
        config_generator.create_feather_env_file(java_executable, &self.java_args, "server.jar")?;

        Ok(())
    }

    fn setup_systemd(&self) -> Result<()> {
        tracing::info!("Setting up systemd service...");

        let service_content = include_str!("templates/feather.service");
        let service_path = "/etc/systemd/system/feather.service";

        std::fs::write(service_path, service_content)
            .with_context(|| format!("Failed to write systemd service file: {service_path}"))?;

        let status = std::process::Command::new("sudo")
            .args(["systemctl", "daemon-reload"])
            .status()
            .context("Failed to reload systemd daemon")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to reload systemd daemon"));
        }

        tracing::info!("Systemd service configured successfully");
        Ok(())
    }
}
