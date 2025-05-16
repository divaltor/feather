use std::path::PathBuf;

use anyhow::Result;

use crate::{
    action::{
        Action, ActionErrorKind, StatefulAction,
        base::{
            create_directory::CreateDirectory, create_feather_env::CreateFeatherEnvAction,
            create_file::CreateFile, install_java::InstallJava,
        },
    },
    cache::CacheManager,
    cli,
    modpack::MinecraftProfile,
};

#[tracing::instrument(level = "debug", skip_all, fields(profile = %profile.snapshot(), working_dir = %working_dir.display()))]
pub async fn plan_specific_actions(
    init_args: &cli::InitArgs,
    profile: &MinecraftProfile,
    working_dir: PathBuf,
    cache_manager: CacheManager,
) -> Result<Vec<StatefulAction<Box<dyn Action>>>, ActionErrorKind> {
    let mut planned_actions: Vec<StatefulAction<Box<dyn Action>>> = Vec::new();

    planned_actions.push(CreateDirectory::default(&working_dir).await?.boxed());

    let java_action_plan =
        InstallJava::plan(profile.version.clone(), cache_manager.clone()).await?;
    planned_actions.push(java_action_plan.boxed());

    let client_actions = profile.plan(&working_dir).await?;

    planned_actions.extend(client_actions);

    let eula_action_plan =
        CreateFile::default(&working_dir.join("eula.txt"), "eula=true".to_string()).await?;
    planned_actions.push(eula_action_plan.boxed());

    let java_version_for_env = cache_manager.determine_java_version(&profile.version);
    let java_bin_for_env = cache_manager
        .get_java_executable(&java_version_for_env)
        .to_string_lossy()
        .to_string();

    let feather_env_action_plan = CreateFeatherEnvAction::plan(
        working_dir.clone(),
        java_bin_for_env,
        init_args.java_args.join(" "),
        "server.jar".to_string(),
    )
    .await?;
    planned_actions.push(feather_env_action_plan.boxed());

    Ok(planned_actions)
}
