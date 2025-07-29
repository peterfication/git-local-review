use std::future::Future;

use crate::database::Database;
use crate::event::{AppEvent, EventHandler};

pub mod comment_service;
pub mod file_view_service;
pub mod git_service;
pub mod review_service;

pub use comment_service::CommentService;
pub use comment_service::CommentsLoadingState;
pub use file_view_service::FileViewService;
pub use git_service::GitBranchesLoadingState;
pub use git_service::GitDiffLoadingState;
pub use git_service::GitService;
pub use review_service::ReviewCreateData;
pub use review_service::ReviewLoadingState;
pub use review_service::ReviewService;
pub use review_service::ReviewsLoadingState;

/// Trait for services that can handle app events
pub trait ServiceHandler {
    /// Handle an app event and potentially send new events through the event handler
    fn handle_app_event<'a>(
        event: &'a AppEvent,
        database: &'a Database,
        events: &'a mut EventHandler,
    ) -> std::pin::Pin<Box<dyn Future<Output = color_eyre::Result<()>> + Send + 'a>>;
}
