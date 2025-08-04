use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};

use crate::{
    models::ReviewId,
    time_provider::{SystemTimeProvider, TimeProvider},
};

#[derive(Debug, Clone, FromRow)]
pub struct FileView {
    pub id: i64,
    pub review_id: ReviewId,
    pub file_path: String,
    pub created_at: DateTime<Utc>,
}

impl FileView {
    pub fn new(review_id: ReviewId, file_path: String) -> Self {
        Self::new_with_time_provider(review_id, file_path, &SystemTimeProvider)
    }

    pub fn new_with_time_provider(
        review_id: ReviewId,
        file_path: String,
        time_provider: &dyn TimeProvider,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            review_id,
            file_path,
            created_at: time_provider.now(),
        }
    }

    /// Mark a file as viewed for a review
    pub async fn mark_as_viewed(
        pool: &SqlitePool,
        review_id: &str,
        file_path: &str,
    ) -> Result<(), sqlx::Error> {
        let created_at = Utc::now().to_rfc3339();
        sqlx::query!(
            r#"
            INSERT OR IGNORE INTO file_views (review_id, file_path, created_at)
            VALUES (?1, ?2, ?3)
            "#,
            review_id,
            file_path,
            created_at
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark a file as unviewed for a review
    pub async fn mark_as_unviewed(
        pool: &SqlitePool,
        review_id: &str,
        file_path: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM file_views
            WHERE review_id = ?1 AND file_path = ?2
            "#,
            review_id,
            file_path
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get all viewed file paths for a review
    pub async fn get_viewed_files(
        pool: &SqlitePool,
        review_id: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        let file_paths = sqlx::query_scalar!(
            r#"
            SELECT file_path
            FROM file_views
            WHERE review_id = ?1
            ORDER BY created_at ASC
            "#,
            review_id
        )
        .fetch_all(pool)
        .await?;
        Ok(file_paths)
    }

    /// Check if a file is viewed for a review
    pub async fn is_file_viewed(
        pool: &SqlitePool,
        review_id: &str,
        file_path: &str,
    ) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM file_views
            WHERE review_id = ?1 AND file_path = ?2
            "#,
            review_id,
            file_path
        )
        .fetch_one(pool)
        .await?;
        Ok(count > 0)
    }

    /// Get all file views for a review
    pub async fn list_for_review(
        pool: &SqlitePool,
        review_id: &str,
    ) -> Result<Vec<FileView>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"
            SELECT id as "id!", review_id as "review_id!", file_path as "file_path!", created_at as "created_at!"
            FROM file_views
            WHERE review_id = ?1
            ORDER BY created_at ASC
            "#,
            review_id
        )
        .fetch_all(pool)
        .await?;

        let mut file_views = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                .with_timezone(&Utc);
            file_views.push(FileView {
                id: row.id,
                review_id: row.review_id,
                file_path: row.file_path,
                created_at,
            });
        }
        Ok(file_views)
    }

    /// Delete all file views for a review (used when review is deleted)
    pub async fn delete_for_review(pool: &SqlitePool, review_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM file_views
            WHERE review_id = ?1
            "#,
            review_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{models::Review, test_utils::fixed_time, time_provider::MockTimeProvider};
    use sqlx::SqlitePool;

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    async fn create_test_review(pool: &SqlitePool) -> Review {
        let review = Review::test_review(());
        review.save(pool).await.unwrap();
        review
    }

    #[test]
    fn test_file_view_new() {
        let review_id = "test-review-id".to_string();
        let file_path = "src/main.rs".to_string();
        let file_view = FileView::new(review_id.clone(), file_path.clone());

        assert_eq!(file_view.review_id, review_id);
        assert_eq!(file_view.file_path, file_path);
        assert_eq!(file_view.id, 0); // Should be 0 initially
    }

    #[test]
    fn test_file_view_new_with_time_provider() {
        let fixed_time = fixed_time();
        let time_provider = MockTimeProvider::new(fixed_time);
        let review_id = "test-review-id".to_string();
        let file_path = "src/main.rs".to_string();

        let file_view =
            FileView::new_with_time_provider(review_id.clone(), file_path.clone(), &time_provider);

        assert_eq!(file_view.review_id, review_id);
        assert_eq!(file_view.file_path, file_path);
        assert_eq!(file_view.created_at, fixed_time);
        assert_eq!(file_view.id, 0);
    }

    #[tokio::test]
    async fn test_mark_as_viewed() {
        let pool = create_test_pool().await;
        let review = create_test_review(&pool).await;
        let file_path = "src/main.rs";

        // Mark file as viewed
        FileView::mark_as_viewed(&pool, &review.id, file_path)
            .await
            .unwrap();

        // Check if file is viewed
        let is_viewed = FileView::is_file_viewed(&pool, &review.id, file_path)
            .await
            .unwrap();
        assert!(is_viewed);
    }

    #[tokio::test]
    async fn test_mark_as_viewed_duplicate() {
        let pool = create_test_pool().await;
        let review = create_test_review(&pool).await;
        let file_path = "src/main.rs";

        // Mark file as viewed twice - should not fail
        FileView::mark_as_viewed(&pool, &review.id, file_path)
            .await
            .unwrap();
        FileView::mark_as_viewed(&pool, &review.id, file_path)
            .await
            .unwrap();

        // Should still be viewed once
        let is_viewed = FileView::is_file_viewed(&pool, &review.id, file_path)
            .await
            .unwrap();
        assert!(is_viewed);

        let viewed_files = FileView::get_viewed_files(&pool, &review.id).await.unwrap();
        assert_eq!(viewed_files.len(), 1);
        assert_eq!(viewed_files[0], file_path);
    }

    #[tokio::test]
    async fn test_mark_as_unviewed() {
        let pool = create_test_pool().await;
        let review = create_test_review(&pool).await;
        let file_path = "src/main.rs";

        // Mark file as viewed first
        FileView::mark_as_viewed(&pool, &review.id, file_path)
            .await
            .unwrap();
        assert!(
            FileView::is_file_viewed(&pool, &review.id, file_path)
                .await
                .unwrap()
        );

        // Mark file as unviewed
        FileView::mark_as_unviewed(&pool, &review.id, file_path)
            .await
            .unwrap();

        // Check if file is no longer viewed
        let is_viewed = FileView::is_file_viewed(&pool, &review.id, file_path)
            .await
            .unwrap();
        assert!(!is_viewed);
    }

    #[tokio::test]
    async fn test_get_viewed_files() {
        let pool = create_test_pool().await;
        let review = create_test_review(&pool).await;

        let file_paths = vec!["src/main.rs", "src/lib.rs", "tests/test.rs"];

        // Mark files as viewed
        for file_path in &file_paths {
            FileView::mark_as_viewed(&pool, &review.id, file_path)
                .await
                .unwrap();
        }

        let viewed_files = FileView::get_viewed_files(&pool, &review.id).await.unwrap();
        assert_eq!(viewed_files.len(), 3);

        // Check that all files are returned
        for file_path in &file_paths {
            assert!(viewed_files.contains(&file_path.to_string()));
        }
    }

    #[tokio::test]
    async fn test_get_viewed_files_empty() {
        let pool = create_test_pool().await;
        let review = create_test_review(&pool).await;

        let viewed_files = FileView::get_viewed_files(&pool, &review.id).await.unwrap();
        assert_eq!(viewed_files.len(), 0);
    }

    #[tokio::test]
    async fn test_is_file_viewed_false() {
        let pool = create_test_pool().await;
        let review = create_test_review(&pool).await;
        let file_path = "src/main.rs";

        let is_viewed = FileView::is_file_viewed(&pool, &review.id, file_path)
            .await
            .unwrap();
        assert!(!is_viewed);
    }

    #[tokio::test]
    async fn test_list_for_review() {
        let pool = create_test_pool().await;
        let review = create_test_review(&pool).await;

        let file_paths = vec!["src/main.rs", "src/lib.rs"];

        // Mark files as viewed
        for file_path in &file_paths {
            FileView::mark_as_viewed(&pool, &review.id, file_path)
                .await
                .unwrap();
        }

        let file_views = FileView::list_for_review(&pool, &review.id).await.unwrap();
        assert_eq!(file_views.len(), 2);

        // Check that file views contain correct data
        for file_view in file_views.iter() {
            assert_eq!(file_view.review_id, review.id);
            assert!(file_paths.contains(&file_view.file_path.as_str()));
            assert!(file_view.id > 0); // ID should be set by database
        }
    }

    #[tokio::test]
    async fn test_delete_for_review() {
        let pool = create_test_pool().await;
        let review = create_test_review(&pool).await;

        let file_paths = vec!["src/main.rs", "src/lib.rs"];

        // Mark files as viewed
        for file_path in &file_paths {
            FileView::mark_as_viewed(&pool, &review.id, file_path)
                .await
                .unwrap();
        }

        // Verify files are viewed
        let viewed_files = FileView::get_viewed_files(&pool, &review.id).await.unwrap();
        assert_eq!(viewed_files.len(), 2);

        // Delete all file views for review
        FileView::delete_for_review(&pool, &review.id)
            .await
            .unwrap();

        // Verify all file views are deleted
        let viewed_files = FileView::get_viewed_files(&pool, &review.id).await.unwrap();
        assert_eq!(viewed_files.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_reviews_isolated() {
        let pool = create_test_pool().await;
        let review1 = create_test_review(&pool).await;
        let review2 = create_test_review(&pool).await;
        let file_path = "src/main.rs";

        // Mark file as viewed for review1 only
        FileView::mark_as_viewed(&pool, &review1.id, file_path)
            .await
            .unwrap();

        // Check that file is viewed for review1 but not review2
        assert!(
            FileView::is_file_viewed(&pool, &review1.id, file_path)
                .await
                .unwrap()
        );
        assert!(
            !FileView::is_file_viewed(&pool, &review2.id, file_path)
                .await
                .unwrap()
        );

        // Get viewed files for each review
        let viewed_files1 = FileView::get_viewed_files(&pool, &review1.id)
            .await
            .unwrap();
        let viewed_files2 = FileView::get_viewed_files(&pool, &review2.id)
            .await
            .unwrap();

        assert_eq!(viewed_files1.len(), 1);
        assert_eq!(viewed_files2.len(), 0);
        assert_eq!(viewed_files1[0], file_path);
    }
}
