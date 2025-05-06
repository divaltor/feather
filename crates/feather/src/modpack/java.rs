use std::path::{Path, PathBuf, Component};
use anyhow::{Result, Context, anyhow};
use flate2::bufread::GzDecoder;

pub enum JavaVersion {
    Java8,
    Java17,
    Java21
}

impl std::fmt::Display for JavaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaVersion::Java8 => write!(f, "8"),
            JavaVersion::Java17 => write!(f, "17"),
            JavaVersion::Java21 => write!(f, "21"),
        }
    }
}

const ARCH: &str = std::env::consts::ARCH;
const OS: &str = std::env::consts::OS;

pub struct JavaManager {
    java_path: Option<PathBuf>,
}

impl JavaManager {
    pub fn new(java_path: Option<&Path>) -> Self {
        Self { java_path: java_path.map(|p| p.to_path_buf()) }
    }
}

impl JavaManager {
    pub async fn ensure_java_present(&mut self, java_version: JavaVersion, destination_dir: &Path) -> Result<()> {
        if let Some(path) = &self.java_path {
            let java_bin = path.join("bin").join("java");

            if java_bin.exists() {
                log::info!("Found Java binary at {}", java_bin.display());
                return Ok(());
            }
        }

        log::info!("Java path not set. Attempting to download Java {} into {}.", java_version, destination_dir.display());

        // TODO: Crash if not supported, probably in main.rs?
        let os = match OS {
            "linux" => "linux",
            "macos" => "mac",
            _ => return Err(anyhow!("Unsupported OS: {}", OS)),
        };
        
        let url = format!(
            "https://api.adoptium.net/v3/binary/latest/{feature_version}/ga/{os}/{arch}/{image_type}/hotspot/normal/eclipse",
            feature_version = java_version,
            os = os,
            arch = ARCH,
            image_type = "jdk"
        );
        
        log::debug!("Downloading Java installer from {}", url);
        
        let response = reqwest::get(&url).await?;
        
        response.error_for_status_ref().with_context(|| format!("Failed to download Java installer from {}", url))?;

        let body = response.bytes().await?;

        let body_reader_find_name = std::io::Cursor::new(&body);
        let mut first_pass_archive = tar::Archive::new(GzDecoder::new(body_reader_find_name));
        let mut top_level_dir_name: Option<String> = None;

        for entry_result in first_pass_archive.entries()? {
            let entry = entry_result.context("Failed to read entry from Java archive")?;
            let path = entry.path().context("Failed to get path from Java archive entry")?;

            if let Some(Component::Normal(name)) = path.components().next() {
                top_level_dir_name = Some(name.to_string_lossy().into_owned());
                break;
            }
        }

        let extracted_dir_name = top_level_dir_name
            .ok_or_else(|| anyhow!("Could not find top-level directory name in Java archive"))?;

        log::debug!("Identified Java JDK top-level directory as: {}", extracted_dir_name);

        let body_reader_for_unpack = std::io::Cursor::new(&body);
        let mut archive = tar::Archive::new(GzDecoder::new(body_reader_for_unpack));

        log::info!("Unpacking Java JDK into {}", destination_dir.display());

        archive.unpack(destination_dir)
            .with_context(|| format!("Failed to unpack Java archive into {}", destination_dir.display()))?;
        
        let final_java_path = destination_dir.join(&extracted_dir_name);
        log::info!("Java successfully installed at: {}", final_java_path.display());
        self.java_path = Some(final_java_path);

        Ok(())
    }
    
    pub fn get_java_executable(&self) -> Option<PathBuf> {
        match OS {
            "linux" => self.java_path.as_ref().map(|path| path.join("bin").join("java")),
            "macos" => self.java_path.as_ref().map(|path| path.join("Contents").join("Home").join("bin").join("java")),
            _ => None,
        }
    }
}
