use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::time_provider::{SystemTimeProvider, TimeProvider};

pub type CommentId = String;

#[derive(Debug, Clone, FromRow)]
pub struct Comment {
    pub id: CommentId,
    pub review_id: String,
    pub file_path: String,
    pub line_number: Option<i64>, // None for file-level comments
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl PartialEq for Comment {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Comment {
    pub fn new_file_comment(review_id: String, file_path: String, content: String) -> Self {
        Self::new_file_comment_with_time_provider(
            review_id,
            file_path,
            content,
            &SystemTimeProvider,
        )
    }

    pub fn new_line_comment(
        review_id: String,
        file_path: String,
        line_number: i64,
        content: String,
    ) -> Self {
        Self::new_line_comment_with_time_provider(
            review_id,
            file_path,
            line_number,
            content,
            &SystemTimeProvider,
        )
    }

    pub fn new_file_comment_with_time_provider(
        review_id: String,
        file_path: String,
        content: String,
        time_provider: &dyn TimeProvider,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            review_id,
            file_path,
            line_number: None,
            content,
            created_at: time_provider.now(),
        }
    }

    pub fn new_line_comment_with_time_provider(
        review_id: String,
        file_path: String,
        line_number: i64,
        content: String,
        time_provider: &dyn TimeProvider,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            review_id,
            file_path,
            line_number: Some(line_number),
            content,
            created_at: time_provider.now(),
        }
    }

    pub fn is_file_comment(&self) -> bool {
        self.line_number.is_none()
    }

    pub fn is_line_comment(&self) -> bool {
        self.line_number.is_some()
    }

    /// Create a new comment in the database
    pub async fn create(&self, pool: &SqlitePool) -> color_eyre::Result<()> {
        let created_at_str = self.created_at.to_rfc3339();
        sqlx::query!(
            r#"
            INSERT INTO comments (id, review_id, file_path, line_number, content, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            self.id,
            self.review_id,
            self.file_path,
            self.line_number,
            self.content,
            created_at_str
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Find comments for a specific file (both file-level and line-level)
    pub async fn find_for_file(
        pool: &SqlitePool,
        review_id: &str,
        file_path: &str,
    ) -> color_eyre::Result<Vec<Comment>> {
        let rows = sqlx::query!(
            r#"
            SELECT id as "id!", review_id as "review_id!", file_path as "file_path!", line_number, content as "content!", created_at as "created_at!"
            FROM comments
            WHERE review_id = ? AND file_path = ?
            ORDER BY created_at DESC
            "#,
            review_id,
            file_path
        )
        .fetch_all(pool)
        .await?;

        let mut comments = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| color_eyre::eyre::eyre!("Failed to parse created_at: {}", e))?
                .with_timezone(&Utc);

            comments.push(Comment {
                id: row.id,
                review_id: row.review_id,
                file_path: row.file_path,
                line_number: row.line_number,
                content: row.content,
                created_at,
            });
        }

        Ok(comments)
    }

    /// Find comments for a specific line in a file
    pub async fn find_for_line(
        pool: &SqlitePool,
        review_id: &str,
        file_path: &str,
        line_number: i64,
    ) -> color_eyre::Result<Vec<Comment>> {
        let rows = sqlx::query!(
            r#"
            SELECT id as "id!", review_id as "review_id!", file_path as "file_path!", line_number, content as "content!", created_at as "created_at!"
            FROM comments
            WHERE review_id = ? AND file_path = ? AND line_number = ?
            ORDER BY created_at DESC
            "#,
            review_id,
            file_path,
            line_number
        )
        .fetch_all(pool)
        .await?;

        let mut comments = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| color_eyre::eyre::eyre!("Failed to parse created_at: {}", e))?
                .with_timezone(&Utc);

            comments.push(Comment {
                id: row.id,
                review_id: row.review_id,
                file_path: row.file_path,
                line_number: row.line_number,
                content: row.content,
                created_at,
            });
        }

        Ok(comments)
    }

    /// Find file-level comments only
    pub async fn find_file_comments(
        pool: &SqlitePool,
        review_id: &str,
        file_path: &str,
    ) -> color_eyre::Result<Vec<Comment>> {
        let rows = sqlx::query!(
            r#"
            SELECT id as "id!", review_id as "review_id!", file_path as "file_path!", line_number, content as "content!", created_at as "created_at!"
            FROM comments
            WHERE review_id = ? AND file_path = ? AND line_number IS NULL
            ORDER BY created_at DESC
            "#,
            review_id,
            file_path
        )
        .fetch_all(pool)
        .await?;

        let mut comments = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| color_eyre::eyre::eyre!("Failed to parse created_at: {}", e))?
                .with_timezone(&Utc);

            comments.push(Comment {
                id: row.id,
                review_id: row.review_id,
                file_path: row.file_path,
                line_number: row.line_number,
                content: row.content,
                created_at,
            });
        }

        Ok(comments)
    }

    /// Check if a file has any comments (file-level or line-level)
    pub async fn file_has_comments(
        pool: &SqlitePool,
        review_id: &str,
        file_path: &str,
    ) -> color_eyre::Result<bool> {
        let count: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as count
            FROM comments
            WHERE review_id = ? AND file_path = ?
            "#,
            review_id,
            file_path
        )
        .fetch_one(pool)
        .await?;

        Ok(count > 0)
    }

    /// Check if a specific line has comments
    pub async fn line_has_comments(
        pool: &SqlitePool,
        review_id: &str,
        file_path: &str,
        line_number: i64,
    ) -> color_eyre::Result<bool> {
        let count: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as count
            FROM comments
            WHERE review_id = ? AND file_path = ? AND line_number = ?
            "#,
            review_id,
            file_path,
            line_number
        )
        .fetch_one(pool)
        .await?;

        Ok(count > 0)
    }

    /// Get all files with comments for a review
    pub async fn get_files_with_comments(
        pool: &SqlitePool,
        review_id: &str,
    ) -> color_eyre::Result<Vec<String>> {
        let files = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT file_path
            FROM comments
            WHERE review_id = ?
            ORDER BY file_path
            "#,
            review_id
        )
        .fetch_all(pool)
        .await?;

        Ok(files)
    }

    /// Get all line numbers with comments for a specific file
    pub async fn get_lines_with_comments(
        pool: &SqlitePool,
        review_id: &str,
        file_path: &str,
    ) -> color_eyre::Result<Vec<i64>> {
        let lines = sqlx::query_scalar!(
            r#"
            SELECT DISTINCT line_number
            FROM comments
            WHERE review_id = ? AND file_path = ? AND line_number IS NOT NULL
            ORDER BY line_number
            "#,
            review_id,
            file_path
        )
        .fetch_all(pool)
        .await?;

        Ok(lines.into_iter().flatten().collect())
    }

    /// Delete a comment by ID
    pub async fn delete(pool: &SqlitePool, comment_id: &str) -> color_eyre::Result<()> {
        sqlx::query!("DELETE FROM comments WHERE id = ?", comment_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete all comments for a review
    pub async fn delete_for_review(pool: &SqlitePool, review_id: &str) -> color_eyre::Result<()> {
        sqlx::query!("DELETE FROM comments WHERE review_id = ?", review_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    #[cfg(test)]
    pub fn test_file_comment(review_id: String, file_path: String, content: String) -> Self {
        Self::new_file_comment(review_id, file_path, content)
    }

    #[cfg(test)]
    pub fn test_line_comment(
        review_id: String,
        file_path: String,
        line_number: i64,
        content: String,
    ) -> Self {
        Self::new_line_comment(review_id, file_path, line_number, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    #[test]
    fn test_comment_creation() {
        let file_comment = Comment::new_file_comment(
            "review-123".to_string(),
            "src/main.rs".to_string(),
            "This is a file comment".to_string(),
        );

        assert!(file_comment.is_file_comment());
        assert!(!file_comment.is_line_comment());
        assert_eq!(file_comment.review_id, "review-123");
        assert_eq!(file_comment.file_path, "src/main.rs");
        assert_eq!(file_comment.content, "This is a file comment");
        assert_eq!(file_comment.line_number, None);

        let line_comment = Comment::new_line_comment(
            "review-123".to_string(),
            "src/main.rs".to_string(),
            42,
            "This is a line comment".to_string(),
        );

        assert!(!line_comment.is_file_comment());
        assert!(line_comment.is_line_comment());
        assert_eq!(line_comment.review_id, "review-123");
        assert_eq!(line_comment.file_path, "src/main.rs");
        assert_eq!(line_comment.content, "This is a line comment");
        assert_eq!(line_comment.line_number, Some(42));
    }

    #[tokio::test]
    async fn test_comment_crud_operations() {
        let pool = create_test_pool().await;

        // Create a test review first to satisfy foreign key constraint
        let review = crate::models::Review::test_review(());
        review.save(&pool).await.unwrap();

        // Create file comment
        let file_comment = Comment::new_file_comment(
            review.id.clone(),
            "src/main.rs".to_string(),
            "File comment".to_string(),
        );
        file_comment.create(&pool).await.unwrap();

        // Create line comment
        let line_comment = Comment::new_line_comment(
            review.id.clone(),
            "src/main.rs".to_string(),
            10,
            "Line comment".to_string(),
        );
        line_comment.create(&pool).await.unwrap();

        // Test find_for_file (should return both comments)
        let file_comments = Comment::find_for_file(&pool, &review.id, "src/main.rs")
            .await
            .unwrap();
        assert_eq!(file_comments.len(), 2);

        // Test find_for_line (should return only line comment)
        let line_comments = Comment::find_for_line(&pool, &review.id, "src/main.rs", 10)
            .await
            .unwrap();
        assert_eq!(line_comments.len(), 1);
        assert_eq!(line_comments[0].content, "Line comment");

        // Test find_file_comments (should return only file comment)
        let only_file_comments = Comment::find_file_comments(&pool, &review.id, "src/main.rs")
            .await
            .unwrap();
        assert_eq!(only_file_comments.len(), 1);
        assert_eq!(only_file_comments[0].content, "File comment");

        // Test file_has_comments
        assert!(
            Comment::file_has_comments(&pool, &review.id, "src/main.rs")
                .await
                .unwrap()
        );
        assert!(
            !Comment::file_has_comments(&pool, &review.id, "src/other.rs")
                .await
                .unwrap()
        );

        // Test line_has_comments
        assert!(
            Comment::line_has_comments(&pool, &review.id, "src/main.rs", 10)
                .await
                .unwrap()
        );
        assert!(
            !Comment::line_has_comments(&pool, &review.id, "src/main.rs", 20)
                .await
                .unwrap()
        );

        // Test get_files_with_comments
        let files = Comment::get_files_with_comments(&pool, &review.id)
            .await
            .unwrap();
        assert_eq!(files, vec!["src/main.rs"]);

        // Test get_lines_with_comments
        let lines = Comment::get_lines_with_comments(&pool, &review.id, "src/main.rs")
            .await
            .unwrap();
        assert_eq!(lines, vec![10]);
    }

    #[tokio::test]
    async fn test_comment_deletion() {
        let pool = create_test_pool().await;

        // Create a test review first to satisfy foreign key constraint
        let review = crate::models::Review::test_review(());
        review.save(&pool).await.unwrap();

        let comment = Comment::new_file_comment(
            review.id.clone(),
            "src/main.rs".to_string(),
            "Test comment".to_string(),
        );
        comment.create(&pool).await.unwrap();

        // Verify comment exists
        assert!(
            Comment::file_has_comments(&pool, &review.id, "src/main.rs")
                .await
                .unwrap()
        );

        // Delete comment
        Comment::delete(&pool, &comment.id).await.unwrap();

        // Verify comment is deleted
        assert!(
            !Comment::file_has_comments(&pool, &review.id, "src/main.rs")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_delete_for_review() {
        let pool = create_test_pool().await;

        // Create a test review first to satisfy foreign key constraint
        let review = crate::models::Review::test_review(());
        review.save(&pool).await.unwrap();

        // Create multiple comments for the same review
        let comment1 = Comment::new_file_comment(
            review.id.clone(),
            "src/main.rs".to_string(),
            "Comment 1".to_string(),
        );
        comment1.create(&pool).await.unwrap();

        let comment2 = Comment::new_line_comment(
            review.id.clone(),
            "src/lib.rs".to_string(),
            5,
            "Comment 2".to_string(),
        );
        comment2.create(&pool).await.unwrap();

        // Verify comments exist
        assert!(
            Comment::file_has_comments(&pool, &review.id, "src/main.rs")
                .await
                .unwrap()
        );
        assert!(
            Comment::file_has_comments(&pool, &review.id, "src/lib.rs")
                .await
                .unwrap()
        );

        // Delete all comments for review
        Comment::delete_for_review(&pool, &review.id).await.unwrap();

        // Verify all comments are deleted
        assert!(
            !Comment::file_has_comments(&pool, &review.id, "src/main.rs")
                .await
                .unwrap()
        );
        assert!(
            !Comment::file_has_comments(&pool, &review.id, "src/lib.rs")
                .await
                .unwrap()
        );
    }
}
