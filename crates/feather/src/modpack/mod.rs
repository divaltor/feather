mod modrinth;

use std::{
    fmt,
    hash::{Hash, Hasher},
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};
use compact_str::{CompactString, ToCompactString, format_compact};
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub use modrinth::ModrinthModpack;
use versions::Versioning;

use crate::action::base::install_fabric_loader::InstallFabricLoaderAction;
use crate::action::{Action, stateful::StatefulAction};

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
    pub fn snapshot(&self) -> CompactString {
        let loader = match &self.loader {
            Some(loader) => format_compact!("{}", loader),
            None => "vanilla".to_compact_string(),
        };

        let mut state = FxHasher::default();

        let modpack = match &self.modpack {
            Some(modpack) => {
                modpack.hash(&mut state);

                format_compact!("{:x}", state.finish())
            }
            None => "none".to_compact_string(),
        };

        format_compact!("{}-{}-{}", self.version, loader, modpack)
    }

    pub fn hash(&self) -> CompactString {
        let snapshot = self.snapshot();

        let mut state = FxHasher::default();
        snapshot.hash(&mut state);

        format_compact!("{:x}", state.finish())
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

    pub async fn plan(&self, working_dir: &Path) -> Result<Vec<StatefulAction<Box<dyn Action>>>> {
        let mut actions: Vec<StatefulAction<Box<dyn Action>>> = Vec::new();

        match &self.loader {
            Some(loader_info) => match &loader_info.name {
                LoaderType::Fabric => {
                    let fabric_action_plan = InstallFabricLoaderAction::plan(
                        self.version.clone(),
                        loader_info.version.clone(),
                        working_dir.to_path_buf(),
                    )
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to manage Fabric loader installation: {:?}", e)
                    })?;
                    actions.push(fabric_action_plan.boxed());
                }
            },
            None => {
                // TODO: Implement Vanilla Minecraft client installation planning
                tracing::warn!(
                    "Vanilla Minecraft client installation is not yet planned. Minecraft version: {}",
                    self.version
                );
                return Err(anyhow!(
                    "Vanilla Minecraft client installation is not yet planned. Minecraft version: {}",
                    self.version
                ));
            }
        }
        Ok(actions)
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
