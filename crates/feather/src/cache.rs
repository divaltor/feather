use std::{
    env::consts::OS,
    path::{Path, PathBuf},
};

use crate::{action::base::install_java::JavaVersion, cli::InitArgs};
use versions::Versioning;

#[derive(Debug, Clone)]
pub struct CacheManager {
    pub(crate) cache_root: PathBuf,
}

impl CacheManager {
    pub fn new(cache_root: impl AsRef<Path>) -> Self {
        Self {
            cache_root: cache_root.as_ref().to_path_buf(),
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
            // Probably beta version or uknown, so try Java 17. Replace with Java 21 for latest beta versions in future.
            _ => JavaVersion::Java17,
        }
    }

    pub fn get_java_executable(&self, java_version: &JavaVersion) -> PathBuf {
        match OS {
            "linux" => self
                .cache_root
                .join(java_version.to_string())
                .join("bin")
                .join("java"),
            "macos" => self
                .cache_root
                .join(java_version.to_string())
                .join("Contents")
                .join("Home")
                .join("bin")
                .join("java"),
            _ => panic!("Unsupported OS: {}", OS),
        }
    }
}
