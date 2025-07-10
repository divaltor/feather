use anyhow::{Context, Result, anyhow};
use flate2::bufread::GzDecoder;
use rustc_hash::FxHasher;
use std::hash::Hasher;
use std::{
    env::consts::{ARCH, OS},
    path::{Path, PathBuf},
};
use tar::Archive;
use versions::Versioning;

#[derive(Debug, Clone)]
pub enum JavaVersion {
    Java8,
    Java17,
    Java21,
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

pub struct JavaInstaller {
    cache_dir: PathBuf,
}

impl JavaInstaller {
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    pub fn determine_java_version(&self, mc_version: &Versioning) -> JavaVersion {
        tracing::debug!(
            "Determining required Java version for Minecraft version {}",
            mc_version
        );

        match mc_version {
            Versioning::Ideal(v) if v.major == 1 && v.minor >= 20 && v.patch >= 5 => {
                JavaVersion::Java21
            }
            Versioning::Ideal(v) if v.major == 1 && v.minor >= 17 => JavaVersion::Java17,
            Versioning::Ideal(v) if v.major == 1 && v.minor < 17 => JavaVersion::Java8,
            _ => JavaVersion::Java17,
        }
    }

    pub fn get_java_executable(&self, java_version: &JavaVersion) -> PathBuf {
        match OS {
            "linux" => self
                .cache_dir
                .join(java_version.to_string())
                .join("bin")
                .join("java"),
            "macos" => self
                .cache_dir
                .join(java_version.to_string())
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java"),
            _ => panic!("Unsupported OS: {}", OS),
        }
    }

    pub async fn install(&self, java_version: JavaVersion) -> Result<PathBuf> {
        let version_specific_path = self.cache_dir.join(java_version.to_string());

        if version_specific_path.is_symlink() {
            match std::fs::read_link(&version_specific_path) {
                Ok(link) => {
                    if self.cache_dir.join(&link).exists() {
                        tracing::info!("Java {} already installed", java_version);
                        return Ok(self.get_java_executable(&java_version));
                    } else {
                        tracing::warn!(
                            "Symlink {} points to a non-existent target {}. Downloading new JDK.",
                            version_specific_path.display(),
                            link.display()
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to read symlink {}: {}. Downloading new JDK.",
                        version_specific_path.display(),
                        e
                    );
                }
            }
        }

        tracing::info!(
            "Installing Java {} to {}",
            java_version,
            self.cache_dir.display()
        );

        let os_str = match OS {
            "linux" => "linux",
            "macos" => "mac",
            _ => return Err(anyhow!("Unsupported OS: {}", OS)),
        };

        let url = format!(
            "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jdk/hotspot/normal/eclipse",
            java_version, os_str, ARCH
        );

        tracing::debug!("Downloading Java from: {}", url);

        let response = reqwest::get(&url)
            .await
            .with_context(|| format!("Failed to download Java from {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download Java: HTTP {}",
                response.status()
            ));
        }

        let body = response
            .bytes()
            .await
            .context("Failed to read Java download response")?;

        let cursor = std::io::Cursor::new(&body);
        let gz_decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(gz_decoder);

        let mut hasher = FxHasher::default();
        hasher.write(&body);
        let hash = hasher.finish();

        let dir_name = format!("{:x}", hash);
        let jdk_install_path = self.cache_dir.join(&dir_name);

        std::fs::create_dir_all(&self.cache_dir).with_context(|| {
            format!(
                "Failed to create cache directory: {}",
                self.cache_dir.display()
            )
        })?;

        archive
            .unpack(&jdk_install_path)
            .with_context(|| format!("Failed to unpack Java to {}", jdk_install_path.display()))?;

        let mut entries = std::fs::read_dir(&jdk_install_path).with_context(|| {
            format!(
                "Failed to read unpacked JDK directory: {}",
                jdk_install_path.display()
            )
        })?;

        let jdk_subdirectory_path = match entries.next() {
            Some(Ok(entry)) if entry.file_type()?.is_dir() => entry.path(),
            _ => {
                return Err(anyhow!(
                    "Could not find JDK directory inside unpacked archive at {}",
                    jdk_install_path.display()
                ));
            }
        };

        if version_specific_path.exists() || version_specific_path.is_symlink() {
            std::fs::remove_file(&version_specific_path).with_context(|| {
                format!(
                    "Failed to remove existing symlink: {}",
                    version_specific_path.display()
                )
            })?;
        }

        std::os::unix::fs::symlink(&jdk_subdirectory_path, &version_specific_path).with_context(
            || {
                format!(
                    "Failed to create symlink from {} to {}",
                    version_specific_path.display(),
                    jdk_subdirectory_path.display()
                )
            },
        )?;

        tracing::info!("Java {} installed successfully", java_version);
        Ok(self.get_java_executable(&java_version))
    }
}
