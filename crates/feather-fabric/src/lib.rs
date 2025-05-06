mod structs;

use std::{fs::File, io::Write, path::Path, sync::LazyLock};

use anyhow::{Context, Result};
use log::debug;
use reqwest::{Client, Url};
use structs::InstallerVersion;
use versions::Versioning;

pub static BASE_FABRIC_URL: LazyLock<Url> =
    LazyLock::new(|| Url::parse("https://meta.fabricmc.net/v2/").unwrap());



pub struct FabricClient {
    client: Client
}

impl Default for FabricClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl FabricClient {
    pub async fn get_installer_versions(&self) -> Result<Vec<InstallerVersion>> {
        let url = BASE_FABRIC_URL.join("versions/installer")?;

        debug!("Getting installer versions from {}", url);

        let response = self.client.get(url).send().await?;

        let body = response.json::<Vec<InstallerVersion>>().await.with_context(|| "Failed to parse installer versions")?;

        Ok(body)
    }
    
    pub async fn download_installer_jar(
        &self,
        installer_version: &InstallerVersion,
        minecraft_version: &Versioning,
        fabric_version: &Versioning,
        directory: &Path
    ) -> Result<()> {
        debug!("Downloading installer jar for {} {} {}", minecraft_version, fabric_version, installer_version.version);

        let url = BASE_FABRIC_URL.join(&format!("versions/loader/{}/{}/{}/server/jar", minecraft_version, fabric_version, installer_version.version))?;
        let response = self.client.get(url).send().await?;
        let body = response.bytes().await?;
        
        let mut file = File::create(directory.join("fabric-server-installer.jar"))?;
        
        debug!("Writing installer jar to {}", directory.join("fabric-server-installer.jar").display());

        file.write_all(&body)?;

        Ok(())
    }
}
