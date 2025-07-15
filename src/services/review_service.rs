use crate::{database::Database, models::review::Review};

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

pub struct ReviewService;

impl ReviewService {
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
        let reviews = Review::list_all(database.pool()).await.unwrap_or_default();
        Ok(reviews)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
