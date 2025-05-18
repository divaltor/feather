use crate::{
    action::{Action, ActionErrorKind, StatefulAction},
    cache::CacheManager,
};
use compact_str::{ToCompactString, format_compact};
use versions::Versioning;

use anyhow::{Context, Result as AnyhowResult, anyhow};
use flate2::bufread::GzDecoder;
use rustc_hash::FxHasher;
use std::hash::Hasher;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::{fs, os::unix::fs::chown};
use tar::Archive;

#[derive(Debug, Clone)]
pub struct InstallJava {
    minecraft_version: Versioning,
    cache_manager: CacheManager,
}

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

const ARCH: &str = std::env::consts::ARCH;
const OS: &str = std::env::consts::OS;

impl InstallJava {
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn plan(
        minecraft_version: Versioning,
        cache_manager: CacheManager,
    ) -> Result<StatefulAction<Self>, ActionErrorKind> {
        let this = Self {
            minecraft_version,
            cache_manager,
        };

        let java_version = this
            .cache_manager
            .determine_java_version(&this.minecraft_version);

        let version_specific_path = this
            .cache_manager
            .cache_root
            .join(java_version.to_compact_string());

        if version_specific_path.is_symlink()
            && tokio::fs::read_link(&version_specific_path).await.is_ok()
        {
            tracing::debug!(
                "Java {} already installed for Minecraft {} at {}",
                java_version,
                this.minecraft_version,
                version_specific_path.display()
            );
            return Ok(StatefulAction::completed(this));
        }

        Ok(StatefulAction::uncompleted(this))
    }

    async fn ensure_java_present(&self) -> AnyhowResult<PathBuf> {
        let java_version = self
            .cache_manager
            .determine_java_version(&self.minecraft_version);

        let version_specific_path = self
            .cache_manager
            .cache_root
            .join(java_version.to_compact_string());

        if version_specific_path.is_symlink() {
            match fs::read_link(&version_specific_path) {
                Ok(link) => {
                    if self.cache_manager.cache_root.join(&link).exists() {
                        return Ok(self.cache_manager.get_java_executable(&java_version));
                    } else {
                        tracing::warn!(
                            "Symlink {} points to a non-existent target {}. Downloading and unpacking new JDK.",
                            version_specific_path.display(),
                            link.display()
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to read symlink {}, probably does not exist: {}. Downloading and unpacking new JDK.",
                        version_specific_path.display(),
                        e
                    );
                }
            }
        }

        tracing::info!(
            "Java {} not found. Will download and unpack into {}",
            java_version,
            self.cache_manager.cache_root.display()
        );

        let os_str = match OS {
            "linux" => "linux",
            "macos" => "mac",
            _ => return Err(anyhow!("Unsupported OS: {}", OS)),
        };

        let url = format!(
            "https://api.adoptium.net/v3/binary/latest/{feature_version}/ga/{os}/{arch}/{image_type}/hotspot/normal/eclipse",
            feature_version = java_version,
            os = os_str,
            arch = ARCH,
            image_type = "jdk"
        );

        tracing::debug!("Downloading Java installer from {}", url);

        let response = reqwest::get(&url).await?;
        response
            .error_for_status_ref()
            .with_context(|| format!("Failed to download Java installer from {}", url))?;

        let body = response.bytes().await?;
        let cursor = std::io::Cursor::new(&body);
        let gz_decoder = GzDecoder::new(cursor);
        let mut archive_for_inspection = Archive::new(gz_decoder);

        let mut hasher = FxHasher::default();
        hasher.write(&body);
        let hash = hasher.finish();

        let dir_name = format_compact!("{:x}", hash);
        let jdk_install_path = self.cache_manager.cache_root.join(&dir_name);

        if !self.cache_manager.cache_root.exists() {
            fs::create_dir_all(&self.cache_manager.cache_root).with_context(|| {
                format!(
                    "Failed to create cache directory at {}",
                    self.cache_manager.cache_root.display()
                )
            })?;
        }

        archive_for_inspection
            .unpack(&jdk_install_path)
            .with_context(|| format!("Failed to unpack Java to {}", jdk_install_path.display()))?;

        let mut entries = fs::read_dir(&jdk_install_path).with_context(|| {
            format!(
                "Failed to read unpacked JDK directory {}",
                jdk_install_path.display()
            )
        })?;

        let jdk_subdirectory_path = match entries.next() {
            Some(Ok(entry)) if entry.file_type()?.is_dir() => entry.path(),
            _ => {
                return Err(anyhow!(
                    "Could not find the JDK directory inside the unpacked archive at {}",
                    jdk_install_path.display()
                ));
            }
        };

        if version_specific_path.exists() || version_specific_path.is_symlink() {
            fs::remove_file(&version_specific_path).with_context(|| {
                format!(
                    "Failed to remove existing symlink at {}",
                    version_specific_path.display()
                )
            })?;
        }

        symlink(&jdk_subdirectory_path, &version_specific_path).with_context(|| {
            format!(
                "Failed to create symlink at {} pointing to {}",
                version_specific_path.display(),
                jdk_subdirectory_path.display()
            )
        })?;

        // FIXME: Man it's fuckin ridicolous that we have to do this
        chown(&version_specific_path, Some(1000), Some(1000))?;

        Ok(self.cache_manager.get_java_executable(&java_version))
    }
}

#[async_trait::async_trait]
impl Action for InstallJava {
    #[tracing::instrument(level = "debug", skip_all)]
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        self.ensure_java_present()
            .await
            .map_err(|e| ActionErrorKind::JavaInstall(e.to_string()))?;

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn revert(&self) -> Result<(), ActionErrorKind> {
        let java_version = self
            .cache_manager
            .determine_java_version(&self.minecraft_version);

        let version_specific_path = self
            .cache_manager
            .cache_root
            .join(java_version.to_compact_string());

        if version_specific_path.exists() {
            tokio::fs::remove_file(&version_specific_path)
                .await
                .map_err(|e| ActionErrorKind::Remove(version_specific_path.clone(), e))?;
        }
        Ok(())
    }
}
