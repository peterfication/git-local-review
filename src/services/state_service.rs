use std::sync::Arc;
use tokio::sync::RwLock;

use crate::database::Database;
use crate::event::{AppEvent, EventHandler};
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

/// Events for requesting and updating state
#[derive(Debug, Clone)]
pub enum StateEvent {
    /// Request reviews data (loads if not cached)
    ReviewsRequest,
    /// Request git branches data (loads if not cached)
    GitBranchesRequest,
    /// Internal event when reviews are loaded
    ReviewsLoaded(ReviewsLoadingState),
    /// Internal event when git branches are loaded
    GitBranchesLoaded(GitBranchesLoadingState),
}

/// Service that manages global application state and provides caching
pub struct StateService {
    state: Arc<RwLock<AppState>>,
}

impl StateService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AppState::default())),
        }
    }

    /// Get current state snapshot
    pub async fn get_state(&self) -> AppState {
        self.state.read().await.clone()
    }

    /// Handle state-related events
    pub async fn handle_state_event(
        &self,
        event: &StateEvent,
        _database: &Database,
        events: &mut EventHandler,
    ) -> color_eyre::Result<()> {
        match event {
            StateEvent::ReviewsRequest => {
                let current_state = self.state.read().await.reviews.clone();
                match current_state {
                    ReviewsLoadingState::Init => {
                        // Data not loaded yet, trigger loading
                        events.send_async(AppEvent::ReviewsLoad).await?;
                    }
                    _ => {
                        // Data already available or loading, broadcast current state
                        let full_state = self.state.read().await.clone();
                        events
                            .send_async(AppEvent::StateUpdate(Arc::new(full_state)))
                            .await?;
                    }
                }
            }
            StateEvent::GitBranchesRequest => {
                let current_state = self.state.read().await.git_branches.clone();
                match current_state {
                    GitBranchesLoadingState::Init => {
                        // Data not loaded yet, trigger loading
                        events.send_async(AppEvent::GitBranchesLoad).await?;
                    }
                    _ => {
                        // Data already available or loading, broadcast current state
                        let full_state = self.state.read().await.clone();
                        events
                            .send_async(AppEvent::StateUpdate(Arc::new(full_state)))
                            .await?;
                    }
                }
            }
            StateEvent::ReviewsLoaded(reviews_state) => {
                // Update internal state
                {
                    let mut state = self.state.write().await;
                    state.reviews = reviews_state.clone();
                }
                // Broadcast updated state to all views
                let full_state = self.state.read().await.clone();
                events
                    .send_async(AppEvent::StateUpdate(Arc::new(full_state)))
                    .await?;
            }
            StateEvent::GitBranchesLoaded(branches_state) => {
                // Update internal state
                {
                    let mut state = self.state.write().await;
                    state.git_branches = branches_state.clone();
                }
                // Broadcast updated state to all views
                let full_state = self.state.read().await.clone();
                events
                    .send_async(AppEvent::StateUpdate(Arc::new(full_state)))
                    .await?;
            }
        }
        Ok(())
    }

    /// Clear cached data (useful for testing or forced refresh)
    pub async fn clear_cache(&self) {
        let mut state = self.state.write().await;
        *state = AppState::default();
    }

    /// Get reviews state specifically
    pub async fn get_reviews_state(&self) -> ReviewsLoadingState {
        self.state.read().await.reviews.clone()
    }

    /// Get git branches state specifically
    pub async fn get_git_branches_state(&self) -> GitBranchesLoadingState {
        self.state.read().await.git_branches.clone()
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
    use crate::event::Event;
    use crate::services::{GitBranchesLoadingState, ReviewsLoadingState};

    #[tokio::test]
    async fn test_initial_state() {
        let service = StateService::new();
        let state = service.get_state().await;

        assert!(matches!(state.reviews, ReviewsLoadingState::Init));
        assert!(matches!(state.git_branches, GitBranchesLoadingState::Init));
    }

    #[tokio::test]
    async fn test_reviews_state_update() {
        let service = StateService::new();
        let mut events = EventHandler::new_for_test();
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let database = Database::from_pool(pool);

        // Simulate reviews loaded
        let reviews = vec![].into();
        let loaded_state = ReviewsLoadingState::Loaded(reviews);
        let event = StateEvent::ReviewsLoaded(loaded_state.clone());

        service
            .handle_state_event(&event, &database, &mut events)
            .await
            .unwrap();

        let state = service.get_state().await;
        assert!(matches!(state.reviews, ReviewsLoadingState::Loaded(_)));

        // Should broadcast state update
        let broadcast_event = events.try_recv().unwrap();
        assert!(matches!(
            *broadcast_event,
            Event::App(AppEvent::StateUpdate(_))
        ));
    }

    #[tokio::test]
    async fn test_git_branches_state_update() {
        let service = StateService::new();
        let mut events = EventHandler::new_for_test();
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let database = Database::from_pool(pool);

        // Simulate git branches loaded
        let branches = vec!["main".to_string(), "feature".to_string()].into();
        let loaded_state = GitBranchesLoadingState::Loaded(branches);
        let event = StateEvent::GitBranchesLoaded(loaded_state.clone());

        service
            .handle_state_event(&event, &database, &mut events)
            .await
            .unwrap();

        let state = service.get_state().await;
        assert!(matches!(
            state.git_branches,
            GitBranchesLoadingState::Loaded(_)
        ));
    }

    #[tokio::test]
    async fn test_reviews_request_when_init() {
        let service = StateService::new();
        let mut events = EventHandler::new_for_test();
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let database = Database::from_pool(pool);

        let event = StateEvent::ReviewsRequest;
        service
            .handle_state_event(&event, &database, &mut events)
            .await
            .unwrap();

        // Should trigger loading
        let load_event = events.try_recv().unwrap();
        assert!(matches!(*load_event, Event::App(AppEvent::ReviewsLoad)));
    }

    #[tokio::test]
    async fn test_reviews_request_when_loaded() {
        let service = StateService::new();
        let mut events = EventHandler::new_for_test();
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let database = Database::from_pool(pool);

        // First load some data
        let reviews = vec![].into();
        let loaded_state = ReviewsLoadingState::Loaded(reviews);
        let load_event = StateEvent::ReviewsLoaded(loaded_state);
        service
            .handle_state_event(&load_event, &database, &mut events)
            .await
            .unwrap();
        events.try_recv().unwrap(); // Clear the broadcast event

        // Now request reviews again
        let request_event = StateEvent::ReviewsRequest;
        service
            .handle_state_event(&request_event, &database, &mut events)
            .await
            .unwrap();

        // Should broadcast current state instead of loading
        let broadcast_event = events.try_recv().unwrap();
        assert!(matches!(
            *broadcast_event,
            Event::App(AppEvent::StateUpdate(_))
        ));
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let service = StateService::new();
        let mut events = EventHandler::new_for_test();
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let database = Database::from_pool(pool);

        // Load some data
        let reviews = vec![].into();
        let loaded_state = ReviewsLoadingState::Loaded(reviews);
        let event = StateEvent::ReviewsLoaded(loaded_state);
        service
            .handle_state_event(&event, &database, &mut events)
            .await
            .unwrap();

        // Clear cache
        service.clear_cache().await;

        let state = service.get_state().await;
        assert!(matches!(state.reviews, ReviewsLoadingState::Init));
        assert!(matches!(state.git_branches, GitBranchesLoadingState::Init));
    }
}
