use serde::{Deserialize, Serialize};
use versions::Versioning;

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallerVersion {
    #[serde(deserialize_with = "Versioning::deserialize_pretty")]
    pub version: Versioning,
    pub url: String,
    pub stable: bool,
}
