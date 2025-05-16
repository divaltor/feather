use std::process::Output;

use anyhow::{Result};
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
        Self { action, state: ActionState::Progress }
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
    
    pub fn completed(action: A) -> Self {
        Self { action, state: ActionState::Completed }
    }

    pub fn uncompleted(action: A) -> Self {
        Self { action, state: ActionState::Uncompleted }
    }

    pub fn skipped(action: A) -> Self {
        Self { action, state: ActionState::Skipped }
    }
}

impl StatefulAction<Box<dyn Action>> {
    pub async fn try_execute(&mut self) -> Result<(), ActionErrorKind> {
        match self.state {
            ActionState::Completed => {},
            ActionState::Skipped => {},
            _ => {
                self.state = ActionState::Progress;
                log::debug!("Executing action: {:?}", self.action);
                self.action.execute().await?;
                log::debug!("Action completed: {:?}", self.action);
                self.state = ActionState::Completed;
            }
        }
        
        Ok(())
    }
    
    pub async fn try_revert(&mut self) -> Result<(), ActionErrorKind> {
        match self.state {
            ActionState::Uncompleted => {},
            ActionState::Skipped => {},
            _ => {
                self.state = ActionState::Uncompleted;
                self.action.revert().await?;
                self.state = ActionState::Completed;
            },
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionState {
    Completed,
    Progress,
    Uncompleted,
    Skipped,
}

#[derive(Debug)]
pub struct ActionTag(pub &'static str);

impl std::fmt::Display for ActionTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl From<&'static str> for ActionTag {
    fn from(value: &'static str) -> Self {
        Self(value)
    }
}


pub type ActionResult<T> = std::result::Result<T, ActionErrorKind>;

#[non_exhaustive]
#[derive(Debug, thiserror::Error, strum::IntoStaticStr)]
pub enum ActionErrorKind {
    #[error(transparent)]
    Child(Box<ActionErrorKind>),
    #[error("Could not find a supported command to create groups in PATH; please install `groupadd` or `addgroup`")]
    MissingGroupCreationCommand,
    #[error("Could not find a supported command to add users to groups in PATH; please install `gpasswd` or `addgroup`")]
    MissingAddUserToGroupCommand,
    #[error("Could not find a supported command to delete groups in PATH; please install `groupdel` or `delgroup`")]
    MissingGroupDeletionCommand,
    #[error(
        "Could not find a supported command to create users in PATH; please install `useradd` or `adduser`"
    )]
    MissingUserCreationCommand,
    #[error(
        "Could not find a supported command to delete users in PATH; please install `userdel` or `deluser`"
    )]
    MissingUserDeletionCommand,
    #[error("Group `{0}` existed but had a different gid ({1}) than planned ({2})")]
    GroupGidMismatch(String, u32, u32),
    #[error("Getting gid for group `{0}`")]
    GettingGroupId(String, #[source] nix::errno::Errno),
    #[error("Getting uid for user `{0}`")]
    GettingUserId(String, #[source] nix::errno::Errno),
    #[error("User `{0}` existed but had a different uid ({1}) than planned ({2})")]
    UserUidMismatch(String, u32, u32),
    #[error("User `{0}` existed but had a different gid ({1}) than planned ({2})")]
    UserGidMismatch(String, u32, u32),
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
    CommandOutput {
        command: String,
        output: Output,
    },
}

impl ActionErrorKind {
    pub fn command(command: &tokio::process::Command, error: std::io::Error) -> Self {
        Self::Command { command: format!("{:?}", command), error }
    }
    
    pub fn command_output(command: &tokio::process::Command, output: std::process::Output) -> Self {
        Self::CommandOutput { command: format!("{:?}", command), output }
    }
}