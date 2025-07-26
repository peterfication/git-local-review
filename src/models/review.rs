use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::time_provider::{SystemTimeProvider, TimeProvider};

const SHORT_SHA_LENGTH: usize = 7;

#[derive(Debug, Clone, FromRow)]
pub struct Review {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub base_branch: String,
    pub target_branch: String,
    pub base_sha: Option<String>,
    pub target_sha: Option<String>,
}

impl PartialEq for Review {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Review {
    pub fn new(base_branch: String, target_branch: String) -> Self {
        Self::new_with_time_provider(base_branch, target_branch, &SystemTimeProvider)
    }

    pub fn new_with_shas(
        base_branch: String,
        target_branch: String,
        base_sha: Option<String>,
        target_sha: Option<String>,
    ) -> Self {
        Self::new_with_shas_and_time_provider(
            base_branch,
            target_branch,
            base_sha,
            target_sha,
            &SystemTimeProvider,
        )
    }

    pub fn new_with_time_provider(
        base_branch: String,
        target_branch: String,
        time_provider: &dyn TimeProvider,
    ) -> Self {
        Self::new_with_shas_and_time_provider(base_branch, target_branch, None, None, time_provider)
    }

    pub fn new_with_shas_and_time_provider(
        base_branch: String,
        target_branch: String,
        base_sha: Option<String>,
        target_sha: Option<String>,
        time_provider: &dyn TimeProvider,
    ) -> Self {
        let now = time_provider.now();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
            base_branch,
            target_branch,
            base_sha,
            target_sha,
        }
    }

    /// Returns a human-readable title for the review in the format "base_branch -> target_branch"
    pub fn title(&self) -> String {
        let default_sha = "unknown".to_string();
        let base_sha = self.base_sha.as_ref().unwrap_or(&default_sha);
        let target_sha = self.target_sha.as_ref().unwrap_or(&default_sha);
        let base_sha_short = base_sha.chars().take(SHORT_SHA_LENGTH).collect::<String>();
        let target_sha_short = target_sha
            .chars()
            .take(SHORT_SHA_LENGTH)
            .collect::<String>();
        format!(
            "{} ({}) -> {} ({})",
            self.base_branch, base_sha_short, self.target_branch, target_sha_short
        )
    }

    #[cfg(test)]
    pub fn test_review(opts: impl Into<TestReviewParams>) -> Self {
        let opts = opts.into();
        Self::new_with_shas(
            opts.base_branch,
            opts.target_branch,
            opts.base_sha,
            opts.target_sha,
        )
    }

    #[cfg(test)]
    pub fn test_review_with_time_provider(
        opts: impl Into<TestReviewParams>,
        time_provider: &dyn TimeProvider,
    ) -> Self {
        let opts = opts.into();
        Self::new_with_shas_and_time_provider(
            opts.base_branch,
            opts.target_branch,
            opts.base_sha,
            opts.target_sha,
            time_provider,
        )
    }

    pub async fn save(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO reviews (id, created_at, updated_at, base_branch, target_branch, base_sha, target_sha)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(&self.id)
        .bind(self.created_at.to_rfc3339())
        .bind(self.updated_at.to_rfc3339())
        .bind(&self.base_branch)
        .bind(&self.target_branch)
        .bind(&self.base_sha)
        .bind(&self.target_sha)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_all(pool: &SqlitePool) -> Result<Vec<Review>, sqlx::Error> {
        let reviews = sqlx::query_as::<_, Review>(
            r#"
            SELECT id, created_at, updated_at, base_branch, target_branch, base_sha, target_sha
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
            SELECT id, created_at, updated_at, base_branch, target_branch, base_sha, target_sha
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
pub struct TestReviewParams {
    pub base_branch: String,
    pub target_branch: String,
    pub base_sha: Option<String>,
    pub target_sha: Option<String>,
}

#[cfg(test)]
impl TestReviewParams {
    pub fn new() -> Self {
        Self {
            base_branch: "default".to_string(),
            target_branch: "default".to_string(),
            base_sha: None,
            target_sha: None,
        }
    }

    pub fn base_branch(mut self, base_branch: &str) -> Self {
        self.base_branch = base_branch.to_string();
        self
    }

    pub fn target_branch(mut self, target_branch: &str) -> Self {
        self.target_branch = target_branch.to_string();
        self
    }

    pub fn base_sha(mut self, base_sha: &str) -> Self {
        self.base_sha = Some(base_sha.to_string());
        self
    }

    pub fn target_sha(mut self, target_sha: &str) -> Self {
        self.target_sha = Some(target_sha.to_string());
        self
    }
}

#[cfg(test)]
impl Default for TestReviewParams {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl From<()> for TestReviewParams {
    fn from(_: ()) -> Self {
        Self::new()
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
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    #[test]
    fn test_review_new() {
        let base_branch = "main".to_string();
        let target_branch = "feature/test".to_string();
        let review = Review::new(base_branch.clone(), target_branch.clone());

        assert!(!review.id.is_empty());
        assert_eq!(review.created_at, review.updated_at);
        assert_eq!(review.base_branch, base_branch);
        assert_eq!(review.target_branch, target_branch);

        // ID should be a valid UUID
        assert!(uuid::Uuid::parse_str(&review.id).is_ok());
    }

    #[test]
    fn test_review_new_with_mock_time() {
        let fixed_time = fixed_time();
        let time_provider = MockTimeProvider::new(fixed_time);

        let review = Review::new_with_time_provider(
            "default".to_string(),
            "default".to_string(),
            &time_provider,
        );

        assert!(!review.id.is_empty());
        assert_eq!(review.created_at, fixed_time);
        assert_eq!(review.updated_at, fixed_time);
        assert_eq!(review.created_at, review.updated_at);
        assert_eq!(review.base_branch, "default".to_string());
        assert_eq!(review.target_branch, "default".to_string());

        // ID should be a valid UUID
        assert!(uuid::Uuid::parse_str(&review.id).is_ok());
    }

    #[test]
    fn test_review_new_time_provider() {
        let base_branch = "develop".to_string();
        let target_branch = "feature/branch-support".to_string();
        let fixed_time = fixed_time();
        let time_provider = MockTimeProvider::new(fixed_time);

        let review = Review::new_with_time_provider(
            base_branch.clone(),
            target_branch.clone(),
            &time_provider,
        );

        assert!(!review.id.is_empty());
        assert_eq!(review.created_at, fixed_time);
        assert_eq!(review.updated_at, fixed_time);
        assert_eq!(review.base_branch, base_branch);
        assert_eq!(review.target_branch, target_branch);

        // ID should be a valid UUID
        assert!(uuid::Uuid::parse_str(&review.id).is_ok());
    }

    #[test]
    fn test_title() {
        let review = Review::test_review(());
        assert_eq!(review.title(), "default (unknown) -> default (unknown)");
        let custom_review = Review::test_review(
            TestReviewParams::new()
                .base_branch("main")
                .target_branch("feature/test")
                .base_sha("abcd1234")
                .target_sha("efgh5678"),
        );
        assert_eq!(
            custom_review.title(),
            "main (abcd123) -> feature/test (efgh567)"
        );
    }

    #[tokio::test]
    async fn test_review_save_and_list() {
        let pool = create_test_pool().await;
        let review = Review::test_review(());

        // Save the review
        review.save(&pool).await.unwrap();

        // List all reviews
        let reviews = Review::list_all(&pool).await.unwrap();

        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].id, review.id);
        assert_eq!(reviews[0].base_branch, "default".to_string());
        assert_eq!(reviews[0].target_branch, "default".to_string());
    }

    #[tokio::test]
    async fn test_review_save_and_list_with_branches() {
        let pool = create_test_pool().await;
        let base_branch = "main".to_string();
        let target_branch = "feature/test".to_string();
        let review = Review::test_review(
            TestReviewParams::new()
                .base_branch(&base_branch)
                .target_branch(&target_branch),
        );

        // Save the review
        review.save(&pool).await.unwrap();

        // List all reviews
        let reviews = Review::list_all(&pool).await.unwrap();

        assert_eq!(reviews.len(), 1);
        assert_eq!(reviews[0].id, review.id);
        assert_eq!(reviews[0].base_branch, base_branch);
        assert_eq!(reviews[0].target_branch, target_branch);

        // Find by ID
        let found_review = Review::find_by_id(&pool, &review.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found_review.base_branch, base_branch);
        assert_eq!(found_review.target_branch, target_branch);
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

        let review1 = Review::test_review_with_time_provider(
            TestReviewParams::default().base_branch("First Review"),
            &time_provider1,
        );
        let review2 = Review::test_review_with_time_provider(
            TestReviewParams::default().base_branch("Second Review"),
            &time_provider2,
        );

        // Save in order
        review1.save(&pool).await.unwrap();
        review2.save(&pool).await.unwrap();

        let reviews = Review::list_all(&pool).await.unwrap();

        assert_eq!(reviews.len(), 2);
        // Should be ordered by created_at DESC, so newest first
        assert_eq!(reviews[0].base_branch, "Second Review");
        assert_eq!(reviews[1].base_branch, "First Review");
        assert!(reviews[0].created_at > reviews[1].created_at);
    }

    #[tokio::test]
    async fn test_review_save_duplicate_id_fails() {
        let pool = create_test_pool().await;

        let review1 = Review::test_review(());
        let mut review2 = Review::test_review(());

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
        let review = Review::test_review(());

        // Save the review
        review.save(&pool).await.unwrap();

        // Find by ID
        let found_review = Review::find_by_id(&pool, &review.id).await.unwrap();
        assert!(found_review.is_some());
        let found_review = found_review.unwrap();
        assert_eq!(found_review.id, review.id);
        assert_eq!(found_review.created_at, review.created_at);
        assert_eq!(found_review.updated_at, review.updated_at);

        // Find by non-existent ID
        let not_found = Review::find_by_id(&pool, "non-existent-id").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_review_delete() {
        let pool = create_test_pool().await;
        let review = Review::test_review(());

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

        let review1 = Review::test_review_with_time_provider(
            TestReviewParams::default().base_branch("main"),
            &time_provider1,
        );
        let mut review2 = Review::test_review_with_time_provider(
            TestReviewParams::default().base_branch("dev"),
            &time_provider2,
        );

        // Make review2 have the same ID as review1
        review2.id = review1.id.clone();

        // Should be equal despite different base_branch and timestamps
        assert_eq!(review1, review2);
    }

    #[test]
    fn test_review_eq_different_id() {
        let fixed_time = fixed_time();
        let time_provider = MockTimeProvider::new(fixed_time);

        let review1 = Review::test_review_with_time_provider(
            TestReviewParams::default().base_branch("Same Title"),
            &time_provider,
        );
        let review2 = Review::test_review_with_time_provider(
            TestReviewParams::default().base_branch("Same Title"),
            &time_provider,
        );

        // Should not be equal despite same base_branch and timestamps because IDs are different
        assert_ne!(review1, review2);
    }

    #[test]
    fn test_review_eq_self() {
        let review = Review::test_review(());

        // Should be equal to itself
        assert_eq!(review, review);
    }

    #[test]
    fn test_review_eq_clone() {
        let review1 = Review::test_review(());
        let review2 = review1.clone();

        // Clone should be equal to original
        assert_eq!(review1, review2);
    }

    #[test]
    fn test_review_test_helper() {
        // Test with all defaults
        let review1 = Review::test_review(());
        assert_eq!(review1.base_branch, "default");
        assert_eq!(review1.target_branch, "default");

        // Test with custom base_branch only
        let review2 = Review::test_review(TestReviewParams::new().base_branch("Custom Title"));
        assert_eq!(review2.base_branch, "Custom Title");
        assert_eq!(review2.target_branch, "default");

        // Test with all custom values
        let review3 = Review::test_review(
            TestReviewParams::new()
                .base_branch("main")
                .target_branch("feature/test"),
        );
        assert_eq!(review3.base_branch, "main");
        assert_eq!(review3.target_branch, "feature/test");

        // Test using Default trait
        let review4 = Review::test_review(TestReviewParams::default());
        assert_eq!(review4.base_branch, "default");
        assert_eq!(review4.target_branch, "default");
    }

    #[test]
    fn test_review_eq_ignores_other_fields() {
        let time1 = fixed_time();
        let time2 = time1 + chrono::Duration::days(30);

        let time_provider1 = MockTimeProvider::new(time1);

        let review1 = Review::new_with_time_provider(
            "default".to_string(),
            "default".to_string(),
            &time_provider1,
        );
        let review2 = Review {
            id: review1.id.clone(),                        // Same ID
            created_at: time2,                             // Different created_at
            updated_at: time2,                             // Different updated_at
            base_branch: "different-base".to_string(),     // Different base_branch
            target_branch: "different-target".to_string(), // Different target_branch
            base_sha: Some("abc123".to_string()),          // Different base_sha
            target_sha: Some("def456".to_string()),        // Different target_sha
        };

        // Should be equal because only ID matters for equality
        assert_eq!(review1, review2);
    }

    #[tokio::test]
    async fn test_review_with_shas() {
        let pool = create_test_pool().await;
        let base_sha = Some("abc123def456789".to_string());
        let target_sha = Some("987654321fedcba".to_string());

        let review = Review::new_with_shas(
            "main".to_string(),
            "feature/test".to_string(),
            base_sha.clone(),
            target_sha.clone(),
        );

        // Verify SHAs are set correctly
        assert_eq!(review.base_sha, base_sha);
        assert_eq!(review.target_sha, target_sha);

        // Save and retrieve
        review.save(&pool).await.unwrap();
        let found_review = Review::find_by_id(&pool, &review.id)
            .await
            .unwrap()
            .unwrap();

        // Verify SHAs are persisted
        assert_eq!(found_review.base_sha, base_sha);
        assert_eq!(found_review.target_sha, target_sha);
        assert_eq!(found_review.base_branch, "main");
        assert_eq!(found_review.target_branch, "feature/test");
    }

    #[test]
    fn test_review_new_with_shas() {
        let base_sha = Some("abc123".to_string());
        let target_sha = Some("def456".to_string());

        let review = Review::new_with_shas(
            "main".to_string(),
            "feature/test".to_string(),
            base_sha.clone(),
            target_sha.clone(),
        );

        assert_eq!(review.base_branch, "main");
        assert_eq!(review.target_branch, "feature/test");
        assert_eq!(review.base_sha, base_sha);
        assert_eq!(review.target_sha, target_sha);
        assert!(!review.id.is_empty());
    }

    #[test]
    fn test_review_test_helper_with_shas() {
        let review = Review::test_review(
            TestReviewParams::new()
                .base_branch("main")
                .target_branch("feature/test")
                .base_sha("abc123")
                .target_sha("def456"),
        );

        assert_eq!(review.base_branch, "main");
        assert_eq!(review.target_branch, "feature/test");
        assert_eq!(review.base_sha, Some("abc123".to_string()));
        assert_eq!(review.target_sha, Some("def456".to_string()));
    }
}
