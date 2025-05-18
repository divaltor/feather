use crate::action::Action;
use crate::action::base::configure_systemd::ConfigureSystemdService;
use crate::action::base::create_directory::CreateDirectory;
use crate::action::base::create_user::CreateUser;
use crate::action::base::start_systemd::StartSystemd;
use crate::action::stateful::{ActionErrorKind, StatefulAction};
use crate::cache::CacheManager;
use crate::cli;
use crate::modpack::MinecraftProfile;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ServerSetupManager {
    actions: Vec<StatefulAction<Box<dyn Action>>>,
}

impl ServerSetupManager {
    pub async fn new(
        args: &cli::InitArgs,
        java_cache_dir: &PathBuf,
        minecraft_servers_root_dir: &PathBuf,
    ) -> Result<Self, ActionErrorKind> {
        let mut actions: Vec<StatefulAction<Box<dyn Action>>> = Vec::new();

        let profile = MinecraftProfile::try_import(&args.file)?;

        // TODO: Add grant permissions for the calling user
        actions.push(CreateUser::plan("feather").await?.boxed());
        actions.push(CreateDirectory::default(&java_cache_dir).await?.boxed());
        actions.push(
            CreateDirectory::default(&minecraft_servers_root_dir)
                .await?
                .boxed(),
        );

        actions.push(ConfigureSystemdService::default().await?.boxed());

        let server_working_dir = minecraft_servers_root_dir.join(profile.hash());

        let cache_manager = CacheManager::new(java_cache_dir);

        let mc_specific_actions = crate::action::base::install_minecraft::plan_specific_actions(
            args,
            &profile,
            server_working_dir.clone(),
            cache_manager,
        )
        .await?;

        actions.extend(mc_specific_actions);

        Ok(Self { actions })
    }

    #[tracing::instrument(level = "info", skip_all)]
    pub async fn install(&mut self) -> Result<(), ActionErrorKind> {
        tracing::info!("Starting server setup process...");

        let actions_len = self.actions.len();

        for (index, action_state) in self.actions.iter_mut().enumerate() {
            let action_description = format!("{:?}", action_state.inner_ref());
            tracing::info!(
                "Executing action {}/{}: {}",
                index + 1,
                actions_len,
                action_description
            );
            match action_state.try_execute().await {
                Ok(_) => {
                    tracing::info!(
                        "Action {}/{} completed successfully.",
                        index + 1,
                        actions_len
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Error executing action {}/{}: {}. Attempting to revert...",
                        index + 1,
                        actions_len,
                        action_description,
                    );
                    self.revert_up_to(index).await;
                    return Err(e);
                }
            }
        }
        tracing::info!("Server setup process completed successfully.");
        Ok(())
    }

    #[tracing::instrument(level = "info", skip_all)]
    pub async fn revert(&mut self) -> Result<(), ActionErrorKind> {
        tracing::info!("Starting rollback process for all actions...");

        let actions_len = self.actions.len();

        for (index, action_state) in self.actions.iter_mut().rev().enumerate() {
            let action_description = format!("{:?}", action_state.inner_ref());

            tracing::info!(
                "Reverting action {}/{}: {}",
                index + 1,
                actions_len,
                action_description
            );

            if let Err(revert_err) = action_state.try_revert().await {
                tracing::error!(
                    "Error reverting action {}/{}: {}. Error: {}. Continuing rollback...",
                    index + 1,
                    actions_len,
                    action_description,
                    revert_err
                );
            }
        }

        tracing::info!("Rollback process completed.");

        Ok(())
    }

    async fn revert_up_to(&mut self, up_to_index: usize) {
        tracing::info!("Reverting actions up to index {}...", up_to_index);

        for i in (0..=up_to_index).rev() {
            if let Some(action_state) = self.actions.get_mut(i) {
                let action_description = format!("{:?}", action_state.inner_ref());
                tracing::info!("Reverting action (index {}): {}", i, action_description);
                if let Err(revert_err) = action_state.try_revert().await {
                    tracing::error!(
                        "Error reverting action (index {}): {}. Error: {}. Continuing rollback...",
                        i,
                        action_description,
                        revert_err
                    );
                }
            } else {
                tracing::warn!(
                    "Attempted to revert action at index {}, but it was out of bounds.",
                    i
                );
            }
        }
    }
}
