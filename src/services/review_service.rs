use std::sync::Arc;

use super::ServiceHandler;

use crate::{
    database::Database,
    event::{AppEvent, EventHandler},
    models::Review,
};

#[derive(Clone, Debug)]
pub struct ReviewCreateData {
    pub title: String,
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
        if data.title.trim().is_empty() {
            return Err(color_eyre::eyre::eyre!("Review title cannot be empty"));
        }

        let review = Review::new(data.title.trim().to_string());
        review.save(database.pool()).await?;
        log::info!("Created review: {}", review.title);

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
        match Review::find_by_id(database.pool(), review_id).await {
            Ok(Some(review)) => {
                log::debug!("Loaded review with ID {}", review.id);
                events.send(AppEvent::ReviewLoaded(Arc::from(review)));
            }
            Ok(None) => {
                log::warn!("No review found with ID: {review_id}");
                events.send(AppEvent::ReviewNotFound(review_id.into()));
            }
            Err(error) => {
                log::error!("Error loading review by ID: {error}");
                events.send(AppEvent::ReviewLoadError(error.to_string().into()));
            }
        }
    }
}

impl ServiceHandler for ReviewService {
    fn handle_app_event<'a>(
        event: &'a AppEvent,
        database: &'a Database,
        events: &'a mut EventHandler,
    ) -> std::pin::Pin<Box<dyn Future<Output = color_eyre::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            match event {
                AppEvent::ReviewsLoad => Self::handle_reviews_load(events),
                AppEvent::ReviewsLoading => Self::handle_reviews_loading(database, events).await,
                AppEvent::ReviewCreateSubmit(data) => {
                    Self::handle_review_create_submit(data, database, events).await
                }
                AppEvent::ReviewDelete(review_id) => {
                    Self::handle_review_delete(review_id, database, events).await
                }
                AppEvent::ReviewLoad(review_id) => {
                    Self::handle_review_load(review_id, database, events).await
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
    use crate::event::{Event, EventHandler, ReviewId};
    use sqlx::SqlitePool;

    async fn create_test_database() -> Database {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();
        Database::from_pool(pool)
    }

    #[tokio::test]
    async fn test_create_review_with_valid_title() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let data = ReviewCreateData {
            title: "Test Review".to_string(),
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
        assert_eq!(reviews[0].title, "Test Review");
    }

    #[tokio::test]
    async fn test_create_review_with_empty_title() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let data = ReviewCreateData {
            title: "".to_string(),
        };

        match ReviewService::create_review(&database, data, &mut events).await {
            Ok(_) => panic!("Expected error for empty title"),
            Err(e) => {
                assert_eq!(e.to_string(), "Review title cannot be empty");
                // Verify no review was created
                let reviews = Review::list_all(database.pool()).await.unwrap();
                assert_eq!(reviews.len(), 0);
            }
        }
    }

    #[tokio::test]
    async fn test_create_review_with_whitespace_title() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();
        let data = ReviewCreateData {
            title: "   ".to_string(),
        };

        match ReviewService::create_review(&database, data, &mut events).await {
            Ok(_) => panic!("Expected error for empty title"),
            Err(e) => {
                assert_eq!(e.to_string(), "Review title cannot be empty");
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
            title: "  Test Review  ".to_string(),
        };

        ReviewService::create_review(&database, data, &mut events)
            .await
            .unwrap();

        // Should have triggered ReviewsLoad event
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ReviewsLoad)));

        // Verify the review was created with trimmed title
        let reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].title, "Test Review");
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
            title: "Review 1".to_string(),
        };
        let data2 = ReviewCreateData {
            title: "Review 2".to_string(),
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
        assert_eq!(reviews[0].title, "Review 2");
        assert_eq!(reviews[1].title, "Review 1");
    }

    #[tokio::test]
    async fn test_delete_review_by_id() {
        let database = create_test_database().await;

        // Create some reviews
        let mut events = EventHandler::new_for_test();
        let data1 = ReviewCreateData {
            title: "Review 1".to_string(),
        };
        let data2 = ReviewCreateData {
            title: "Review 2".to_string(),
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
        assert_eq!(updated_reviews[0].title, "Review 1");
    }

    #[tokio::test]
    async fn test_delete_review_by_invalid_id() {
        let database = create_test_database().await;

        // Create one review
        let mut events = EventHandler::new_for_test();
        let data = ReviewCreateData {
            title: "Review 1".to_string(),
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
        assert_eq!(updated_reviews[0].title, "Review 1");
    }

    #[tokio::test]
    async fn test_handle_app_event_reviews_load() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        ReviewService::handle_app_event(&AppEvent::ReviewsLoad, &database, &mut events)
            .await
            .unwrap();

        // Should have sent a ReviewsLoading event
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        assert!(matches!(*event, Event::App(AppEvent::ReviewsLoading)));
    }

    #[tokio::test]
    async fn test_handle_app_event_reviews_loading_with_data() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Create a test review
        let review = Review::new("Test Review".to_string());
        review.save(database.pool()).await.unwrap();

        ReviewService::handle_app_event(&AppEvent::ReviewsLoading, &database, &mut events)
            .await
            .unwrap();

        // Should have sent a ReviewsLoadingState event with the review
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoadingState(ReviewsLoadingState::Loaded(ref reviews))) =
            *event
        {
            assert_eq!(reviews.len(), 1);
            assert_eq!(reviews[0].title, "Test Review");
        } else {
            panic!("Expected ReviewsLoadingState event with reviews");
        }
    }

    #[tokio::test]
    async fn test_handle_app_event_reviews_loading_empty() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        ReviewService::handle_app_event(&AppEvent::ReviewsLoading, &database, &mut events)
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

        // Test that other events are ignored
        ReviewService::handle_app_event(&AppEvent::Quit, &database, &mut events)
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
            title: "Created Review".to_string(),
        };

        ReviewService::handle_app_event(
            &AppEvent::ReviewCreateSubmit(data.into()),
            &database,
            &mut events,
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
        let reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].title, "Created Review");
    }

    #[tokio::test]
    async fn test_handle_app_event_review_create_submit_empty_title() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        let data = ReviewCreateData {
            title: "".to_string(),
        };

        // Test empty title submission
        ReviewService::handle_app_event(
            &AppEvent::ReviewCreateSubmit(data.into()),
            &database,
            &mut events,
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
        let reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_app_event_review_delete() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Create two reviews
        let review1 = Review::new("Review 1".to_string());
        let review2 = Review::new("Review 2".to_string());
        review1.save(database.pool()).await.unwrap();
        review2.save(database.pool()).await.unwrap();

        // Load reviews to get IDs (they will be ordered by created_at DESC)
        let reviews = Review::list_all(database.pool()).await.unwrap();
        let review_id_to_delete: Arc<ReviewId> = reviews[0].id.clone().into();

        // Test review deletion
        ReviewService::handle_app_event(
            &AppEvent::ReviewDelete(review_id_to_delete.clone()),
            &database,
            &mut events,
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
        let review = Review::find_by_id(database.pool(), &review_id_to_delete)
            .await
            .unwrap();
        assert!(review.is_none());
    }

    #[tokio::test]
    async fn test_handle_app_event_review_delete_non_existing_id() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        // Create a review but try to delete with non-existent ID
        let review = Review::new("Test Review".to_string());
        review.save(database.pool()).await.unwrap();

        // Test deletion with non-existent ID
        ReviewService::handle_app_event(
            &AppEvent::ReviewDelete("non-existent-id".into()),
            &database,
            &mut events,
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
        let reviews = Review::list_all(database.pool()).await.unwrap();
        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].title, "Test Review");
    }

    #[tokio::test]
    async fn test_handle_app_event_reviews_loading_database_error() {
        // Create a database without the reviews table to simulate error
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // Note: We deliberately don't create the table to cause an error
        let database = Database::from_pool(pool);

        let mut events = EventHandler::new_for_test();

        ReviewService::handle_app_event(&AppEvent::ReviewsLoading, &database, &mut events)
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

        // Create a test review
        let review = Review::new("Test Review".to_string());
        review.save(database.pool()).await.unwrap();

        // Test loading the review
        ReviewService::handle_app_event(
            &AppEvent::ReviewLoad(review.id.clone().into()),
            &database,
            &mut events,
        )
        .await
        .unwrap();

        // Should send ReviewLoaded event
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::ReviewLoaded(loaded_review)) => {
                assert_eq!(loaded_review.id, review.id);
                assert_eq!(loaded_review.title, "Test Review");
            }
            _ => panic!("Expected ReviewLoaded event"),
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
            &database,
            &mut events,
        )
        .await
        .unwrap();

        // Should send ReviewNotFound event
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::ReviewNotFound(review_id)) => {
                assert_eq!(review_id.as_ref(), non_existent_id);
            }
            _ => panic!("Expected ReviewNotFound event"),
        }
    }

    #[tokio::test]
    async fn test_handle_app_event_review_load_database_error() {
        // Create a database without the reviews table to simulate an error
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // Note: We intentionally don't call Review::create_table here
        let database = Database::from_pool(pool);
        let mut events = EventHandler::new_for_test();

        // Test loading a review when the table doesn't exist (will cause a database error)
        ReviewService::handle_app_event(
            &AppEvent::ReviewLoad("some-id".into()),
            &database,
            &mut events,
        )
        .await
        .unwrap();

        // Should send ReviewLoadError event
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        match &*event {
            Event::App(AppEvent::ReviewLoadError(error)) => {
                assert!(error.contains("no such table: reviews"));
            }
            _ => panic!("Expected ReviewLoadError event"),
        }
    }
}
