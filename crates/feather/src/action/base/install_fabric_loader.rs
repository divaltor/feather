use std::path::PathBuf;

use anyhow::{Context, Result};
use versions::Versioning;

use crate::action::{Action, ActionErrorKind, StatefulAction};
use feather_fabric::FabricClient;

#[derive(Debug, Clone)]
pub struct InstallFabricLoaderAction {
    minecraft_version: Versioning,
    loader_version: Versioning,
    working_dir: PathBuf,
}

impl InstallFabricLoaderAction {
    pub async fn plan(
        minecraft_version: Versioning,
        loader_version: Versioning,
        working_dir: PathBuf,
    ) -> Result<StatefulAction<Self>, ActionErrorKind> {
        let this = Self {
            minecraft_version,
            loader_version,
            working_dir,
        };

        // TODO: For now, always plan as uncompleted. A more robust check for existing installation can be added later.
        Ok(StatefulAction::uncompleted(this))
    }
}

#[async_trait::async_trait]
impl Action for InstallFabricLoaderAction {
    #[tracing::instrument(level = "debug", skip_all, fields(minecraft_version = %self.minecraft_version, loader_version = %self.loader_version, working_dir = %self.working_dir.display()))]
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        let fabric_client = FabricClient::default();

        let installer_versions = fabric_client.get_installer_versions().await?;

        let installer_version = installer_versions
            .first()
            .ok_or(anyhow::anyhow!("No Fabric installer versions found"))?;

        fabric_client
            .download_installer_jar(
                installer_version,
                &self.minecraft_version,
                &self.loader_version,
                &self.working_dir,
            )
            .await
            .with_context(|| {
                format!(
                    "Failed to download Fabric installer for version {} and loader version {}",
                    installer_version.version, self.loader_version
                )
            })?;

        tracing::info!(
            "Fabric installer downloaded to {}.",
            self.working_dir.display()
        );

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn revert(&self) -> Result<(), ActionErrorKind> {
        // Reverting the download of a Fabric installer is complex as the exact filename downloaded
        // isn't easily available without more intricate tracking or assumptions about FabricClient's behavior.
        // For now, this is a no-op. A more robust implementation would remove the specific installer JAR.
        tracing::warn!(
            "Revert for InstallFabricLoaderAction is currently a no-op for working_dir: {}.",
            self.working_dir.display()
        );
        Ok(())
    }
}
