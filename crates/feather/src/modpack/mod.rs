mod modrinth;
mod java;

use std::{fs::File, io::Write, path::{Component, Path, PathBuf}, str::FromStr};

use anyhow::{anyhow, Context, Result};
use feather_fabric::FabricClient;
use java::{JavaManager, JavaVersion};
use log::debug;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub use modrinth::ModrinthModpack;
use versions::Versioning;

pub trait Importable<T> {
    fn import<P: AsRef<Path>>(path: P) -> Result<T> where T: DeserializeOwned + Sized;
}

pub struct SetupContext {
    pub java_executable: PathBuf,
    pub minecraft_jar: PathBuf,
}

pub trait Setupable {
    async fn setup<T: AsRef<Path>>(&self, working_dir: T) -> Result<SetupContext>;
}

fn create_eula_file(working_dir: &Path) -> Result<()> {
    debug!("Creating eula.txt file in {}", working_dir.display());

    let eula_file = working_dir.join("eula.txt");
    let mut file = File::create(eula_file)?;
    file.write_all(b"eula=true")?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Modpack {
    Modrinth(ModrinthModpack),
}

impl Modpack {
    pub fn try_import<P: AsRef<Path>>(path: P) -> Result<Self> {
        match ModrinthModpack::import(&path) {
            Ok(modpack) => return Ok(Modpack::Modrinth(modpack)),
            Err(modrinth_error) => {
                eprintln!("Debug: Failed to import as Modrinth: {:?}", modrinth_error);

                Err(modrinth_error).context(format!(
                    "Attempted to import '{}' as Modrinth format failed.",
                    path.as_ref().display()
                ))?;
            }
        }

        Err(anyhow::anyhow!(
            "Could not import file '{}' as any known modpack type.",
            path.as_ref().display()
        ))
    }
    
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MinecraftProfile {
    pub version: Versioning,
    pub loader: Option<Loader>,
    pub modpack: Option<Modpack>,
}

impl MinecraftProfile {
    pub fn try_import<T: AsRef<Path>>(file: T) -> Result<Self> {
        debug!("Importing Minecraft profile from {}", file.as_ref().display());

        match Modpack::try_import(file) {
            Ok(modpack) => match modpack {
                Modpack::Modrinth(ref modrinth_modpack) => {
                    let profile = MinecraftProfile {
                        version: modrinth_modpack.get_minecraft_version(),
                        loader: modrinth_modpack.get_loader(),
                        modpack: Some(modpack)
                    };
                    Ok(profile)
                }
            }
            Err(e) => Err(e),
        }
    }
    
    fn required_java_version(&self) -> JavaVersion {
        debug!("Determining required Java version for {} version of Minecraft", self.version);

        match &self.version {
            Versioning::Ideal(v) if v.major == 1 && v.minor >= 20 && v.patch >= 5 => JavaVersion::Java21,
            Versioning::Ideal(v) if v.major == 1 && v.minor >= 17 => JavaVersion::Java17,
            Versioning::Ideal(v) if v.major == 1 && v.minor < 17 => JavaVersion::Java8,
            // Probably beta version or uknown, so try Java 17. Replace with Java 21 for latest beta versions in future.
            _ => JavaVersion::Java17,
        }
    }
}

impl Setupable for MinecraftProfile {
    async fn setup<T: AsRef<Path>>(&self, working_dir: T) -> Result<SetupContext> {
        match &self.loader {
            Some(loader) => {
                match &loader.name {
                    LoaderType::Fabric => {
                        let fabric_client = FabricClient::default();
                        let installer_versions = fabric_client.get_installer_versions().await?;
                        let installer_version = installer_versions.first().unwrap();

                        fabric_client.download_installer_jar(
                            installer_version,
                            &self.version,
                            &loader.version,
                            working_dir.as_ref()
                        ).await?;
                    }
                }
            }
            None => panic!("No loader found"),
        }
        
        create_eula_file(working_dir.as_ref())?;

        // TODO: Implement CacheManager to cache Java downloads.
        let mut java_manager = JavaManager::new(None);
        java_manager.ensure_java_present(self.required_java_version(), working_dir.as_ref()).await?;

        let java_executable = java_manager.get_java_executable().ok_or(anyhow!("Failed to get Java executable"))?;

        Ok(SetupContext {
            java_executable,
            minecraft_jar: working_dir.as_ref().join("fabric-server-installer.jar"),
        })
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub enum LoaderType {
    Fabric,
    // Babric,
    // Forge,
    // Quilt,
    // NeoForge,
}

impl FromStr for LoaderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "fabric" | "fabric-loader" => LoaderType::Fabric,
            // "forge" => LoaderType::Forge,
            // "quilt" => LoaderType::Quilt,
            // "neoforge" => LoaderType::NeoForge,
            _ => return Err(anyhow!("Unknown mod loader: {}", s)),
        })
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Loader {
    pub name: LoaderType,
    pub version: Versioning,
}