use std::{fmt::Display, path};

use gitbutler_core::{projects::ProjectId, sessions};

/// An event for internal use, as merge between [super::file_monitor::Event] and [Event].
#[derive(Debug)]
pub(super) enum InternalEvent {
    // From public API
    Flush(ProjectId, sessions::Session),
    CalculateVirtualBranches(ProjectId),
    FetchGitbutlerData(ProjectId),
    PushGitbutlerData(ProjectId),

    // From file monitor
    GitFileChange(ProjectId, path::PathBuf),
    ProjectFileChange(ProjectId, path::PathBuf),
}

/// This type captures all operations that can be fed into a watcher that runs in the background.
// TODO(ST): This should not have to be implemented in the Watcher, figure out how this can be moved
//           to application logic at least. However, it's called through a trait in `core`.
#[derive(Debug, PartialEq, Clone)]
pub enum Event {
    Flush(ProjectId, sessions::Session),
    CalculateVirtualBranches(ProjectId),
    FetchGitbutlerData(ProjectId),
    PushGitbutlerData(ProjectId),
}

impl Event {
    pub fn project_id(&self) -> &ProjectId {
        match self {
            Event::FetchGitbutlerData(project_id)
            | Event::Flush(project_id, _)
            | Event::CalculateVirtualBranches(project_id)
            | Event::PushGitbutlerData(project_id) => project_id,
        }
    }
}

impl From<Event> for InternalEvent {
    fn from(value: Event) -> Self {
        match value {
            Event::Flush(a, b) => InternalEvent::Flush(a, b),
            Event::CalculateVirtualBranches(v) => InternalEvent::CalculateVirtualBranches(v),
            Event::FetchGitbutlerData(v) => InternalEvent::FetchGitbutlerData(v),
            Event::PushGitbutlerData(v) => InternalEvent::PushGitbutlerData(v),
        }
    }
}

impl Display for InternalEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InternalEvent::FetchGitbutlerData(pid) => {
                write!(f, "FetchGitbutlerData({})", pid,)
            }
            InternalEvent::Flush(project_id, session) => {
                write!(f, "Flush({}, {})", project_id, session.id)
            }
            InternalEvent::GitFileChange(project_id, path) => {
                write!(f, "GitFileChange({}, {})", project_id, path.display())
            }
            InternalEvent::ProjectFileChange(project_id, path) => {
                write!(f, "ProjectFileChange({}, {})", project_id, path.display())
            }
            InternalEvent::CalculateVirtualBranches(pid) => write!(f, "VirtualBranch({})", pid),
            InternalEvent::PushGitbutlerData(pid) => write!(f, "PushGitbutlerData({})", pid),
        }
    }
}
