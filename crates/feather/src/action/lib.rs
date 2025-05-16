use std::process::Output;

use tokio::process::Command;

use crate::action::ActionErrorKind;

use super::stateful::ActionResult;

pub(crate) async fn execute_command(command: &mut Command) -> ActionResult<Output> {
    log::debug!("Executing command: {:?}", command);

    let output = command
        .output()
        .await
        .map_err(|e| ActionErrorKind::command(command, e))?;

    match output.status.success() {
        true => {
            log::debug!("Command executed successfully: {:?}", command);

            Ok(output)
        },
        false => Err(ActionErrorKind::command_output(command, output)),
    }
}
