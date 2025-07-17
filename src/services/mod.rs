pub mod review_service;

use crate::database::Database;
use crate::event::{AppEvent, EventHandler};

pub use review_service::ReviewCreateData;
pub use review_service::ReviewService;
pub use review_service::ReviewsLoadingState;

/// Trait for services that can handle app events
#[allow(async_fn_in_trait)]
pub trait ServiceHandler {
    /// Handle an app event and potentially send new events through the event handler
    async fn handle_app_event(
        event: &AppEvent,
        database: &Database,
        events: &mut EventHandler,
    ) -> color_eyre::Result<()>;
}
