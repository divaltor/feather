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
}
