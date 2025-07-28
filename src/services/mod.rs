use std::future::Future;

use crate::database::Database;
use crate::event::{AppEvent, EventHandler};

pub mod git_service;
pub mod review_service;

pub use git_service::GitBranchesLoadingState;
pub use git_service::GitDiffLoadingState;
pub use git_service::GitService;
pub use review_service::ReviewCreateData;
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
