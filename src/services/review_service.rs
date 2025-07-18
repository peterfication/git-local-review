use super::ServiceHandler;
use crate::{
    database::Database,
    event::{AppEvent, EventHandler},
    models::review::Review,
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
    Loaded,
    /// Error occurred during loading
    Error(String),
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
    /// Create a new review and return the updated reviews list
    pub async fn create_review(
        database: &Database,
        data: ReviewCreateData,
    ) -> color_eyre::Result<Vec<Review>> {
        if !data.title.trim().is_empty() {
            let review = Review::new(data.title.trim().to_string());
            review.save(database.pool()).await?;
            log::info!("Created review: {}", review.title);
        }

        // Return updated reviews list
        let reviews = Review::list_all(database.pool()).await.unwrap_or_default();

        Ok(reviews)
    }

    /// List all reviews
    pub async fn list_reviews(database: &Database) -> color_eyre::Result<Vec<Review>> {
        let reviews = Review::list_all(database.pool()).await?;
        Ok(reviews)
    }

    /// Delete a review by ID and return the updated reviews list
    pub async fn delete_review_by_id(
        database: &Database,
        review_id: &str,
    ) -> color_eyre::Result<Vec<Review>> {
        // Find the review by ID
        let reviews = Review::list_all(database.pool()).await.unwrap_or_default();
        if let Some(review_to_delete) = reviews.iter().find(|r| r.id == review_id) {
            review_to_delete.delete(database.pool()).await?;
            log::info!("Deleted review: {}", review_to_delete.title);
        }

        // Return updated reviews list
        let reviews = Review::list_all(database.pool()).await.unwrap_or_default();
        Ok(reviews)
    }

    /// Send loading event to start the actual loading process
    fn handle_reviews_load(events: &mut EventHandler) {
        events.send(AppEvent::ReviewsLoading);
    }

    /// Actually load reviews from database
    async fn handle_reviews_loaded(database: &Database, events: &mut EventHandler) {
        match Self::list_reviews(database).await {
            Ok(reviews) => {
                events.send(AppEvent::ReviewsLoaded(reviews));
            }
            Err(e) => {
                events.send(AppEvent::ReviewsLoadingError(e.to_string()));
            }
        }
    }

    /// Handle review creation submission
    async fn handle_review_create_submit(
        data: &ReviewCreateData,
        database: &Database,
        events: &mut EventHandler,
    ) {
        match Self::create_review(database, data.clone()).await {
            Ok(reviews) => {
                events.send(AppEvent::ReviewsLoaded(reviews));
                events.send(AppEvent::ViewClose);
            }
            Err(e) => {
                log::error!("Failed to create review: {e}");
                // For now, we'll still close the dialog even on error
                // In the future, we might want to show an error message
                events.send(AppEvent::ViewClose);
            }
        }
    }

    /// Handle review deletion
    async fn handle_review_delete(review_id: &str, database: &Database, events: &mut EventHandler) {
        match Self::delete_review_by_id(database, review_id).await {
            Ok(reviews) => {
                events.send(AppEvent::ReviewsLoaded(reviews));
                // Close the confirmation dialog by popping the view
                events.send(AppEvent::ViewClose);
            }
            Err(e) => {
                log::error!("Failed to delete review: {e}");
                // Even on error, we should close the dialog
                events.send(AppEvent::ViewClose);
            }
        }
    }
}

impl ServiceHandler for ReviewService {
    async fn handle_app_event(
        event: &AppEvent,
        database: &Database,
        events: &mut EventHandler,
    ) -> color_eyre::Result<()> {
        match event {
            AppEvent::ReviewsLoad => Self::handle_reviews_load(events),
            AppEvent::ReviewsLoading => Self::handle_reviews_loaded(database, events).await,
            AppEvent::ReviewCreateSubmit(data) => {
                Self::handle_review_create_submit(data, database, events).await
            }
            AppEvent::ReviewDelete(review_id) => {
                Self::handle_review_delete(review_id, database, events).await
            }
            _ => {
                // Other events are not handled by ReviewService
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventHandler};
    use sqlx::SqlitePool;

    async fn create_test_database() -> Database {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();
        Database::from_pool(pool)
    }

    #[tokio::test]
    async fn test_create_review_with_valid_title() {
        let database = create_test_database().await;
        let data = ReviewCreateData {
            title: "Test Review".to_string(),
        };

        let reviews = ReviewService::create_review(&database, data).await.unwrap();

        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].title, "Test Review");
    }

    #[tokio::test]
    async fn test_create_review_with_empty_title() {
        let database = create_test_database().await;
        let data = ReviewCreateData {
            title: "".to_string(),
        };

        let reviews = ReviewService::create_review(&database, data).await.unwrap();

        assert_eq!(reviews.len(), 0);
    }

    #[tokio::test]
    async fn test_create_review_with_whitespace_title() {
        let database = create_test_database().await;
        let data = ReviewCreateData {
            title: "   ".to_string(),
        };

        let reviews = ReviewService::create_review(&database, data).await.unwrap();

        assert_eq!(reviews.len(), 0);
    }

    #[tokio::test]
    async fn test_create_review_trims_whitespace() {
        let database = create_test_database().await;
        let data = ReviewCreateData {
            title: "  Test Review  ".to_string(),
        };

        let reviews = ReviewService::create_review(&database, data).await.unwrap();

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
        let data1 = ReviewCreateData {
            title: "Review 1".to_string(),
        };
        let data2 = ReviewCreateData {
            title: "Review 2".to_string(),
        };

        ReviewService::create_review(&database, data1)
            .await
            .unwrap();
        ReviewService::create_review(&database, data2)
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
        let data1 = ReviewCreateData {
            title: "Review 1".to_string(),
        };
        let data2 = ReviewCreateData {
            title: "Review 2".to_string(),
        };

        ReviewService::create_review(&database, data1)
            .await
            .unwrap();
        let reviews = ReviewService::create_review(&database, data2)
            .await
            .unwrap();

        assert_eq!(reviews.len(), 2);

        // Delete first review (which should be "Review 2" due to DESC ordering)
        let review_id_to_delete = reviews[0].id.clone();
        let updated_reviews = ReviewService::delete_review_by_id(&database, &review_id_to_delete)
            .await
            .unwrap();

        assert_eq!(updated_reviews.len(), 1);
        assert_eq!(updated_reviews[0].title, "Review 1");
    }

    #[tokio::test]
    async fn test_delete_review_by_invalid_id() {
        let database = create_test_database().await;

        // Create one review
        let data = ReviewCreateData {
            title: "Review 1".to_string(),
        };
        let reviews = ReviewService::create_review(&database, data).await.unwrap();

        assert_eq!(reviews.len(), 1);

        // Try to delete with non-existent ID
        let updated_reviews = ReviewService::delete_review_by_id(&database, "non-existent-id")
            .await
            .unwrap();

        // Should still have 1 review since deletion didn't happen
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
        assert!(matches!(event, Event::App(AppEvent::ReviewsLoading)));
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

        // Should have sent a ReviewsLoaded event with the review
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoaded(reviews)) = event {
            assert_eq!(reviews.len(), 1);
            assert_eq!(reviews[0].title, "Test Review");
        } else {
            panic!("Expected ReviewsLoaded event with reviews");
        }
    }

    #[tokio::test]
    async fn test_handle_app_event_reviews_loading_empty() {
        let database = create_test_database().await;
        let mut events = EventHandler::new_for_test();

        ReviewService::handle_app_event(&AppEvent::ReviewsLoading, &database, &mut events)
            .await
            .unwrap();

        // Should have sent a ReviewsLoaded event with empty list
        assert!(events.has_pending_events());
        let event = events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoaded(reviews)) = event {
            assert_eq!(reviews.len(), 0);
        } else {
            panic!("Expected ReviewsLoaded event with empty reviews");
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
            &AppEvent::ReviewCreateSubmit(data),
            &database,
            &mut events,
        )
        .await
        .unwrap();

        // Should have sent ReviewsLoaded and ViewClose events
        assert!(events.has_pending_events());

        // First event should be ReviewsLoaded with the new review
        let event1 = events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoaded(reviews)) = event1 {
            assert_eq!(reviews.len(), 1);
            assert_eq!(reviews[0].title, "Created Review");
        } else {
            panic!("Expected ReviewsLoaded event, got: {event1:?}");
        }

        // Second event should be ViewClose
        let event2 = events.try_recv().unwrap();
        assert!(matches!(event2, Event::App(AppEvent::ViewClose)));
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
            &AppEvent::ReviewCreateSubmit(data),
            &database,
            &mut events,
        )
        .await
        .unwrap();

        // Should have sent ReviewsLoaded and ViewClose events
        assert!(events.has_pending_events());

        // First event should be ReviewsLoaded with empty list (no review created)
        let event1 = events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoaded(reviews)) = event1 {
            assert_eq!(reviews.len(), 0);
        } else {
            panic!("Expected ReviewsLoaded event, got: {event1:?}");
        }

        // Second event should be ViewClose
        let event2 = events.try_recv().unwrap();
        assert!(matches!(event2, Event::App(AppEvent::ViewClose)));

        // No more events should be pending
        assert!(!events.has_pending_events());
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
        let review_id_to_delete = reviews[0].id.clone();

        // Test review deletion
        ReviewService::handle_app_event(
            &AppEvent::ReviewDelete(review_id_to_delete.clone()),
            &database,
            &mut events,
        )
        .await
        .unwrap();

        // Should have sent ReviewsLoaded and ViewClose events
        assert!(events.has_pending_events());

        // First event should be ReviewsLoaded with one less review
        let event1 = events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoaded(reviews)) = event1 {
            assert_eq!(reviews.len(), 1);
            assert_eq!(reviews[0].title, "Review 1");
        } else {
            panic!("Expected ReviewsLoaded event, got: {event1:?}");
        }

        // Second event should be ViewClose (to close the dialog)
        let event2 = events.try_recv().unwrap();
        assert!(matches!(event2, Event::App(AppEvent::ViewClose)));

        // No more events should be pending
        assert!(!events.has_pending_events());

        // Review should be deleted from database
        let reviews = Review::list_all(database.pool()).await.unwrap();
        assert!(!reviews.iter().any(|r| r.id == review_id_to_delete));
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
            &AppEvent::ReviewDelete("non-existent-id".to_string()),
            &database,
            &mut events,
        )
        .await
        .unwrap();

        // Should have sent ReviewsLoaded and ViewClose events
        assert!(events.has_pending_events());

        // First event should be ReviewsLoaded with original review still there
        let event1 = events.try_recv().unwrap();
        if let Event::App(AppEvent::ReviewsLoaded(reviews)) = event1 {
            assert_eq!(reviews.len(), 1);
            assert_eq!(reviews[0].title, "Test Review");
        } else {
            panic!("Expected ReviewsLoaded event, got: {event1:?}");
        }

        // Second event should be ViewClose (to close the dialog)
        let event2 = events.try_recv().unwrap();
        assert!(matches!(event2, Event::App(AppEvent::ViewClose)));

        // No more events should be pending
        assert!(!events.has_pending_events());
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
            event,
            Event::App(AppEvent::ReviewsLoadingError(_))
        ));
    }
}
