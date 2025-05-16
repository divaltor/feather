use anyhow::Result;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

use crate::action::{
    Action, ActionErrorKind, StatefulAction,
    lib::{OnMissing, remove_file},
};

#[derive(Debug, Clone)]
pub struct CreateFeatherEnvAction {
    working_dir: PathBuf,
    java_bin: String,
    java_args: String,
    minecraft_jar: String,
}

impl CreateFeatherEnvAction {
    pub async fn plan(
        working_dir: PathBuf,
        java_bin: String,
        java_args: String,
        minecraft_jar: String,
    ) -> Result<StatefulAction<Self>, ActionErrorKind> {
        let this = Self {
            working_dir: working_dir.clone(),
            java_bin,
            java_args,
            minecraft_jar,
        };

        let env_file_path = working_dir.join("feather.env");
        let expected_content = this.generate_content();

        // TODO: Don't override file because we can change something in the future
        if tokio::fs::try_exists(&env_file_path).await? {
            match tokio::fs::read_to_string(&env_file_path).await {
                Ok(current_content) => {
                    if current_content == expected_content {
                        Ok(StatefulAction::completed(this))
                    } else {
                        // Content differs, plan to overwrite
                        tracing::debug!("feather.env content differs, will be overwritten.");
                        Ok(StatefulAction::uncompleted(this))
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to read existing feather.env at {}: {}. Will attempt to overwrite.",
                        env_file_path.display(),
                        e
                    );
                    Ok(StatefulAction::uncompleted(this)) // Plan to create/overwrite
                }
            }
        } else {
            Ok(StatefulAction::uncompleted(this))
        }
    }

    fn generate_content(&self) -> String {
        format!(
            "JAVA_BIN={}\nJAVA_ARGS={}\nMINECRAFT_JAR={}\n",
            self.java_bin, self.java_args, self.minecraft_jar
        )
    }
}

#[async_trait::async_trait]
impl Action for CreateFeatherEnvAction {
    #[tracing::instrument(level = "debug", skip_all, fields(working_dir = %self.working_dir.display()))]
    async fn execute(&self) -> Result<(), ActionErrorKind> {
        let env_file_path = self.working_dir.join("feather.env");

        let content = self.generate_content();

        let mut file = tokio::fs::File::create(&env_file_path)
            .await
            .map_err(|e| ActionErrorKind::Write(env_file_path.clone(), e))?;
        file.write_all(content.as_bytes())
            .await
            .map_err(|e| ActionErrorKind::Write(env_file_path.clone(), e))?;

        tracing::info!("Created feather.env in {}", self.working_dir.display());
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all, fields(working_dir = %self.working_dir.display()))]
    async fn revert(&self) -> Result<(), ActionErrorKind> {
        let env_file_path = self.working_dir.join("feather.env");

        remove_file(&env_file_path, OnMissing::Ignore)
            .await
            .map_err(|e| ActionErrorKind::Remove(env_file_path, e))?;

        Ok(())
    }
}
