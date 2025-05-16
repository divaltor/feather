use std::{path::Path, process::Output};

use tokio::process::Command;

use crate::action::ActionErrorKind;

use super::stateful::ActionResult;

#[tracing::instrument(level = "debug", skip_all, fields(command = %format!("{:?}", command.as_std())))]
pub(crate) async fn execute_command(command: &mut Command) -> ActionResult<Output> {
    tracing::trace!("Executing command: {:?}", command);

    let output = command
        .output()
        .await
        .map_err(|e| ActionErrorKind::command(command, e))?;

    match output.status.success() {
        true => {
            tracing::trace!("Command executed successfully: {:?}", command);

            Ok(output)
        }
        false => Err(ActionErrorKind::command_output(command, output)),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OnMissing {
    Ignore,
    // Error,
}

pub(crate) async fn remove_file(path: &Path, on_missing: OnMissing) -> std::io::Result<()> {
    tracing::trace!("Removing file");

    let res = tokio::fs::remove_file(path).await;
    match res {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && on_missing == OnMissing::Ignore => {
            tracing::trace!("Ignoring nonexistent file");
            Ok(())
        }
        e @ Err(_) => e,
    }
}
