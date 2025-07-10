mod modrinth;

use std::{
    fmt,
    hash::{Hash, Hasher},
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};

use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub use modrinth::ModrinthModpack;
use versions::Versioning;

pub trait Importable<T> {
    fn import<P: AsRef<Path>>(path: P) -> Result<T>
    where
        T: DeserializeOwned + Sized;
}

#[derive(Serialize, Deserialize, Debug, Hash, Clone)]
pub enum Modpack {
    Modrinth(ModrinthModpack),
}

impl Modpack {
    pub fn try_import<P: AsRef<Path>>(path: P) -> Result<Self> {
        match ModrinthModpack::import(&path) {
            Ok(modpack) => return Ok(Modpack::Modrinth(modpack)),
            Err(modrinth_error) => {
                tracing::error!("Debug: Failed to import as Modrinth: {:?}", modrinth_error);

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MinecraftProfile {
    pub version: Versioning,
    pub loader: Option<Loader>,
    pub modpack: Option<Modpack>,
}

impl MinecraftProfile {
    pub fn snapshot(&self) -> String {
        let loader = match &self.loader {
            Some(loader) => format!("{}", loader),
            None => "vanilla".to_string(),
        };

        let mut state = FxHasher::default();

        let modpack = match &self.modpack {
            Some(modpack) => {
                modpack.hash(&mut state);
                format!("{:x}", state.finish())
            }
            None => "none".to_string(),
        };

        format!("{}-{}-{}", self.version, loader, modpack)
    }

    pub fn hash(&self) -> String {
        let snapshot = self.snapshot();

        let mut state = FxHasher::default();
        snapshot.hash(&mut state);

        format!("{:x}", state.finish())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub fn try_import<T: AsRef<Path>>(file: T) -> Result<Self> {
        tracing::debug!(
            "Importing Minecraft profile from {}",
            file.as_ref().display()
        );

        match Modpack::try_import(file) {
            Ok(modpack) => match modpack {
                Modpack::Modrinth(ref modrinth_modpack) => {
                    let profile = MinecraftProfile {
                        version: modrinth_modpack.get_minecraft_version(),
                        loader: modrinth_modpack.get_loader(),
                        modpack: Some(modpack),
                    };
                    Ok(profile)
                }
            },
            Err(e) => Err(e),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, Clone)]
pub enum LoaderType {
    Fabric,
    // Babric,
    // Forge,
    // Quilt,
    // NeoForge,
}

impl fmt::Display for LoaderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LoaderType::Fabric => "fabric",
                // LoaderType::Forge => "forge",
                // LoaderType::Quilt => "quilt",
                // LoaderType::NeoForge => "neoforge",
            }
        )
    }
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

#[derive(Serialize, Deserialize, Debug, Hash, Clone)]
pub struct Loader {
    pub name: LoaderType,
    pub version: Versioning,
}

impl fmt::Display for Loader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
