use std::{
    fs::File,
    hash::{Hash, Hasher},
    io::BufReader,
    path::Path,
};

use anyhow::{Context, Result};
use compact_str::CompactString;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tempfile::tempdir;
use versions::Versioning;
use zip::ZipArchive;

use super::{FromStr, Importable, Loader, LoaderType};

#[derive(Serialize, Deserialize, Debug, Hash, Clone)]
#[serde(rename_all = "snake_case")]
enum EnvironmentSupport {
    Required,
    Optional,
    Unsupported,
}

#[derive(Serialize, Deserialize, Debug, Hash, Clone)]
struct MinecraftEnvironment {
    client: EnvironmentSupport,
    server: EnvironmentSupport,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ModrinthFile {
    path: String,
    hashes: FxHashMap<CompactString, String>,
    // Usually Modrinth only has one download value
    downloads: SmallVec<[String; 1]>,
    file_size: u64,
    env: Option<MinecraftEnvironment>,
}

impl Hash for ModrinthFile {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        for (key, value) in self.hashes.iter() {
            key.hash(state);
            value.hash(state);
        }
        self.downloads.hash(state);
        self.file_size.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthModpack {
    format_version: u32,
    version_id: CompactString,
    name: CompactString,
    summary: Option<CompactString>,
    files: Vec<ModrinthFile>,
    dependencies: FxHashMap<CompactString, CompactString>,
}

impl Hash for ModrinthModpack {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.version_id.hash(state);
        self.name.hash(state);
        self.summary.hash(state);
        self.files.hash(state);

        for (key, value) in self.dependencies.iter() {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl ModrinthModpack {
    pub fn get_minecraft_version(&self) -> Versioning {
        Versioning::new(self.dependencies.get("minecraft").unwrap()).unwrap()
    }

    pub fn get_loader(&self) -> Option<Loader> {
        let mut loader = None;

        for (key, value) in self.dependencies.iter() {
            match key.as_str() {
                "forge" | "fabric-loader" | "quilt-loader" | "neoforge" => {
                    loader = Some(Loader {
                        version: Versioning::new(value).unwrap(),
                        name: LoaderType::from_str(key).unwrap(),
                    });
                    break;
                }
                _ => {}
            }
        }

        loader
    }
}

impl Importable<ModrinthModpack> for ModrinthModpack {
    // TODO: Make async including the extraction
    fn import<P: AsRef<Path>>(path: P) -> Result<Self> {
        // PERF: Import directly from ZIP file without extracting to temp dir
        let path = path.as_ref();

        let file = File::open(path)
            .with_context(|| format!("Failed to open .mrpack file: {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)
            .with_context(|| format!("Failed to read .mrpack file: {}", path.display()))?;

        let temp_dir = tempdir().context("Failed to create temporary directory")?;
        archive.extract(temp_dir.path()).with_context(|| {
            format!(
                "Failed to extract .mrpack file to temp dir: {}",
                temp_dir.path().display()
            )
        })?;

        let index_path = temp_dir.path().join("modrinth.index.json");

        let index_file = File::open(&index_path)?;
        let modpack: Self = serde_json::from_reader(index_file)?;

        Ok(modpack)
    }
}
