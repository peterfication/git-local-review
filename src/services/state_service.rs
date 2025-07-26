use futures::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::event::AppEvent;
use crate::services::{GitBranchesLoadingState, ReviewsLoadingState};

/// Centralized application state that can be shared across views
#[derive(Debug, Clone)]
pub struct AppState {
    pub reviews: ReviewsLoadingState,
    pub git_branches: GitBranchesLoadingState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            reviews: ReviewsLoadingState::Init,
            git_branches: GitBranchesLoadingState::Init,
        }
    }
}

/// Service that manages global application state and provides caching
pub struct StateService {
    state: Arc<AppState>,
}

impl StateService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AppState::default()),
        }
    }

    /// Get current state snapshot
    pub fn get_state(&self) -> &Arc<AppState> {
        &self.state
    }

    /// Clear cached data (useful for testing or forced refresh)
    pub async fn clear_cache(&mut self) {
        self.state = Arc::new(AppState::default());
    }

    pub fn handle_app_event<'a>(
        &'a mut self,
        event: &'a AppEvent,
    ) -> Pin<Box<dyn Future<Output = color_eyre::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match event {
                AppEvent::ReviewsLoadingState(reviews_state) => {
                    // Clone the current state inside the Arc to modify it
                    let old_state = self.get_state();
                    let app_state_arc = old_state.clone();
                    let mut app_state = (*app_state_arc).clone();
                    app_state.reviews = reviews_state.clone();
                    self.state = Arc::new(app_state);
                }
                AppEvent::GitBranchesLoadingState(branches_state) => {
                    // Clone the current state inside the Arc to modify it
                    let old_state = self.get_state();
                    let app_state_arc = old_state.clone();
                    let mut app_state = (*app_state_arc).clone();
                    app_state.git_branches = branches_state.clone();
                    self.state = Arc::new(app_state);
                }
                _ => {
                    // Other events are ignored
                }
            }
            Ok(())
        })
    }
}

impl Default for StateService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{GitBranchesLoadingState, ReviewsLoadingState};

    #[tokio::test]
    async fn test_initial_state() {
        let service = StateService::new();
        let state = service.get_state();

        assert!(matches!(state.reviews, ReviewsLoadingState::Init));
        assert!(matches!(state.git_branches, GitBranchesLoadingState::Init));
    }
}
