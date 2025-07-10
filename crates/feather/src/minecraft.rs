use crate::modpack::{LoaderType, MinecraftProfile};
use anyhow::{Context, Result};
use std::path::Path;

pub struct MinecraftInstaller {
    server_dir: std::path::PathBuf,
}

impl MinecraftInstaller {
    pub fn new(server_dir: &Path) -> Self {
        Self {
            server_dir: server_dir.to_path_buf(),
        }
    }

    pub async fn install(&self, profile: &MinecraftProfile) -> Result<()> {
        match &profile.loader {
            Some(loader) => match &loader.name {
                LoaderType::Fabric => {
                    self.install_fabric_loader(profile).await?;
                }
            },
            None => {
                return Err(anyhow::anyhow!(
                    "Vanilla Minecraft server installation is not yet supported. Minecraft version: {}",
                    profile.version
                ));
            }
        }

        if let Some(modpack) = &profile.modpack {
            self.install_modpack_files(modpack).await?;
        }

        Ok(())
    }

    async fn install_fabric_loader(&self, profile: &MinecraftProfile) -> Result<()> {
        tracing::info!("Installing Fabric loader...");

        let loader = profile.loader.as_ref().unwrap();
        let fabric_installer_url = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/1.0.1/server/jar",
            profile.version, loader.version
        );

        tracing::debug!("Downloading Fabric server from: {}", fabric_installer_url);

        let response = reqwest::get(&fabric_installer_url).await.with_context(|| {
            format!("Failed to download Fabric server from {fabric_installer_url}")
        })?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download Fabric server: HTTP {}",
                response.status()
            ));
        }

        let server_jar_path = self.server_dir.join("server.jar");
        let bytes = response
            .bytes()
            .await
            .context("Failed to read Fabric server response")?;

        tokio::fs::write(&server_jar_path, bytes)
            .await
            .with_context(|| {
                format!(
                    "Failed to write server.jar to {}",
                    server_jar_path.display()
                )
            })?;

        tracing::info!("Fabric loader installed successfully");
        Ok(())
    }

    async fn install_modpack_files(&self, modpack: &crate::modpack::Modpack) -> Result<()> {
        tracing::info!("Installing modpack files...");

        match modpack {
            crate::modpack::Modpack::Modrinth(modrinth_modpack) => {
                self.install_modrinth_modpack(modrinth_modpack).await?;
            }
        }

        Ok(())
    }

    async fn install_modrinth_modpack(
        &self,
        modpack: &crate::modpack::ModrinthModpack,
    ) -> Result<()> {
        tracing::info!("Installing Modrinth modpack...");

        let mods_dir = self.server_dir.join("mods");
        tokio::fs::create_dir_all(&mods_dir)
            .await
            .with_context(|| format!("Failed to create mods directory: {}", mods_dir.display()))?;

        for file in &modpack.files {
            if let Some(downloads) = &file.downloads {
                if let Some(download_url) = downloads.first() {
                    tracing::debug!("Downloading mod: {} from {}", file.path, download_url);

                    let response = reqwest::get(download_url)
                        .await
                        .with_context(|| format!("Failed to download mod from {download_url}"))?;

                    if !response.status().is_success() {
                        tracing::warn!(
                            "Failed to download mod {}: HTTP {}",
                            file.path,
                            response.status()
                        );
                        continue;
                    }

                    let mod_path = mods_dir.join(&file.path);
                    if let Some(parent) = mod_path.parent() {
                        tokio::fs::create_dir_all(parent).await.with_context(|| {
                            format!("Failed to create mod directory: {}", parent.display())
                        })?;
                    }

                    let bytes = response
                        .bytes()
                        .await
                        .context("Failed to read mod file response")?;

                    tokio::fs::write(&mod_path, bytes).await.with_context(|| {
                        format!("Failed to write mod file: {}", mod_path.display())
                    })?;

                    tracing::debug!("Downloaded mod: {}", file.path);
                }
            }
        }

        tracing::info!("Modrinth modpack installed successfully");
        Ok(())
    }
}
