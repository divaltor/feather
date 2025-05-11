use std::fs;
use std::hash::Hasher;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context, anyhow};
use flate2::bufread::GzDecoder;
use rustc_hash::FxHasher;
use tar::Archive;

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

pub async fn ensure_java_present(
    java_version: JavaVersion, 
    cache_root: &Path
) -> Result<PathBuf> {
    let java_version_str = java_version.to_string();

    // $HOME/.cache/feather/$JAVA_VERSION
    let version_specific_path = cache_root.join(&java_version_str);
    
    // Check if the version-specific path is a symlink
    if version_specific_path.is_symlink() {
        match fs::read_link(&version_specific_path) {
            Ok(link) => {
                return Ok(get_java_executable(&link));
            }
            Err(e) => {
                log::warn!("Failed to read symlink {}, probably does not exist: {}. Downloading and unpacking new JDK.", version_specific_path.display(), e);
            }
        }
    }

    log::info!("Java {} not found. Will download and unpack into {}", java_version, version_specific_path.display());

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
        
    log::debug!("Downloading Java installer from {}", url);

    let response = reqwest::get(&url).await?;
    response.error_for_status_ref().with_context(|| format!("Failed to download Java installer from {}", url))?;

    let body = response.bytes().await?;

    let cursor = std::io::Cursor::new(&body);
    let gz_decoder = GzDecoder::new(cursor);
    let mut archive_for_inspection = Archive::new(gz_decoder);
    
    // Calculate hash of the archive to determine the JDK directory name
    let mut hasher = FxHasher::default();
    hasher.write(&body);
    let hash = hasher.finish();
    
    let dir_name = format!("{:x}", hash);
    let jdk_dir_name = cache_root.join(dir_name);

    archive_for_inspection.unpack(&jdk_dir_name)?;
    
    let Some(dir) = fs::read_dir(&jdk_dir_name)?.next() else {
        return Err(anyhow!("No JDK directory found in {}", jdk_dir_name.display()));
    };

    let actual_jdk_dir_name_str = dir.unwrap().path().display().to_string();

    log::info!("Creating symlink from {} to {}", version_specific_path.display(), actual_jdk_dir_name_str);

    symlink(cache_root.join(&actual_jdk_dir_name_str), &version_specific_path)
        .with_context(|| format!("Failed to create symlink at {} pointing to {}", version_specific_path.display(), actual_jdk_dir_name_str))?;
    
    Ok(get_java_executable(&version_specific_path))
}
    
fn get_java_executable(java_symlink_path: &Path) -> PathBuf {
    match OS {
        "linux" => java_symlink_path.join("bin").join("java"),
        "macos" => java_symlink_path.join("Contents").join("Home").join("bin").join("java"),
        _ => panic!("Unsupported OS: {}", OS),
    }
}