use anyhow::Result;
use compact_str::CompactString;
use nix::unistd::{Group, User};
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt, chown},
    path::{Path, PathBuf},
};

use crate::action::{Action, ActionErrorKind, StatefulAction};

#[derive(Debug, Clone)]
pub struct CreateDirectory {
    path: PathBuf,
    user: Option<CompactString>,
    group: Option<CompactString>,
    mode: Option<u32>,
}

impl CreateDirectory {
    pub async fn default<P: AsRef<Path>>(path: P) -> Result<StatefulAction<Self>, ActionErrorKind> {
        Self::plan(path, Some("feather".into()), Some("feather".into()), None).await
    }

    pub async fn plan<P: AsRef<Path>>(
        path: P,
        user: Option<CompactString>,
        group: Option<CompactString>,
        mode: Option<u32>,
    ) -> Result<StatefulAction<Self>, ActionErrorKind> {
        let this = Self {
            path: path.as_ref().to_path_buf(),
            user,
            group,
            mode,
        };

        if !this.path.exists() {
            return Ok(StatefulAction::uncompleted(this));
        }

        let metadata = tokio::fs::metadata(&this.path).await?;

        if !metadata.is_dir() {
            return Err(anyhow::anyhow!("Path {} is not a directory", this.path.display()).into());
        }

        if let Some(user) = &this.user {
            let expected_uid = User::from_name(user.as_str())
                .map_err(|e| ActionErrorKind::GettingUserId(user.clone(), e))?
                .ok_or_else(|| ActionErrorKind::UserNotFound(user.clone()))?;

            if metadata.uid() != expected_uid.uid.as_raw() {
                return Err(anyhow::anyhow!(
                    "Path {} is not owned by user {}",
                    this.path.display(),
                    user
                )
                .into());
            }
        }

        if let Some(group) = &this.group {
            let expected_gid = Group::from_name(group.as_str())
                .map_err(|e| ActionErrorKind::GettingGroupId(group.clone(), e))?
                .ok_or_else(|| ActionErrorKind::GroupNotFound(group.clone()))?;

            if metadata.gid() != expected_gid.gid.as_raw() {
                return Err(anyhow::anyhow!(
                    "Path {} is not owned by group {}",
                    this.path.display(),
                    group
                )
                .into());
            }
        }

        // Always plan as uncompleted, as tokio::fs::create_dir_all is idempotent.
        // It will do nothing if the directory already exists and is a directory.
        // If it exists and is a file, it will error out during execute, which is appropriate.
        Ok(StatefulAction::uncompleted(this))
    }
}

#[async_trait::async_trait]
impl Action for CreateDirectory {
    #[tracing::instrument(level = "debug", skip_all, fields(path = %self.path.display()))]
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        tokio::fs::create_dir_all(&self.path).await?;

        let uid = if let Some(user) = &self.user {
            Some(
                User::from_name(user.as_str())
                    .map_err(|e| ActionErrorKind::GettingUserId(user.clone(), e))?
                    .ok_or_else(|| ActionErrorKind::UserNotFound(user.clone()))?
                    .uid
                    .as_raw(),
            )
        } else {
            None
        };

        let gid = if let Some(group) = &self.group {
            Some(
                Group::from_name(group.as_str())
                    .map_err(|e| ActionErrorKind::GettingGroupId(group.clone(), e))?
                    .ok_or_else(|| ActionErrorKind::GroupNotFound(group.clone()))?
                    .gid
                    .as_raw(),
            )
        } else {
            None
        };

        chown(&self.path, uid, gid)?;

        if let Some(mode) = self.mode {
            tokio::fs::set_permissions(&self.path, PermissionsExt::from_mode(mode)).await?;
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(path = %self.path.display()))]
    async fn revert(&self) -> Result<(), ActionErrorKind> {
        // TODO: idk, may be we should remove the directory?
        // Because we usually call `revert` only if `execute` failed.
        // Reverting directory creation is generally a no-op to avoid accidental data loss.
        tracing::warn!(
            "Revert for CreateDirectoryAction on path {} is a no-op.",
            self.path.display()
        );
        Ok(())
    }
}
