use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Review {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Review {
    pub fn new(title: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            created_at: now,
            updated_at: now,
        }
    }

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
}

#[cfg(test)]
mod tests {
    use super::*;
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

        let review1 = Review::new("First Review".to_string());
        let review2 = Review::new("Second Review".to_string());

        // Save in order
        review1.save(&pool).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // Ensure different timestamps
        review2.save(&pool).await.unwrap();

        let reviews = Review::list_all(&pool).await.unwrap();

        assert_eq!(reviews.len(), 2);
        // Should be ordered by created_at DESC, so newest first
        assert_eq!(reviews[0].title, "Second Review");
        assert_eq!(reviews[1].title, "First Review");
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
}
