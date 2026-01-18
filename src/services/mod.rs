use std::{future::Future, pin::Pin};

use crate::database::Database;
use crate::event::{AppEvent, EventHandler};

pub mod branch_status_service;
pub mod comment_service;
pub mod file_view_service;
pub mod git_service;
pub mod review_service;

pub use branch_status_service::BranchStatusService;
pub use comment_service::CommentService;
pub use comment_service::CommentsLoadParams;
pub use comment_service::CommentsLoadingState;
pub use file_view_service::FileViewService;
pub use git_service::GitBranchesLoadingState;
pub use git_service::GitDiffLoadingState;
pub use git_service::GitService;
pub use review_service::ReviewCreateData;
pub use review_service::ReviewLoadingState;
pub use review_service::ReviewService;
pub use review_service::ReviewsLoadingState;

/// Context struct containing the app state that services need access to
pub struct ServiceContext<'a> {
    pub database: &'a Database,
    pub repo_path: &'a str,
    pub events: &'a mut EventHandler,
}

/// Trait for services that can handle app events
pub trait ServiceHandler {
    /// Handle an app event and potentially send new events through the event handler
    fn handle_app_event<'a>(
        event: &'a AppEvent,
        context: ServiceContext<'a>,
    ) -> Pin<Box<dyn Future<Output = color_eyre::Result<()>> + Send + 'a>>;
}
