use std::{
    path::{self, PathBuf},
    time,
};

use serde::{Deserialize, Serialize};

use crate::{
    git, id::Id, types::default_true::DefaultTrue, virtual_branches::VirtualBranchesHandle,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AuthKey {
    Default,
    Generated,
    #[default]
    SystemExecutable,
    GitCredentialsHelper,
    Local {
        private_key_path: path::PathBuf,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiProject {
    pub name: String,
    pub description: Option<String>,
    pub repository_id: String,
    pub git_url: String,
    pub code_git_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub sync: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FetchResult {
    Fetched {
        timestamp: time::SystemTime,
    },
    Error {
        timestamp: time::SystemTime,
        error: String,
    },
}

impl FetchResult {
    pub fn timestamp(&self) -> &time::SystemTime {
        match self {
            FetchResult::Fetched { timestamp } | FetchResult::Error { timestamp, .. } => timestamp,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub struct CodePushState {
    pub id: git::Oid,
    pub timestamp: time::SystemTime,
}

pub type ProjectId = Id<Project>;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Project {
    pub id: ProjectId,
    pub title: String,
    pub description: Option<String>,
    /// The worktree directory of the project's repository.
    pub path: path::PathBuf,
    #[serde(default)]
    pub preferred_key: AuthKey,
    /// if ok_with_force_push is true, we'll not try to avoid force pushing
    /// for example, when updating base branch
    #[serde(default)]
    pub ok_with_force_push: DefaultTrue,
    pub api: Option<ApiProject>,
    #[serde(default)]
    pub gitbutler_data_last_fetch: Option<FetchResult>,
    #[serde(default)]
    pub gitbutler_code_push_state: Option<CodePushState>,
    #[serde(default)]
    pub project_data_last_fetch: Option<FetchResult>,
    #[serde(default)]
    pub omit_certificate_check: Option<bool>,
    #[serde(default)]
    pub enable_snapshots: Option<bool>,
    // The number of changed lines that will trigger a snapshot
    #[serde(default = "default_snapshot_lines_threshold")]
    pub snapshot_lines_threshold: usize,
}

fn default_snapshot_lines_threshold() -> usize {
    20
}

impl Project {
    pub fn is_sync_enabled(&self) -> bool {
        self.api.as_ref().map(|api| api.sync).unwrap_or_default()
    }

    pub fn has_code_url(&self) -> bool {
        self.api
            .as_ref()
            .map(|api| api.code_git_url.is_some())
            .unwrap_or_default()
    }

    /// Returns the path to the directory containing the `GitButler` state for this project.
    ///
    /// Normally this is `.git/gitbutler` in the project's repository.
    pub fn gb_dir(&self) -> PathBuf {
        self.path.join(".git").join("gitbutler")
    }

    /// Returns a handle to the virtual branches manager of the project.
    pub fn virtual_branches(&self) -> VirtualBranchesHandle {
        VirtualBranchesHandle::new(self.gb_dir())
    }
}
