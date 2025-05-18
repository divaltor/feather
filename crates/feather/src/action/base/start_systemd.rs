use tokio::process::Command;

use crate::action::{
    Action,
    lib::execute_command,
    stateful::{ActionErrorKind, StatefulAction},
};

use super::check_systemd_exists;

#[derive(Debug, Clone)]
pub struct StartSystemd {
    unit: String,
    enable: bool,
}

impl StartSystemd {
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn plan(
        name: impl AsRef<str>,
        enable: bool,
    ) -> Result<StatefulAction<Self>, ActionErrorKind> {
        check_systemd_exists()?;

        let this = Self {
            unit: name.as_ref().to_string(),
            enable,
        };

        let is_active =
            execute_command(Command::new("systemctl").args(["is-active", &this.unit])).await?;

        if is_active.status.success() {
            Ok(StatefulAction::completed(this))
        } else {
            Ok(StatefulAction::uncompleted(this))
        }
    }
}

#[async_trait::async_trait]
impl Action for StartSystemd {
    #[tracing::instrument(level = "debug", skip_all)]
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        match self.enable {
            true => {
                execute_command(
                    Command::new("systemctl")
                        .process_group(0)
                        .arg("enable")
                        .arg("--now")
                        .arg(&self.unit)
                        .stdin(std::process::Stdio::null()),
                )
                .await?;
            }
            false => {
                execute_command(
                    Command::new("systemctl")
                        .process_group(0)
                        .arg("start")
                        .arg(&self.unit)
                        .stdin(std::process::Stdio::null()),
                )
                .await?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn revert(&self) -> Result<(), ActionErrorKind> {
        if self.enable {
            let _ = execute_command(
                Command::new("systemctl")
                    .process_group(0)
                    .arg("disable")
                    .arg(&self.unit)
                    .stdin(std::process::Stdio::null()),
            )
            .await;
        }

        // We do both to avoid an error doing `disable --now` if the user did stop it already somehow.
        execute_command(
            Command::new("systemctl")
                .process_group(0)
                .arg("stop")
                .arg(&self.unit)
                .stdin(std::process::Stdio::null()),
        )
        .await?;

        Ok(())
    }
}
