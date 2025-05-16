pub(crate) mod configure_systemd;
pub(crate) mod create_file;
pub(crate) mod create_group;
pub(crate) mod create_user;
pub(crate) mod install_java;
pub(crate) mod start_systemd;

pub mod create_directory;
pub mod create_feather_env;
pub mod install_fabric_loader;
pub mod install_minecraft;

use std::path::Path;

use super::stateful::ActionErrorKind;

// TODO: Move to pre-check before installation
#[tracing::instrument(level = "debug", skip_all)]
pub(crate) fn check_systemd_exists() -> Result<(), ActionErrorKind> {
    if !Path::new("/run/systemd/system").exists() {
        return Err(ActionErrorKind::SystemdMissing);
    }

    if which::which("systemctl").is_err() {
        return Err(ActionErrorKind::SystemdMissing);
    }

    Ok(())
}
