use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct ConfigGenerator {
    server_dir: PathBuf,
}

impl ConfigGenerator {
    pub fn new(server_dir: &Path) -> Self {
        Self {
            server_dir: server_dir.to_path_buf(),
        }
    }

    pub fn create_eula_file(&self) -> Result<()> {
        tracing::info!("Creating EULA file...");

        let eula_path = self.server_dir.join("eula.txt");
        let eula_content = "eula=true";

        std::fs::write(&eula_path, eula_content)
            .with_context(|| format!("Failed to write EULA file: {}", eula_path.display()))?;

        tracing::debug!("EULA file created at: {}", eula_path.display());
        Ok(())
    }

    pub fn create_feather_env_file(
        &self,
        java_executable: &Path,
        java_args: &[String],
        server_jar: &str,
    ) -> Result<()> {
        tracing::info!("Creating Feather environment file...");

        let env_path = self.server_dir.join("feather.env");
        let java_args_str = java_args.join(" ");

        let env_content = format!(
            "JAVA_EXECUTABLE={}\nJAVA_ARGS={}\nSERVER_JAR={}\n",
            java_executable.display(),
            java_args_str,
            server_jar
        );

        std::fs::write(&env_path, env_content).with_context(|| {
            format!(
                "Failed to write Feather environment file: {}",
                env_path.display()
            )
        })?;

        tracing::debug!(
            "Feather environment file created at: {}",
            env_path.display()
        );
        Ok(())
    }

    pub fn create_server_properties(&self) -> Result<()> {
        tracing::info!("Creating server.properties file...");

        let properties_path = self.server_dir.join("server.properties");

        if properties_path.exists() {
            tracing::debug!("server.properties already exists, skipping creation");
            return Ok(());
        }

        let properties_content = r#"#Minecraft server properties
server-port=25565
gamemode=survival
allow-flight=false
allow-nether=true
difficulty=easy
enable-command-block=false
level-name=world
level-seed=
level-type=default
max-players=20
motd=A Minecraft Server
online-mode=true
op-permission-level=4
player-idle-timeout=0
pvp=true
spawn-animals=true
spawn-monsters=true
spawn-npcs=true
view-distance=10
white-list=false
"#;

        std::fs::write(&properties_path, properties_content).with_context(|| {
            format!(
                "Failed to write server.properties: {}",
                properties_path.display()
            )
        })?;

        tracing::debug!(
            "server.properties created at: {}",
            properties_path.display()
        );
        Ok(())
    }
}
