use compact_str::{CompactString, ToCompactString};
use nix::unistd::User;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::action::{Action, ActionErrorKind, StatefulAction, lib::execute_command};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub username: CompactString,
}

impl CreateUser {
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn plan(name: impl AsRef<str>) -> Result<StatefulAction<Self>, ActionErrorKind> {
        let this = Self {
            username: name.as_ref().to_compact_string(),
        };

        match std::env::consts::OS {
            "macos" => (),
            _ => {
                if !(which::which("useradd").is_ok() || which::which("adduser").is_ok()) {
                    return Err(ActionErrorKind::MissingUserCreationCommand);
                }

                if !(which::which("userdel").is_ok() || which::which("deluser").is_ok()) {
                    return Err(ActionErrorKind::MissingUserDeletionCommand);
                }
            }
        }

        if (User::from_name(this.username.as_str())
            .map_err(|e| ActionErrorKind::GettingUserId(this.username.clone(), e))?)
        .is_some()
        {
            tracing::debug!("Creating user `{}` already complete", &this.username);
            return Ok(StatefulAction::completed(this));
        }

        Ok(StatefulAction::uncompleted(this))
    }
}

#[async_trait::async_trait]
impl Action for CreateUser {
    #[tracing::instrument(level = "debug", skip_all)]
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        let Self { username } = self;

        match std::env::consts::OS {
            "macos" => panic!("We don't support creating users on macOS"),
            _ => {
                if which::which("useradd").is_ok() {
                    execute_command(
                        Command::new("useradd")
                            .process_group(0)
                            .args([
                                "--home-dir",
                                "/var/empty",
                                "--comment",
                                "User managed by Feather CLI",
                                "--user-group",
                                "--shell",
                                "/usr/sbin/nologin",
                                username,
                            ])
                            .stdin(std::process::Stdio::null()),
                    )
                    .await?;
                } else if which::which("adduser").is_ok() {
                    execute_command(
                        Command::new("adduser")
                            .process_group(0)
                            .args([
                                "--home",
                                "/var/empty",
                                "--no-create-home",
                                "--comment",
                                "User managed by Feather CLI",
                                "--shell",
                                "/usr/sbin/nologin",
                                "--disabled-login",
                                username,
                            ])
                            .stdin(std::process::Stdio::null()),
                    )
                    .await?;
                } else {
                    return Err(ActionErrorKind::MissingUserCreationCommand);
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn revert(&self) -> Result<(), ActionErrorKind> {
        match std::env::consts::OS {
            "macos" => panic!("We don't support creating users on macOS"),
            _ => {
                if which::which("userdel").is_ok() {
                    execute_command(
                        Command::new("userdel")
                            .process_group(0)
                            .arg(&self.username)
                            .stdin(std::process::Stdio::null()),
                    )
                    .await?;
                } else if which::which("deluser").is_ok() {
                    execute_command(
                        Command::new("deluser")
                            .process_group(0)
                            .arg(&self.username)
                            .stdin(std::process::Stdio::null()),
                    )
                    .await?;
                } else {
                    return Err(ActionErrorKind::MissingUserDeletionCommand);
                }
            }
        }

        Ok(())
    }
}
