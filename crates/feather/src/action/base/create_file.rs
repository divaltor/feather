use compact_str::CompactString;
use nix::unistd::{Group, User};
use std::{
    os::unix::fs::{PermissionsExt, chown},
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

use crate::action::{Action, ActionErrorKind, StatefulAction};

#[derive(Debug, Clone)]
pub struct CreateFile {
    path: PathBuf,
    content: String,
    user: Option<CompactString>,
    group: Option<CompactString>,
    mode: Option<u32>,
}

impl CreateFile {
    pub async fn default<P: AsRef<Path>>(
        path: P,
        content: impl AsRef<str>,
    ) -> Result<StatefulAction<Self>, ActionErrorKind> {
        Self::plan(
            path,
            content.as_ref().to_string(),
            Some("feather".into()),
            Some("feather".into()),
            None,
        )
        .await
    }

    #[tracing::instrument(level = "debug", skip_all)]
    pub async fn plan<P: AsRef<Path>>(
        path: P,
        content: String,
        user: Option<CompactString>,
        group: Option<CompactString>,
        mode: Option<u32>,
    ) -> Result<StatefulAction<Self>, ActionErrorKind> {
        let this = Self {
            path: path.as_ref().to_path_buf(),
            content,
            user,
            group,
            mode,
        };

        if let Ok(metadata) = tokio::fs::metadata(&this.path).await {
            if metadata.is_dir() {
                return Err(anyhow::anyhow!(
                    "Cannot create file at {}: path exists and is a directory.",
                    this.path.display()
                )
                .into());
            }
        }

        Ok(StatefulAction::uncompleted(this))
    }
}

#[async_trait::async_trait]
impl Action for CreateFile {
    #[tracing::instrument(level = "debug", skip_all, fields(path = %self.path.display()))]
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        let mut file = tokio::fs::File::create(&self.path).await?;

        file.write_all(self.content.as_bytes()).await?;
        file.sync_all().await?;

        let uid_to_set: Option<u32> = if let Some(user_name) = &self.user {
            Some(
                User::from_name(user_name.as_str())
                    .map_err(|e| ActionErrorKind::GettingUserId(user_name.clone(), e))?
                    .ok_or_else(|| ActionErrorKind::UserNotFound(user_name.clone()))?
                    .uid
                    .as_raw(),
            )
        } else {
            None
        };

        let gid_to_set: Option<u32> = if let Some(group_name) = &self.group {
            Some(
                Group::from_name(group_name.as_str())
                    .map_err(|e| ActionErrorKind::GettingGroupId(group_name.clone(), e))?
                    .ok_or_else(|| ActionErrorKind::GroupNotFound(group_name.clone()))?
                    .gid
                    .as_raw(),
            )
        } else {
            None
        };

        if uid_to_set.is_some() || gid_to_set.is_some() {
            std::os::unix::fs::chown(&self.path, uid_to_set, gid_to_set)
                .map_err(ActionErrorKind::Io)?;
        }

        if let Some(mode_val) = self.mode {
            let permissions = std::os::unix::fs::PermissionsExt::from_mode(mode_val);
            tokio::fs::set_permissions(&self.path, permissions).await?;
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(path = %self.path.display()))]
    async fn revert(&self) -> Result<(), ActionErrorKind> {
        match tokio::fs::remove_file(&self.path).await {
            Ok(_) => {
                tracing::debug!(
                    "Successfully removed file {} during revert.",
                    self.path.display()
                );
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(
                    "File {} not found during revert, nothing to do.",
                    self.path.display()
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to remove file {} during revert: {}",
                    self.path.display(),
                    e
                );
                Err(ActionErrorKind::Io(e))
            }
        }
    }
}
