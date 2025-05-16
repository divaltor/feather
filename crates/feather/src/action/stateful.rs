use std::process::Output;

use anyhow::Result;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::action::Action;
use std::os::unix::process::ExitStatusExt as _;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatefulAction<A>
where
    A: Action,
{
    action: A,
    state: ActionState,
}

impl<A> From<A> for StatefulAction<A>
where
    A: Action,
{
    fn from(action: A) -> Self {
        Self {
            action,
            state: ActionState::Progress,
        }
    }
}

impl<A> StatefulAction<A>
where
    A: Action,
{
    pub fn boxed(self) -> StatefulAction<Box<dyn Action>>
    where
        Self: 'static,
    {
        StatefulAction {
            action: Box::new(self.action),
            state: self.state,
        }
    }

    #[inline]
    pub fn inner_ref(&self) -> &A {
        &self.action
    }

    pub fn is_completed(&self) -> bool {
        self.state == ActionState::Completed
    }

    pub fn completed(action: A) -> Self {
        Self {
            action,
            state: ActionState::Completed,
        }
    }

    pub fn uncompleted(action: A) -> Self {
        Self {
            action,
            state: ActionState::Uncompleted,
        }
    }

    pub fn skipped(action: A) -> Self {
        Self {
            action,
            state: ActionState::Skipped,
        }
    }
}

impl StatefulAction<Box<dyn Action>> {
    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn try_execute(&mut self) -> Result<(), ActionErrorKind> {
        match self.state {
            ActionState::Completed => {}
            ActionState::Skipped => {}
            _ => {
                self.state = ActionState::Progress;
                tracing::debug!("Executing action: {:?}", self.action);
                self.action.execute().await?;
                tracing::debug!("Action completed: {:?}", self.action);
                self.state = ActionState::Completed;
            }
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn try_revert(&mut self) -> Result<(), ActionErrorKind> {
        match self.state {
            ActionState::Uncompleted => {}
            ActionState::Skipped => {}
            _ => {
                self.state = ActionState::Uncompleted;
                tracing::debug!("Reverting action: {:?}", self.action);
                self.action.revert().await?;
                tracing::debug!("Action reverted: {:?}", self.action);
                self.state = ActionState::Completed;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionState {
    Completed,
    Progress,
    Uncompleted,
    Skipped,
}

pub type ActionResult<T> = std::result::Result<T, ActionErrorKind>;

#[non_exhaustive]
#[derive(Debug, thiserror::Error, strum::IntoStaticStr)]
pub enum ActionErrorKind {
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(
        "Could not find a supported command to create groups in PATH; please install `groupadd` or `addgroup`"
    )]
    MissingGroupCreationCommand,
    #[error(
        "Could not find a supported command to add users to groups in PATH; please install `gpasswd` or `addgroup`"
    )]
    MissingAddUserToGroupCommand,
    #[error(
        "Could not find a supported command to delete groups in PATH; please install `groupdel` or `delgroup`"
    )]
    MissingGroupDeletionCommand,
    #[error(
        "Could not find a supported command to create users in PATH; please install `useradd` or `adduser`"
    )]
    MissingUserCreationCommand,
    #[error(
        "Could not find a supported command to delete users in PATH; please install `userdel` or `deluser`"
    )]
    MissingUserDeletionCommand,
    // #[error("Failed to create eula.txt file in {0}: {1}")]
    // CreateEulaFile(std::path::PathBuf, #[source] std::io::Error), // Commented out this line
    #[error("Group `{0}` existed but had a different gid ({1}) than planned ({2})")]
    GroupGidMismatch(CompactString, u32, u32),
    #[error("Getting gid for group `{0}`")]
    GettingGroupId(CompactString, #[source] nix::errno::Errno),
    #[error("Getting uid for user `{0}`")]
    GettingUserId(CompactString, #[source] nix::errno::Errno),
    #[error("Group `{0}` not found")]
    GroupNotFound(CompactString),
    #[error("User `{0}` not found")]
    UserNotFound(CompactString),
    #[error("User `{0}` existed but had a different uid ({1}) than planned ({2})")]
    UserUidMismatch(CompactString, u32, u32),
    #[error("User `{0}` existed but had a different gid ({1}) than planned ({2})")]
    UserGidMismatch(CompactString, u32, u32),
    #[error(
        "Could not detect systemd. We require systemd to be installed to be able to manage services."
    )]
    SystemdMissing,
    #[error("Read path `{0}`")]
    Read(std::path::PathBuf, #[source] std::io::Error),
    #[error("Write path `{0}`")]
    Write(std::path::PathBuf, #[source] std::io::Error),
    #[error("Remove path `{0}`")]
    Remove(std::path::PathBuf, #[source] std::io::Error),
    #[error("`{0}` exists with different content than planned, consider removing it with `rm {0}`")]
    DifferentContent(std::path::PathBuf),
    #[error("Java installation failed: {0}")]
    JavaInstall(String),
    #[error("Failed to execute command: {command}")]
    Command {
        command: String,
        #[source]
        error: std::io::Error,
    },
    #[error(
        "Failed to execute command: `{command}`\nstdout: {stdout}\nstderr: {stderr}\n{maybe_status}\n{maybe_signal}",
        command = .command,
        stdout = String::from_utf8_lossy(&.output.stdout),
        stderr = String::from_utf8_lossy(&.output.stderr),
        maybe_status = if let Some(status) = .output.status.code() {
            format!("exited with status code: {status}\n")
        } else {
            "".to_string()
        },
        maybe_signal = if let Some(signal) = .output.status.signal() {
            format!("terminated by signal: {signal}\n")
        } else {
            "".to_string()
        },
    )]
    CommandOutput { command: String, output: Output },
}

impl ActionErrorKind {
    pub fn command(command: &tokio::process::Command, error: std::io::Error) -> Self {
        Self::Command {
            command: format!("{:?}", command),
            error,
        }
    }

    pub fn command_output(command: &tokio::process::Command, output: std::process::Output) -> Self {
        Self::CommandOutput {
            command: format!("{:?}", command),
            output,
        }
    }
}
