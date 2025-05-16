use anyhow::Result;
use nix::unistd::Group;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::action::{lib::execute_command, Action, ActionErrorKind, StatefulAction};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroup {
    pub name: String,
    pub gid: u32,
}

impl CreateGroup {
    pub fn plan(name: String, gid: u32) -> Result<StatefulAction<Self>, ActionErrorKind> {
        match std::env::consts::OS {
            "macos" => panic!("macOS does not support creating groups"),
            _ => {
                if !(which::which("groupadd").is_ok() || which::which("addgroup").is_ok()) {
                    return Err(ActionErrorKind::MissingGroupCreationCommand);
                }
                
                if !(which::which("groupdel").is_ok() || which::which("delgroup").is_ok()) {
                    return Err(ActionErrorKind::MissingGroupDeletionCommand);
                }
            }
        }
        
        // Ensure group exists
        if let Some(group) = Group::from_name(&name)
            .map_err(|e| ActionErrorKind::GettingGroupId(name.clone(), e))? {
            if group.gid.as_raw() != gid {
                return Err(ActionErrorKind::GroupGidMismatch(name, group.gid.as_raw(), gid));
            }
            
            return Ok(StatefulAction::completed(Self { name, gid }));
        }

        Ok(StatefulAction::uncompleted(Self { name, gid }))
    }
}

#[async_trait::async_trait]
impl Action for CreateGroup {
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        let Self { name, gid } = self;

        match std::env::consts::OS {
            "macos" => panic!("macOS does not support creating groups"),
            _ => {
                if which::which("groupadd").is_ok() {
                    execute_command(
                        Command::new("groupadd")
                            .args([
                                "-g",
                                &format!("{gid}"),
                                name,
                            ])
                            .stdin(std::process::Stdio::null())
                    ).await?;
                } else if which::which("addgroup").is_ok() {
                    execute_command(
                        Command::new("addgroup")
                            .args([
                                &format!("{gid}"),
                                name,
                            ])
                    ).await?;
                } else {
                    return Err(ActionErrorKind::MissingGroupCreationCommand);
                }
            }
        }
        
        Ok(())
    }
    
    async fn revert(&self) -> Result<(), ActionErrorKind> {
        match std::env::consts::OS {
            "linux" => {
                if which::which("groupdel").is_ok() {
                    execute_command(
                        Command::new("groupdel")
                            .process_group(0)
                            .args([&self.name])
                            .stdin(std::process::Stdio::null())
                    ).await?;
                } else if which::which("delgroup").is_ok() {
                    execute_command(
                        Command::new("delgroup")
                            .process_group(0)
                            .args([&self.name])
                            .stdin(std::process::Stdio::null())
                    ).await?;
                } else {
                    return Err(ActionErrorKind::MissingGroupDeletionCommand);
                }
            },
            _ => panic!("macOS does not support deleting groups"),
        }
        Ok(())
    }
}
