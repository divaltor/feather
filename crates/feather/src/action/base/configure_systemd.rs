use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::action::{
    Action,
    lib::{OnMissing, execute_command, remove_file},
    stateful::{ActionErrorKind, StatefulAction},
};

use super::check_systemd_exists;

pub(crate) const SERVICE_DESTINATION: &str = "/etc/systemd/system/feather@.service";

#[derive(Debug, Clone)]
pub(crate) struct ConfigureSystemdService {
    service_dest: PathBuf,
    start_daemon: bool,
    content: String,
}

impl ConfigureSystemdService {
    pub(crate) async fn default() -> Result<StatefulAction<Self>, ActionErrorKind> {
        Self::plan(
            SERVICE_DESTINATION,
            include_str!("./templates/feather.service"),
            false,
        )
        .await
    }
}

impl ConfigureSystemdService {
    #[tracing::instrument(level = "debug", skip_all)]
    pub(crate) async fn plan(
        service_dest: impl AsRef<Path>,
        content: impl AsRef<str>,
        start_daemon: bool,
    ) -> Result<StatefulAction<Self>, ActionErrorKind> {
        // TODO: Move to pre-check before installing the service
        check_systemd_exists()?;

        let this = Self {
            start_daemon,
            service_dest: service_dest.as_ref().to_path_buf(),
            content: content.as_ref().to_string(),
        };

        if this.service_dest.exists() {
            tracing::debug!("Service destination file already exists, checking content");

            let file_content = tokio::fs::read_to_string(this.service_dest.clone())
                .await
                .map_err(|e| ActionErrorKind::Read(this.service_dest.clone(), e))?;

            if content.as_ref() != file_content {
                tracing::debug!("Service destination file content is different");

                // TODO: Consider upgrading the service when the content is different and we gonna migrate to a new version
                return Err(ActionErrorKind::DifferentContent(this.service_dest));
            }

            return Ok(StatefulAction::completed(this));
        }

        Ok(StatefulAction::uncompleted(this))
    }
}

#[async_trait::async_trait]
impl Action for ConfigureSystemdService {
    #[tracing::instrument(level = "debug", skip_all)]
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        tokio::fs::write(&self.service_dest, &self.content)
            .await
            .map_err(|e| ActionErrorKind::Write(self.service_dest.clone(), e))?;

        if self.start_daemon {
            execute_command(
                Command::new("systemctl")
                    .process_group(0)
                    .arg("daemon-reload")
                    .stdin(std::process::Stdio::null()),
            )
            .await?;
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn revert(&self) -> Result<(), ActionErrorKind> {
        // TODO: Add warning if any dynamic service is running or enabled so user can confirm they want to remove it
        tracing::debug!("Reverting service configuration");

        if self.start_daemon {
            // FIXME: Handle errors
            let _ = execute_command(
                Command::new("systemctl")
                    .process_group(0)
                    .arg("stop")
                    .arg("feather@*.service") // HACK: Replace with the actual service name from `const` variable
                    .stdin(std::process::Stdio::null()),
            )
            .await;

            let _ = execute_command(
                Command::new("systemctl")
                    .process_group(0)
                    .arg("disable")
                    .arg("feather@*.service") // HACK: Replace with the actual service name from `const` variable
                    .stdin(std::process::Stdio::null()),
            )
            .await;
        }

        remove_file(&self.service_dest, OnMissing::Ignore)
            .await
            .map_err(|e| ActionErrorKind::Remove(self.service_dest.clone(), e))?;

        execute_command(
            Command::new("systemctl")
                .process_group(0)
                .arg("daemon-reload")
                .stdin(std::process::Stdio::null()),
        )
        .await?;

        Ok(())
    }
}
