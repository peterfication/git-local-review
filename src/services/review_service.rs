use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    database::Database,
    event::{AppEvent, EventHandler},
    models::{Review, ReviewId},
    services::git_service::GitService,
    time_provider::TimeProvider,
};

use super::{ServiceContext, ServiceHandler};

#[derive(Clone, Debug)]
pub struct ReviewCreateData {
    pub base_branch: String,
    pub target_branch: String,
    pub base_sha: Option<String>,
    pub target_sha: Option<String>,
}

/// State of reviews loading process
#[derive(Debug, Clone, PartialEq)]
pub enum ReviewsLoadingState {
    /// Initial state - no loading has been attempted
    Init,
    /// Currently loading reviews from database
    Loading,
    /// Reviews have been successfully loaded
    Loaded(Arc<[Review]>),
    /// Error occurred during loading
    Error(Arc<str>),
}

/// State of single review loading process
#[derive(Debug, Clone, PartialEq)]
pub enum ReviewLoadingState {
    /// Initial state - no loading has been attempted
    Init,
    /// Currently loading review from database
    Loading,
    /// Review has been successfully loaded
    Loaded(Arc<Review>),
    /// Review was not found
    NotFound(Arc<ReviewId>),
    /// Error occurred during loading
    Error(Arc<str>),
}

pub struct ReviewService {
    // ReviewService can be stateless for now
}

impl Default for ReviewService {
    fn default() -> Self {
        Self::new()
    }
}

impl ReviewService {
    pub fn new() -> Self {
        Self {}
    }
    /// Create a new review and trigger reviews reload
    pub async fn create_review(
        database: &Database,
        data: ReviewCreateData,
        events: &mut EventHandler,
    ) -> color_eyre::Result<Review> {
        if data.base_branch.trim().is_empty() {
            return Err(color_eyre::eyre::eyre!("Base branch cannot be empty"));
        }
        if data.target_branch.trim().is_empty() {
            return Err(color_eyre::eyre::eyre!("Target branch cannot be empty"));
        }

        // Get SHAs from Git if not provided in the data
        let base_sha = if data.base_sha.is_some() {
            data.base_sha
        } else {
            match GitService::get_branch_sha(".", &data.base_branch) {
                Ok(base) => base,
                Err(error) => {
                    log::warn!("Failed to get Git SHAs: {error}");
                    None
                }
            }
        };
        let target_sha = if data.target_sha.is_some() {
            data.target_sha
        } else {
            match GitService::get_branch_sha(".", &data.target_branch) {
                Ok(target) => target,
                Err(error) => {
                    log::warn!("Failed to get Git SHAs: {error}");
                    None
                }
            }
        };

        let review = Review::builder()
            .base_branch(data.base_branch.trim().to_string())
            .target_branch(data.target_branch.trim().to_string())
            .base_sha(base_sha)
            .target_sha(target_sha)
            .build();
        review.save(database.pool()).await?;
        log::info!("Created review: {}", review.title());

        // Trigger reviews reload
        events.send(AppEvent::ReviewsLoad);

        Ok(review)
    }

    /// List all reviews
    pub async fn list_reviews(database: &Database) -> color_eyre::Result<Vec<Review>> {
        let reviews = Review::list_all(database.pool()).await.map_err(|error| {
            eprintln!("Failed to list reviews: {error}");
            error
        })?;
        Ok(reviews)
    }

    /// Delete a review by ID and trigger reviews reload
    pub async fn delete_review_by_id(
        database: &Database,
        review_id: &str,
        events: &mut EventHandler,
    ) -> color_eyre::Result<()> {
        match Review::find_by_id(database.pool(), review_id).await {
            Ok(Some(review)) => {
                log::debug!("Found review to delete with ID {}", review.id);
                review.delete(database.pool()).await?;
                log::info!("Deleted review with ID {}", review.id);
                events.send(AppEvent::ReviewsLoad);
                Ok(())
            }
            Ok(None) => {
                log::warn!("No review found with ID: {review_id}");
                Ok(()) // No error, just nothing to delete
            }
            Err(error) => {
                log::error!("Error finding review by ID: {error}");
                Err(error.into())
            }
        }
    }

    /// Send loading event to start the actual loading process
    fn handle_reviews_load(events: &mut EventHandler) {
        events.send(AppEvent::ReviewsLoading);
        events.send(AppEvent::ReviewsLoadingState(ReviewsLoadingState::Loading));
    }

    /// Actually load reviews from database
    async fn handle_reviews_loading(database: &Database, events: &mut EventHandler) {
        match Self::list_reviews(database).await {
            Ok(reviews) => {
                events.send(AppEvent::ReviewsLoadingState(ReviewsLoadingState::Loaded(
                    reviews.into(),
                )));
            }
            Err(error) => {
                events.send(AppEvent::ReviewsLoadingState(ReviewsLoadingState::Error(
                    error.to_string().into(),
                )));
            }
        }
    }

    /// Handle review creation submission
    async fn handle_review_create_submit(
        data: &ReviewCreateData,
        database: &Database,
        events: &mut EventHandler,
    ) {
        match Self::create_review(database, data.clone(), events).await {
            Ok(review) => {
                events.send(AppEvent::ReviewCreated(review));
            }
            Err(error) => {
                log::error!("Failed to create review: {error}");
                // For now, we'll still close the dialog even on error
                // In the future, we might want to show an error message
                events.send(AppEvent::ReviewCreatedError(error.to_string().into()));
            }
        }
    }

    /// Handle review deletion
    async fn handle_review_delete(review_id: &str, database: &Database, events: &mut EventHandler) {
        match Self::delete_review_by_id(database, review_id, events).await {
            Ok(()) => {
                events.send(AppEvent::ReviewDeleted);
            }
            Err(error) => {
                log::error!("Failed to delete review: {error}");
                events.send(AppEvent::ReviewDeletedError(error.to_string().into()));
            }
        }
    }

    /// Handle loading a single review by ID
    async fn handle_review_load(review_id: &str, database: &Database, events: &mut EventHandler) {
        events.send(AppEvent::ReviewLoadingState(ReviewLoadingState::Loading));

        match Review::find_by_id(database.pool(), review_id).await {
            Ok(Some(review)) => {
                log::debug!("Loaded review with ID {}", review.id);
                events.send(AppEvent::ReviewLoadingState(ReviewLoadingState::Loaded(
                    Arc::from(review),
                )));
            }
            Ok(None) => {
                log::warn!("No review found with ID: {review_id}");
                events.send(AppEvent::ReviewLoadingState(ReviewLoadingState::NotFound(
                    Arc::from(review_id.to_string()),
                )));
            }
            Err(error) => {
                log::error!("Error loading review by ID: {error}");
                events.send(AppEvent::ReviewLoadingState(ReviewLoadingState::Error(
                    error.to_string().into(),
                )));
            }
        }
    }

    async fn handle_review_refresh(
        review_id: &str,
        refresh_base: bool,
        refresh_target: bool,
        context: ServiceContext<'_>,
    ) {
        let review = match Review::find_by_id(context.database.pool(), review_id).await {
            Ok(Some(review)) => review,
            Ok(None) => {
                log::warn!("No review found with ID: {review_id}");
                return;
            }
            Err(error) => {
                log::error!("Error loading review by ID: {error}");
                return;
            }
        };

        let mut updated_review = review.clone();
        let mut did_update = false;

        if refresh_base {
            match GitService::get_branch_sha(context.repo_path, &review.base_branch) {
                Ok(Some(base_sha)) => {
                    updated_review.base_sha = Some(base_sha);
                    updated_review.base_sha_changed = None;
                    did_update = true;
                }
                Ok(None) => {
                    log::warn!("No base branch SHA found for review {}", review.id);
                }
                Err(error) => {
                    log::warn!(
                        "Failed to refresh base SHA for review {}: {error}",
                        review.id
                    );
                }
            }
        }

        if refresh_target {
            match GitService::get_branch_sha(context.repo_path, &review.target_branch) {
                Ok(Some(target_sha)) => {
                    updated_review.target_sha = Some(target_sha);
                    updated_review.target_sha_changed = None;
                    did_update = true;
                }
                Ok(None) => {
                    log::warn!("No target branch SHA found for review {}", review.id);
                }
                Err(error) => {
                    log::warn!(
                        "Failed to refresh target SHA for review {}: {error}",
                        review.id
                    );
                }
            }
        }

        if !did_update {
            return;
        }

        updated_review.updated_at = crate::time_provider::SystemTimeProvider.now();
        if let Err(error) = updated_review
            .update_shas(
                context.database.pool(),
                updated_review.base_sha.clone(),
                updated_review.target_sha.clone(),
                updated_review.base_sha_changed.clone(),
                updated_review.target_sha_changed.clone(),
            )
            .await
        {
            log::error!("Failed to refresh review SHAs for {}: {error}", review.id);
            return;
        }

        context.events.send(AppEvent::ReviewsLoad);
        context
            .events
            .send(AppEvent::ReviewLoad(Arc::from(review.id)));
    }
}

impl ServiceHandler for ReviewService {
    fn handle_app_event<'a>(
        event: &'a AppEvent,
        context: ServiceContext<'a>,
    ) -> Pin<Box<dyn Future<Output = color_eyre::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match event {
                AppEvent::ReviewsLoad => Self::handle_reviews_load(context.events),
                AppEvent::ReviewsLoading => {
                    Self::handle_reviews_loading(context.database, context.events).await
                }
                AppEvent::ReviewCreateSubmit(data) => {
                    Self::handle_review_create_submit(data, context.database, context.events).await
                }
                AppEvent::ReviewDelete(review_id) => {
                    Self::handle_review_delete(review_id, context.database, context.events).await
                }
                AppEvent::ReviewLoad(review_id) => {
                    Self::handle_review_load(review_id, context.database, context.events).await
                }
                AppEvent::ReviewRefreshBase { review_id } => {
                    Self::handle_review_refresh(review_id, true, false, context).await
                }
                AppEvent::ReviewRefreshTarget { review_id } => {
                    Self::handle_review_refresh(review_id, false, true, context).await
                }
                AppEvent::ReviewRefreshBoth { review_id } => {
                    Self::handle_review_refresh(review_id, true, true, context).await
                }
                _ => {
                    // Other events are not handled by ReviewService
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sqlx::SqlitePool;

    use crate::{
        app::App,
        event::{AppEvent, Event, EventHandler, ReviewId},
    };

    async fn create_test_database() -> Database {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        Database::from_pool(pool)
    }

    #[tokio::test]
    async fn test_create_review_with_valid_branches() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let data = ReviewCreateData {
            base_branch: "main".to_string(),
            target_branch: "feature/test".to_string(),
            base_sha: None,
            target_sha: None,
        };

        ReviewService::create_review(&database, data, &mut events)
            .await
            .unwrap();

        // Should have triggered ReviewsLoad event
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ReviewsLoad)));

        // Verify the review was actually created
        let reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].base_branch, "main");
        assert_eq!(reviews[0].target_branch, "feature/test");
    }

    #[tokio::test]
    async fn test_create_review_with_empty_base_branch() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let data = ReviewCreateData {
            base_branch: "".to_string(),
            target_branch: "feature/test".to_string(),
            base_sha: None,
            target_sha: None,
        };

        match ReviewService::create_review(&database, data, &mut events).await {
            Ok(_) => panic!("Expected error for empty base branch"),
            Err(e) => {
                assert_eq!(e.to_string(), "Base branch cannot be empty");
                // Verify no review was created
                let reviews = Review::list_all(database.pool()).await.unwrap();
                assert_eq!(reviews.len(), 0);
            }
        }
    }

    #[tokio::test]
    async fn test_create_review_with_empty_target_branch() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let data = ReviewCreateData {
            base_branch: "main".to_string(),
            target_branch: "".to_string(),
            base_sha: None,
            target_sha: None,
        };

        match ReviewService::create_review(&database, data, &mut events).await {
            Ok(_) => panic!("Expected error for empty target branch"),
            Err(e) => {
                assert_eq!(e.to_string(), "Target branch cannot be empty");
                // Verify no review was created
                let reviews = Review::list_all(database.pool()).await.unwrap();
                assert_eq!(reviews.len(), 0);
            }
        }
    }

    #[tokio::test]
    async fn test_create_review_trims_whitespace() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let data = ReviewCreateData {
            base_branch: "  main  ".to_string(),
            target_branch: "  feature/test  ".to_string(),
            base_sha: None,
            target_sha: None,
        };

        ReviewService::create_review(&database, data, &mut events)
            .await
            .unwrap();

        // Should have triggered ReviewsLoad event
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ReviewsLoad)));

        // Verify the review was created with trimmed branches
        let reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].base_branch, "main");
        assert_eq!(reviews[0].target_branch, "feature/test");
    }

    #[tokio::test]
    async fn test_list_reviews_empty() {
        let database = create_test_database().await;

        let reviews = ReviewService::list_reviews(&database).await.unwrap();

        assert_eq!(reviews.len(), 0);
    }

    #[tokio::test]
    async fn test_list_reviews_with_data() {
        let database = create_test_database().await;

        // Create some reviews
        let mut events = EventHandler::new_for_test();
        let data1 = ReviewCreateData {
            base_branch: "main".to_string(),
            target_branch: "feature/review-1".to_string(),
            base_sha: None,
            target_sha: None,
        };
        let data2 = ReviewCreateData {
            base_branch: "main".to_string(),
            target_branch: "feature/review-2".to_string(),
            base_sha: None,
            target_sha: None,
        };

        ReviewService::create_review(&database, data1, &mut events)
            .await
            .unwrap();
        ReviewService::create_review(&database, data2, &mut events)
            .await
            .unwrap();

        let reviews = ReviewService::list_reviews(&database).await.unwrap();

        assert_eq!(reviews.len(), 2);
        // Should be ordered by created_at DESC, so newest first
        assert_eq!(reviews[0].target_branch, "feature/review-2");
        assert_eq!(reviews[1].target_branch, "feature/review-1");
    }

    #[tokio::test]
    async fn test_delete_review_by_id() {
        let database = create_test_database().await;

        // Create some reviews
        let mut events = EventHandler::new_for_test();
        let data1 = ReviewCreateData {
            base_branch: "main".to_string(),
            target_branch: "feature/review-1".to_string(),
            base_sha: None,
            target_sha: None,
        };
        let data2 = ReviewCreateData {
            base_branch: "main".to_string(),
            target_branch: "feature/review-2".to_string(),
            base_sha: None,
            target_sha: None,
        };

        ReviewService::create_review(&database, data1, &mut events)
            .await
            .unwrap();
        ReviewService::create_review(&database, data2, &mut events)
            .await
            .unwrap();

        let reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 2);

        // Delete first review (which should be "Review 2" due to DESC ordering)
        let review_id_to_delete = reviews[0].id.clone();
        ReviewService::delete_review_by_id(&database, &review_id_to_delete, &mut events)
            .await
            .unwrap();

        // Should have triggered ReviewsLoad event
        let event = events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ReviewsLoad)));

        // Verify the review was deleted
        let updated_reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(updated_reviews.len(), 1);
        assert_eq!(updated_reviews[0].target_branch, "feature/review-1");
    }

    #[tokio::test]
    async fn test_delete_review_by_invalid_id() {
        let database = create_test_database().await;

        // Create one review
        let mut events = EventHandler::new_for_test();
        let data = ReviewCreateData {
            base_branch: "main".to_string(),
            target_branch: "feature/review-1".to_string(),
            base_sha: None,
            target_sha: None,
        };
        ReviewService::create_review(&database, data, &mut events)
            .await
            .unwrap();
        // Receive the event that was sent by create_review
        let _event = events.try_recv().unwrap();

        let reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 1);

        // Try to delete with non-existent ID
        ReviewService::delete_review_by_id(&database, "non-existent-id", &mut events)
            .await
            .unwrap();

        assert!(!events.has_pending_events());

        // Should still have 1 review since deletion didn't happen
        let updated_reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(updated_reviews.len(), 1);
        assert_eq!(updated_reviews[0].target_branch, "feature/review-1");
    }

    #[tokio::test]
    async fn test_handle_app_event_reviews_load() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        ReviewService::handle_app_event(
            &AppEvent::ReviewsLoad,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent a ReviewsLoading event
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ReviewsLoading)));
    }

    #[tokio::test]
    async fn test_handle_review_refresh_base_updates_sha() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let branches = GitService::get_branches(".").unwrap();
        assert!(!branches.is_empty());
        let branch = branches[0].clone();
        let current_sha = GitService::get_branch_sha(".", &branch).unwrap();

        let review = Review::builder()
            .base_branch(branch.clone())
            .target_branch(branch.clone())
            .base_sha(Some("old-base".to_string()))
            .target_sha(Some("old-target".to_string()))
            .base_sha_changed(Some("changed-base".to_string()))
            .target_sha_changed(Some("changed-target".to_string()))
            .build();
        review.save(database.pool()).await.unwrap();

        ReviewService::handle_app_event(
            &AppEvent::ReviewRefreshBase {
                review_id: Arc::from(review.id.clone()),
            },
            ServiceContext {
                database: &database,
                repo_path: ".",
                events: &mut events,
            },
        )
        .await
        .unwrap();

        let updated = Review::find_by_id(database.pool(), &review.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.base_sha, current_sha);
        assert!(updated.base_sha_changed.is_none());
        assert_eq!(updated.target_sha, Some("old-target".to_string()));
        assert_eq!(
            updated.target_sha_changed,
            Some("changed-target".to_string())
        );
    }

    #[tokio::test]
    async fn test_handle_review_refresh_both_updates_shas() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let branches = GitService::get_branches(".").unwrap();
        assert!(!branches.is_empty());
        let branch = branches[0].clone();
        let current_sha = GitService::get_branch_sha(".", &branch).unwrap();

        let review = Review::builder()
            .base_branch(branch.clone())
            .target_branch(branch.clone())
            .base_sha(Some("old-base".to_string()))
            .target_sha(Some("old-target".to_string()))
            .base_sha_changed(Some("changed-base".to_string()))
            .target_sha_changed(Some("changed-target".to_string()))
            .build();
        review.save(database.pool()).await.unwrap();

        ReviewService::handle_app_event(
            &AppEvent::ReviewRefreshBoth {
                review_id: Arc::from(review.id.clone()),
            },
            ServiceContext {
                database: &database,
                repo_path: ".",
                events: &mut events,
            },
        )
        .await
        .unwrap();

        let updated = Review::find_by_id(database.pool(), &review.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.base_sha, current_sha);
        assert_eq!(updated.target_sha, current_sha);
        assert!(updated.base_sha_changed.is_none());
        assert!(updated.target_sha_changed.is_none());
    }

    #[tokio::test]
    async fn test_handle_app_event_reviews_loading_with_data() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Create a test review
        let review = Review::builder().build();
        review.save(database.pool()).await.unwrap();

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        ReviewService::handle_app_event(
            &AppEvent::ReviewsLoading,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent a ReviewsLoadingState event with the review
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoadingState(ReviewsLoadingState::Loaded(ref reviews))) =
            *event
        {
            assert_eq!(reviews.len(), 1);
            assert_eq!(reviews[0].base_branch, "default");
        } else {
            panic!("Expected ReviewsLoadingState event with reviews");
        }
    }

    #[tokio::test]
    async fn test_handle_app_event_reviews_loading_empty() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        ReviewService::handle_app_event(
            &AppEvent::ReviewsLoading,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent a ReviewsLoadingState event with empty list
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoadingState(ReviewsLoadingState::Loaded(ref reviews))) =
            *event
        {
            assert_eq!(reviews.len(), 0);
        } else {
            panic!("Expected ReviewsLoadingState event with empty reviews");
        }
    }

    #[tokio::test]
    async fn test_handle_app_event_other_events() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        // Test that other events are ignored
        ReviewService::handle_app_event(
            &AppEvent::Quit,
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should not have sent any events
        assert!(!events.has_pending_events());
    }

    #[tokio::test]
    async fn test_handle_app_event_review_create_submit() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        let data = ReviewCreateData {
            base_branch: "main".to_string(),
            target_branch: "feature/created".to_string(),
            base_sha: None,
            target_sha: None,
        };

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        ReviewService::handle_app_event(
            &AppEvent::ReviewCreateSubmit(data.into()),
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent ReviewsLoadingState and ViewClose events
        assert!(events.has_pending_events());

        // First event should be ReviewsLoad (triggered by create_review)
        let event1 = events.try_recv().unwrap();
        assert!(matches!(*event1, Event::App(AppEvent::ReviewsLoad)));

        // Second event should be ReviewCreated
        let event2 = events.try_recv().unwrap();
        assert!(matches!(*event2, Event::App(AppEvent::ReviewCreated(_))));

        // Verify the review was created
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].target_branch, "feature/created");
    }

    #[tokio::test]
    async fn test_handle_app_event_review_create_submit_empty_branches() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        let data = ReviewCreateData {
            base_branch: "".to_string(),
            target_branch: "feature/test".to_string(),
            base_sha: None,
            target_sha: None,
        };

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        // Test empty branches submission
        ReviewService::handle_app_event(
            &AppEvent::ReviewCreateSubmit(data.into()),
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent ViewClose event only
        assert!(events.has_pending_events());

        // First event should be ReviewCreatedError
        let event = events.try_recv().unwrap();
        assert!(matches!(
            *event,
            Event::App(AppEvent::ReviewCreatedError(_))
        ));

        // No more events should be pending
        assert!(!events.has_pending_events());

        // Verify no review was created
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_app_event_review_delete() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        // Create two reviews
        let review1 = Review::builder().base_branch("main").build();
        let review2 = Review::builder().base_branch("dev").build();
        review1.save(app.database.pool()).await.unwrap();
        review2.save(app.database.pool()).await.unwrap();

        // Load reviews to get IDs (they will be ordered by created_at DESC)
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        let review_id_to_delete: Arc<ReviewId> = reviews[0].id.clone().into();

        // Test review deletion
        ReviewService::handle_app_event(
            &AppEvent::ReviewDelete(review_id_to_delete.clone()),
            ServiceContext {
                database: &app.database,
                repo_path: &app.repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent ReviewsLoadingState and ViewClose events
        assert!(events.has_pending_events());

        // First event should be ReviewsLoad (triggered by delete_review_by_id)
        let event1 = events.try_recv().unwrap();
        assert!(matches!(*event1, Event::App(AppEvent::ReviewsLoad)));

        // Second event should be ReviewDeleted
        let event2 = events.try_recv().unwrap();
        assert!(matches!(*event2, Event::App(AppEvent::ReviewDeleted)));

        // No more events should be pending
        assert!(!events.has_pending_events());

        // Review should be deleted from database
        let review = Review::find_by_id(app.database.pool(), &review_id_to_delete)
            .await
            .unwrap();
        assert!(review.is_none());
    }

    #[tokio::test]
    async fn test_handle_app_event_review_delete_non_existing_id() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        // Create a review but try to delete with non-existent ID
        let review = Review::builder().build();
        review.save(app.database.pool()).await.unwrap();

        // Test deletion with non-existent ID
        ReviewService::handle_app_event(
            &AppEvent::ReviewDelete("non-existent-id".into()),
            ServiceContext {
                database: &app.database,
                repo_path: ".",
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent ReviewsLoadingState and ViewClose events
        assert!(events.has_pending_events());

        // Event should be ViewClose (to close the dialog)
        let event1 = events.try_recv().unwrap();
        assert!(matches!(*event1, Event::App(AppEvent::ReviewDeleted)));

        // No more events should be pending
        assert!(!events.has_pending_events());

        // Original review should still be there
        let reviews = Review::list_all(app.database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].base_branch, "default");
    }

    #[tokio::test]
    async fn test_handle_app_event_reviews_loading_database_error() {
        // Create a database without the reviews table to simulate error
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // Note: We deliberately don't create the table to cause an error
        let database = Database::from_pool(pool);

        let mut events = EventHandler::new_for_test();
        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        ReviewService::handle_app_event(
            &AppEvent::ReviewsLoading,
            ServiceContext {
                database: &app.database,
                repo_path: ".",
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should have sent a ReviewsLoadingError event due to missing table
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        assert!(matches!(
            *event,
            Event::App(AppEvent::ReviewsLoadingState(ReviewsLoadingState::Error(_)))
        ));
    }

    #[tokio::test]
    async fn test_handle_app_event_review_load_existing_review() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        let app = App {
            running: true,
            events: EventHandler::new_for_test(),
            database,
            view_stack: vec![],
            repo_path: ".".to_string(),
        };

        // Create a test review
        let review = Review::builder().build();
        review.save(app.database.pool()).await.unwrap();

        // Test loading the review
        ReviewService::handle_app_event(
            &AppEvent::ReviewLoad(review.id.clone().into()),
            ServiceContext {
                database: &app.database,
                repo_path: ".",
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should send ReviewLoadingState events
        assert!(events.has_pending_events());

        // First event should be Loading
        let event1 = events.try_recv().unwrap();
        match &*event1 {
            Event::App(AppEvent::ReviewLoadingState(ReviewLoadingState::Loading)) => {}
            _ => panic!("Expected ReviewLoadingState::Loading event, got: {event1:?}"),
        }

        // Second event should be Loaded with the review
        let event2 = events.try_recv().unwrap();
        match &*event2 {
            Event::App(AppEvent::ReviewLoadingState(ReviewLoadingState::Loaded(loaded_review))) => {
                assert_eq!(loaded_review.id, review.id);
                assert_eq!(loaded_review.base_branch, "default");
            }
            _ => panic!("Expected ReviewLoadingState::Loaded event, got: {event2:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_app_event_review_load_non_existent_review() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Test loading a non-existent review
        let non_existent_id = "non-existent-id";
        ReviewService::handle_app_event(
            &AppEvent::ReviewLoad(non_existent_id.into()),
            ServiceContext {
                database: &database,
                repo_path: ".",
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should send ReviewLoadingState events
        assert!(events.has_pending_events());

        // First event should be Loading
        let event1 = events.try_recv().unwrap();
        match &*event1 {
            Event::App(AppEvent::ReviewLoadingState(ReviewLoadingState::Loading)) => {}
            _ => panic!("Expected ReviewLoadingState::Loading event, got: {event1:?}"),
        }

        // Second event should be NotFound
        let event2 = events.try_recv().unwrap();
        match &*event2 {
            Event::App(AppEvent::ReviewLoadingState(ReviewLoadingState::NotFound(review_id))) => {
                assert_eq!(review_id.as_ref(), non_existent_id);
            }
            _ => panic!("Expected ReviewLoadingState::NotFound event, got: {event2:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_app_event_review_load_database_error() {
        // Create a database without the reviews table to simulate an error
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let database = Database::from_pool(pool);
        let mut events = EventHandler::new_for_test();
        let repo_path = ".".to_string();

        // Test loading a review when the table doesn't exist (will cause a database error)
        ReviewService::handle_app_event(
            &AppEvent::ReviewLoad("some-id".into()),
            ServiceContext {
                database: &database,
                repo_path: &repo_path,
                events: &mut events,
            },
        )
        .await
        .unwrap();

        // Should send ReviewLoadingState events
        assert!(events.has_pending_events());

        // First event should be Loading
        let event1 = events.try_recv().unwrap();
        match &*event1 {
            Event::App(AppEvent::ReviewLoadingState(ReviewLoadingState::Loading)) => {}
            _ => panic!("Expected ReviewLoadingState::Loading event, got: {event1:?}"),
        }

        // Second event should be Error
        let event2 = events.try_recv().unwrap();
        match &*event2 {
            Event::App(AppEvent::ReviewLoadingState(ReviewLoadingState::Error(error))) => {
                assert!(error.contains("no such table: reviews"));
            }
            _ => panic!("Expected ReviewLoadingState::Error event, got: {event2:?}"),
        }
    }
}
