pub mod commands {
    use crate::error::Error;
    use anyhow::Context;
    use gitbutler_core::{
        askpass::AskpassBroker,
        assets,
        error::Code,
        git, projects,
        projects::ProjectId,
        virtual_branches::{
            branch::{self, BranchId, BranchOwnershipClaims},
            controller::Controller,
            BaseBranch, RemoteBranch, RemoteBranchData, RemoteBranchFile, VirtualBranches,
        },
    };
    use tauri::{AppHandle, Manager};
    use tracing::instrument;

    use crate::watcher;

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn commit_virtual_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch: BranchId,
        message: &str,
        ownership: Option<BranchOwnershipClaims>,
        run_hooks: bool,
    ) -> Result<git::Oid, Error> {
        let oid = handle
            .state::<Controller>()
            .create_commit(&project_id, &branch, message, ownership.as_ref(), run_hooks)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(oid)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn list_virtual_branches(
        handle: AppHandle,
        project_id: ProjectId,
    ) -> Result<VirtualBranches, Error> {
        let (branches, skipped_files) = handle
            .state::<Controller>()
            .list_virtual_branches(&project_id)
            .await?;

        let proxy = handle.state::<assets::Proxy>();
        let branches = proxy.proxy_virtual_branches(branches).await;
        Ok(VirtualBranches {
            branches,
            skipped_files,
        })
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn create_virtual_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch: branch::BranchCreateRequest,
    ) -> Result<BranchId, Error> {
        let branch_id = handle
            .state::<Controller>()
            .create_virtual_branch(&project_id, &branch)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(branch_id)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn create_virtual_branch_from_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch: git::Refname,
    ) -> Result<BranchId, Error> {
        let branch_id = handle
            .state::<Controller>()
            .create_virtual_branch_from_branch(&project_id, &branch)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(branch_id)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn merge_virtual_branch_upstream(
        handle: AppHandle,
        project_id: ProjectId,
        branch: BranchId,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .merge_virtual_branch_upstream(&project_id, &branch)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn get_base_branch_data(
        handle: AppHandle,
        project_id: ProjectId,
    ) -> Result<Option<BaseBranch>, Error> {
        if let Some(base_branch) = handle
            .state::<Controller>()
            .get_base_branch_data(&project_id)
            .await?
        {
            let proxy = handle.state::<assets::Proxy>();
            let base_branch = proxy.proxy_base_branch(base_branch).await;
            Ok(Some(base_branch))
        } else {
            Ok(None)
        }
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn set_base_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch: &str,
        push_remote: Option<&str>, // optional different name of a remote to push to (defaults to same as the branch)
    ) -> Result<BaseBranch, Error> {
        dbg!(&project_id, &branch, &push_remote);
        let branch_name = format!("refs/remotes/{}", branch)
            .parse()
            .context("Invalid branch name")?;
        let base_branch = handle
            .state::<Controller>()
            .set_base_branch(&project_id, &branch_name)
            .await?;
        let base_branch = handle
            .state::<assets::Proxy>()
            .proxy_base_branch(base_branch)
            .await;

        // if they also sent a different push remote, set that too
        if let Some(push_remote) = push_remote {
            handle
                .state::<Controller>()
                .set_target_push_remote(&project_id, push_remote)
                .await?;
        }
        emit_vbranches(&handle, &project_id).await;
        Ok(base_branch)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn update_base_branch(handle: AppHandle, project_id: ProjectId) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .update_base_branch(&project_id)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn update_virtual_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch: branch::BranchUpdateRequest,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .update_virtual_branch(&project_id, branch)
            .await?;

        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn delete_virtual_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .delete_virtual_branch(&project_id, &branch_id)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn apply_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch: BranchId,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .apply_virtual_branch(&project_id, &branch)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn unapply_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch: BranchId,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .unapply_virtual_branch(&project_id, &branch)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn unapply_ownership(
        handle: AppHandle,
        project_id: ProjectId,
        ownership: BranchOwnershipClaims,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .unapply_ownership(&project_id, &ownership)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn reset_files(
        handle: AppHandle,
        project_id: ProjectId,
        files: &str,
    ) -> Result<(), Error> {
        // convert files to Vec<String>
        let files = files
            .split('\n')
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>();
        handle
            .state::<Controller>()
            .reset_files(&project_id, &files)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn push_virtual_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        with_force: bool,
    ) -> Result<(), Error> {
        let askpass_broker = handle.state::<AskpassBroker>();
        handle
            .state::<Controller>()
            .push_virtual_branch(
                &project_id,
                &branch_id,
                with_force,
                Some((askpass_broker.inner().clone(), Some(branch_id))),
            )
            .await
            .map_err(|err| err.context(Code::Unknown))?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn can_apply_virtual_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
    ) -> Result<bool, Error> {
        handle
            .state::<Controller>()
            .can_apply_virtual_branch(&project_id, &branch_id)
            .await
            .map_err(Into::into)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn can_apply_remote_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch: git::RemoteRefname,
    ) -> Result<bool, Error> {
        Ok(handle
            .state::<Controller>()
            .can_apply_remote_branch(&project_id, &branch)
            .await?)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn list_remote_commit_files(
        handle: AppHandle,
        project_id: ProjectId,
        commit_oid: git::Oid,
    ) -> Result<Vec<RemoteBranchFile>, Error> {
        handle
            .state::<Controller>()
            .list_remote_commit_files(&project_id, commit_oid)
            .await
            .map_err(Into::into)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn reset_virtual_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        target_commit_oid: git::Oid,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .reset_virtual_branch(&project_id, &branch_id, target_commit_oid)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn cherry_pick_onto_virtual_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        target_commit_oid: git::Oid,
    ) -> Result<Option<git::Oid>, Error> {
        let oid = handle
            .state::<Controller>()
            .cherry_pick(&project_id, &branch_id, target_commit_oid)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(oid)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn amend_virtual_branch(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        commit_oid: git::Oid,
        ownership: BranchOwnershipClaims,
    ) -> Result<git::Oid, Error> {
        let oid = handle
            .state::<Controller>()
            .amend(&project_id, &branch_id, commit_oid, &ownership)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(oid)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn move_commit_file(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        from_commit_oid: git::Oid,
        to_commit_oid: git::Oid,
        ownership: BranchOwnershipClaims,
    ) -> Result<git::Oid, Error> {
        let oid = handle
            .state::<Controller>()
            .move_commit_file(
                &project_id,
                &branch_id,
                from_commit_oid,
                to_commit_oid,
                &ownership,
            )
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(oid)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn undo_commit(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        commit_oid: git::Oid,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .undo_commit(&project_id, &branch_id, commit_oid)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn insert_blank_commit(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        commit_oid: git::Oid,
        offset: i32,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .insert_blank_commit(&project_id, &branch_id, commit_oid, offset)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn reorder_commit(
        handle: AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        commit_oid: git::Oid,
        offset: i32,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .reorder_commit(&project_id, &branch_id, commit_oid, offset)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn list_remote_branches(
        handle: tauri::AppHandle,
        project_id: ProjectId,
    ) -> Result<Vec<RemoteBranch>, Error> {
        let branches = handle
            .state::<Controller>()
            .list_remote_branches(&project_id)
            .await?;
        Ok(branches)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn get_remote_branch_data(
        handle: tauri::AppHandle,
        project_id: ProjectId,
        refname: git::Refname,
    ) -> Result<RemoteBranchData, Error> {
        let branch_data = handle
            .state::<Controller>()
            .get_remote_branch_data(&project_id, &refname)
            .await?;
        let branch_data = handle
            .state::<assets::Proxy>()
            .proxy_remote_branch_data(branch_data)
            .await;
        Ok(branch_data)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn squash_branch_commit(
        handle: tauri::AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        target_commit_oid: git::Oid,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .squash(&project_id, &branch_id, target_commit_oid)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn fetch_from_target(
        handle: tauri::AppHandle,
        project_id: ProjectId,
        action: Option<String>,
    ) -> Result<BaseBranch, Error> {
        let askpass_broker = handle.state::<AskpassBroker>().inner().clone();
        let base_branch = handle
            .state::<Controller>()
            .fetch_from_target(
                &project_id,
                Some((
                    askpass_broker,
                    action.unwrap_or_else(|| "unknown".to_string()),
                )),
            )
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(base_branch)
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn move_commit(
        handle: tauri::AppHandle,
        project_id: ProjectId,
        commit_oid: git::Oid,
        target_branch_id: BranchId,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .move_commit(&project_id, &target_branch_id, commit_oid)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    #[tauri::command(async)]
    #[instrument(skip(handle), err(Debug))]
    pub async fn update_commit_message(
        handle: tauri::AppHandle,
        project_id: ProjectId,
        branch_id: BranchId,
        commit_oid: git::Oid,
        message: &str,
    ) -> Result<(), Error> {
        handle
            .state::<Controller>()
            .update_commit_message(&project_id, &branch_id, commit_oid, message)
            .await?;
        emit_vbranches(&handle, &project_id).await;
        Ok(())
    }

    async fn emit_vbranches(handle: &AppHandle, project_id: &projects::ProjectId) {
        if let Err(error) = handle
            .state::<watcher::Watchers>()
            .post(gitbutler_watcher::Action::CalculateVirtualBranches(
                *project_id,
            ))
            .await
        {
            tracing::error!(?error);
        }
    }
}
