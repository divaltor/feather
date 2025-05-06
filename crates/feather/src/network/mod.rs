use anyhow::Result;
use reqwest::blocking;
use reqwest::header::{HeaderMap, HeaderValue};
use rustc_hash::FxHashMap;
use serde::Deserialize;
use versions::Version;

#[derive(Deserialize)]
struct VersionManifest {
    versions: Vec<MinecraftVersion>,
}

#[derive(Deserialize)]
enum VersionType {
    #[serde(rename = "release")]
    Release,
    #[serde(rename = "snapshot")]
    Snapshot,
    #[serde(rename = "old_alpha")]
    OldAlpha,
    #[serde(rename = "old_beta")]
    OldBeta,
}

#[derive(Deserialize)]
pub struct MinecraftVersion {
    id: Version,
    #[serde(rename = "type")]
    version_type: VersionType,
    url: String,
    sha1: String,
}

pub struct MinecraftVersions(FxHashMap<Version, MinecraftVersion>);

impl From<VersionManifest> for MinecraftVersions {
    fn from(manifest: VersionManifest) -> Self {
        // PERF: Optimize cloning
        Self(manifest.versions.into_iter().map(|v| (v.id.clone(), v)).collect())
    }
}

impl MinecraftVersions {
    pub fn new() -> Result<Self> {
        let url = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

        let mut headers = HeaderMap::new();
        headers.insert("Accept-Encoding", HeaderValue::from_static("zstd"));

        let client = blocking::Client::builder().default_headers(headers).build()?;
        let response = client.get(url).send()?;
        let body = response.json::<VersionManifest>()?;

        Ok(MinecraftVersions::from(body))
    }
    
    pub fn get_version(&self, id: &Version) -> Option<&MinecraftVersion> {
        self.0.get(id)
    }
}