use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::time_provider::{SystemTimeProvider, TimeProvider};

#[derive(Debug, Clone, FromRow)]
pub struct Review {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PartialEq for Review {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Review {
    pub fn new(title: String) -> Self {
        Self::new_with_time_provider(title, &SystemTimeProvider)
    }

    pub fn new_with_time_provider(title: String, time_provider: &dyn TimeProvider) -> Self {
        let now = time_provider.now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            created_at: now,
            updated_at: now,
        }
    }

    #[cfg(test)]
    pub async fn create_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS reviews (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn save(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO reviews (id, title, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(&self.id)
        .bind(&self.title)
        .bind(self.created_at.to_rfc3339())
        .bind(self.updated_at.to_rfc3339())
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_all(pool: &SqlitePool) -> Result<Vec<Review>, sqlx::Error> {
        let reviews = sqlx::query_as::<_, Review>(
            r#"
            SELECT id, title, created_at, updated_at
            FROM reviews
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(reviews)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Review>, sqlx::Error> {
        let review = sqlx::query_as::<_, Review>(
            r#"
            SELECT id, title, created_at, updated_at
            FROM reviews
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(review)
    }

    pub async fn delete(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM reviews
            WHERE id = ?1
            "#,
        )
        .bind(&self.id)
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::fixed_time;
    use crate::time_provider::MockTimeProvider;
    use sqlx::SqlitePool;

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        Review::create_table(&pool).await.unwrap();
        pool
    }

    #[test]
    fn test_review_new() {
        let title = "Test Review".to_string();
        let review = Review::new(title.clone());

        assert_eq!(review.title, title);
        assert!(!review.id.is_empty());
        assert_eq!(review.created_at, review.updated_at);

        // ID should be a valid UUID
        assert!(uuid::Uuid::parse_str(&review.id).is_ok());
    }

    #[test]
    fn test_review_new_with_mock_time() {
        let title = "Test Review".to_string();
        let fixed_time = fixed_time();
        let time_provider = MockTimeProvider::new(fixed_time);

        let review = Review::new_with_time_provider(title.clone(), &time_provider);

        assert_eq!(review.title, title);
        assert!(!review.id.is_empty());
        assert_eq!(review.created_at, fixed_time);
        assert_eq!(review.updated_at, fixed_time);
        assert_eq!(review.created_at, review.updated_at);

        // ID should be a valid UUID
        assert!(uuid::Uuid::parse_str(&review.id).is_ok());
    }

    #[test]
    fn test_review_new_generates_unique_ids() {
        let review1 = Review::new("Review 1".to_string());
        let review2 = Review::new("Review 2".to_string());

        assert_ne!(review1.id, review2.id);
    }

    #[tokio::test]
    async fn test_review_save_and_list() {
        let pool = create_test_pool().await;
        let review = Review::new("Test Review".to_string());

        // Save the review
        review.save(&pool).await.unwrap();

        // List all reviews
        let reviews = Review::list_all(&pool).await.unwrap();

        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].id, review.id);
        assert_eq!(reviews[0].title, review.title);
    }

    #[tokio::test]
    async fn test_review_list_empty() {
        let pool = create_test_pool().await;

        let reviews = Review::list_all(&pool).await.unwrap();

        assert_eq!(reviews.len(), 0);
    }

    #[tokio::test]
    async fn test_review_list_ordered_by_created_at_desc() {
        let pool = create_test_pool().await;

        let time1 = fixed_time();
        let time2 = time1 + chrono::Duration::hours(1);

        let time_provider1 = MockTimeProvider::new(time1);
        let time_provider2 = MockTimeProvider::new(time2);

        let review1 = Review::new_with_time_provider("First Review".to_string(), &time_provider1);
        let review2 = Review::new_with_time_provider("Second Review".to_string(), &time_provider2);

        // Save in order
        review1.save(&pool).await.unwrap();
        review2.save(&pool).await.unwrap();

        let reviews = Review::list_all(&pool).await.unwrap();

        assert_eq!(reviews.len(), 2);
        // Should be ordered by created_at DESC, so newest first
        assert_eq!(reviews[0].title, "Second Review");
        assert_eq!(reviews[1].title, "First Review");
        assert!(reviews[0].created_at > reviews[1].created_at);
    }

    #[tokio::test]
    async fn test_review_save_duplicate_id_fails() {
        let pool = create_test_pool().await;
        let review1 = Review::new("Review 1".to_string());
        let mut review2 = Review::new("Review 2".to_string());

        // Make them have the same ID
        review2.id = review1.id.clone();

        // First save should succeed
        review1.save(&pool).await.unwrap();

        // Second save with same ID should fail
        assert!(review2.save(&pool).await.is_err());
    }

    #[tokio::test]
    async fn test_review_find_by_id() {
        let pool = create_test_pool().await;
        let review = Review::new("Test Review".to_string());

        // Save the review
        review.save(&pool).await.unwrap();

        // Find by ID
        let found_review = Review::find_by_id(&pool, &review.id).await.unwrap();
        assert!(found_review.is_some());
        let found_review = found_review.unwrap();
        assert_eq!(found_review.id, review.id);
        assert_eq!(found_review.title, review.title);
        assert_eq!(found_review.created_at, review.created_at);
        assert_eq!(found_review.updated_at, review.updated_at);

        // Find by non-existent ID
        let not_found = Review::find_by_id(&pool, "non-existent-id").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_review_delete() {
        let pool = create_test_pool().await;
        let review = Review::new("Test Review".to_string());

        // Save the review
        review.save(&pool).await.unwrap();

        // Verify it exists
        let reviews = Review::list_all(&pool).await.unwrap();
        assert_eq!(reviews.len(), 1);

        // Delete the review
        review.delete(&pool).await.unwrap();

        // Verify it's gone
        let reviews = Review::list_all(&pool).await.unwrap();
        assert_eq!(reviews.len(), 0);
    }

    #[test]
    fn test_review_eq_same_id() {
        let time1 = fixed_time();
        let time2 = time1 + chrono::Duration::hours(1);

        let time_provider1 = MockTimeProvider::new(time1);
        let time_provider2 = MockTimeProvider::new(time2);

        let review1 = Review::new_with_time_provider("Title 1".to_string(), &time_provider1);
        let mut review2 = Review::new_with_time_provider("Title 2".to_string(), &time_provider2);

        // Make review2 have the same ID as review1
        review2.id = review1.id.clone();

        // Should be equal despite different titles and timestamps
        assert_eq!(review1, review2);
    }

    #[test]
    fn test_review_eq_different_id() {
        let fixed_time = fixed_time();
        let time_provider = MockTimeProvider::new(fixed_time);

        let review1 = Review::new_with_time_provider("Same Title".to_string(), &time_provider);
        let review2 = Review::new_with_time_provider("Same Title".to_string(), &time_provider);

        // Should not be equal despite same title and timestamps because IDs are different
        assert_ne!(review1, review2);
    }

    #[test]
    fn test_review_eq_self() {
        let review = Review::new("Test Review".to_string());

        // Should be equal to itself
        assert_eq!(review, review);
    }

    #[test]
    fn test_review_eq_clone() {
        let review1 = Review::new("Test Review".to_string());
        let review2 = review1.clone();

        // Clone should be equal to original
        assert_eq!(review1, review2);
    }

    #[test]
    fn test_review_eq_ignores_other_fields() {
        let time1 = fixed_time();
        let time2 = time1 + chrono::Duration::days(30);

        let time_provider1 = MockTimeProvider::new(time1);

        let review1 = Review::new_with_time_provider("Original Title".to_string(), &time_provider1);
        let review2 = Review {
            id: review1.id.clone(),                          // Same ID
            title: "Completely Different Title".to_string(), // Different title
            created_at: time2,                               // Different created_at
            updated_at: time2,                               // Different updated_at
        };

        // Should be equal because only ID matters for equality
        assert_eq!(review1, review2);
    }
}
