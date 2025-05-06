use std::{fs::File, io::BufReader, path::Path};

use anyhow::{Context, Result};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tempfile::tempdir;
use versions::Versioning;
use zip::ZipArchive;

use super::{Importable, Loader, LoaderType, FromStr};

#[derive(Serialize, Deserialize, Debug)]
struct ModrinthFile {
    path: String,
    hashes: FxHashMap<String, String>,
    downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    file_size: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModrinthModpack {
    #[serde(rename = "formatVersion")]
    format_version: u32,
    #[serde(rename = "versionId")]
    version: String,
    name: String,
    summary: Option<String>,
    files: Vec<ModrinthFile>,
    #[serde(rename = "dependencies")]
    dependencies: FxHashMap<String, String>,
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
    fn import<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)
            .with_context(|| format!("Failed to open .mrpack file: {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)
            .with_context(|| format!("Failed to read .mrpack file: {}", path.display()))?;

        let temp_dir = tempdir().context("Failed to create temporary directory")?;
        archive.extract(temp_dir.path())
            .with_context(|| format!("Failed to extract .mrpack file to temp dir: {}", temp_dir.path().display()))?;

        let index_path = temp_dir.path().join("modrinth.index.json");

        let index_file = File::open(&index_path)?;
        let modpack: Self = serde_json::from_reader(index_file)?;
        
        Ok(modpack)
    }
}