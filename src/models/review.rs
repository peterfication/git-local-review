use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::time_provider::{SystemTimeProvider, TimeProvider};

const SHORT_SHA_LENGTH: usize = 7;

pub type ReviewId = String;

#[derive(Debug, Clone, FromRow)]
pub struct Review {
    pub id: ReviewId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub base_branch: String,
    pub target_branch: String,
    pub base_sha: Option<String>,
    pub target_sha: Option<String>,
    pub base_sha_changed: Option<String>,
    pub target_sha_changed: Option<String>,
    pub base_branch_exists: Option<bool>,
    pub target_branch_exists: Option<bool>,
}

impl PartialEq for Review {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Review {
    pub fn builder() -> ReviewBuilder {
        ReviewBuilder::new()
    }

    /// Returns a human-readable title for the review in the format "base_branch -> target_branch"
    pub fn title(&self) -> String {
        let base_title = Self::format_branch_title(
            &self.base_branch,
            self.base_sha.as_ref(),
            self.base_sha_changed.as_ref(),
        );
        let target_title = Self::format_branch_title(
            &self.target_branch,
            self.target_sha.as_ref(),
            self.target_sha_changed.as_ref(),
        );
        format!("{base_title} -> {target_title}")
    }

    fn format_branch_title(
        branch_name: &str,
        original_sha: Option<&String>,
        changed_sha: Option<&String>,
    ) -> String {
        let original_short = Self::format_short_sha(original_sha);
        if let Some(changed_sha) = changed_sha {
            let changed_short = Self::format_short_sha(Some(changed_sha));
            format!("{branch_name} ({original_short} -> {changed_short})")
        } else {
            format!("{branch_name} ({original_short})")
        }
    }

    fn format_short_sha(sha: Option<&String>) -> String {
        sha.map(|value| value.chars().take(SHORT_SHA_LENGTH).collect())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub async fn save(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let created_at = self.created_at.to_rfc3339();
        let updated_at = self.updated_at.to_rfc3339();
        sqlx::query!(
            r#"
            INSERT INTO reviews (id, created_at, updated_at, base_branch, target_branch, base_sha, target_sha, base_sha_changed, target_sha_changed, base_branch_exists, target_branch_exists)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            self.id,
            created_at,
            updated_at,
            self.base_branch,
            self.target_branch,
            self.base_sha,
            self.target_sha,
            self.base_sha_changed,
            self.target_sha_changed,
            self.base_branch_exists,
            self.target_branch_exists
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn list_all(pool: &SqlitePool) -> Result<Vec<Review>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"
            SELECT id as "id!", created_at as "created_at!", updated_at as "updated_at!", base_branch as "base_branch!", target_branch as "target_branch!", base_sha, target_sha, base_sha_changed, target_sha_changed, base_branch_exists, target_branch_exists
            FROM reviews
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(pool)
        .await?;

        let mut reviews = Vec::new();
        for row in rows {
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                .with_timezone(&Utc);
            reviews.push(Review {
                id: row.id,
                created_at,
                updated_at,
                base_branch: row.base_branch,
                target_branch: row.target_branch,
                base_sha: row.base_sha,
                target_sha: row.target_sha,
                base_sha_changed: row.base_sha_changed,
                target_sha_changed: row.target_sha_changed,
                base_branch_exists: row.base_branch_exists,
                target_branch_exists: row.target_branch_exists,
            });
        }
        Ok(reviews)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Review>, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT id as "id!", created_at as "created_at!", updated_at as "updated_at!", base_branch as "base_branch!", target_branch as "target_branch!", base_sha, target_sha, base_sha_changed, target_sha_changed, base_branch_exists, target_branch_exists
            FROM reviews
            WHERE id = ?1
            "#,
            id
        )
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => {
                let created_at = DateTime::parse_from_rfc3339(&row.created_at)
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                    .with_timezone(&Utc);
                let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?
                    .with_timezone(&Utc);
                Ok(Some(Review {
                    id: row.id,
                    created_at,
                    updated_at,
                    base_branch: row.base_branch,
                    target_branch: row.target_branch,
                    base_sha: row.base_sha,
                    target_sha: row.target_sha,
                    base_sha_changed: row.base_sha_changed,
                    target_sha_changed: row.target_sha_changed,
                    base_branch_exists: row.base_branch_exists,
                    target_branch_exists: row.target_branch_exists,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn update_branch_status(
        &self,
        pool: &SqlitePool,
        base_sha_changed: Option<String>,
        target_sha_changed: Option<String>,
        base_branch_exists: Option<bool>,
        target_branch_exists: Option<bool>,
    ) -> Result<(), sqlx::Error> {
        let updated_at = self.updated_at.to_rfc3339();
        sqlx::query!(
            r#"
            UPDATE reviews 
            SET base_sha_changed = ?2, target_sha_changed = ?3, base_branch_exists = ?4, target_branch_exists = ?5, updated_at = ?6
            WHERE id = ?1
            "#,
            self.id,
            base_sha_changed,
            target_sha_changed,
            base_branch_exists,
            target_branch_exists,
            updated_at
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_shas(
        &self,
        pool: &SqlitePool,
        base_sha: Option<String>,
        target_sha: Option<String>,
        base_sha_changed: Option<String>,
        target_sha_changed: Option<String>,
    ) -> Result<(), sqlx::Error> {
        let updated_at = self.updated_at.to_rfc3339();
        sqlx::query!(
            r#"
            UPDATE reviews
            SET base_sha = ?2, target_sha = ?3, base_sha_changed = ?4, target_sha_changed = ?5, updated_at = ?6
            WHERE id = ?1
            "#,
            self.id,
            base_sha,
            target_sha,
            base_sha_changed,
            target_sha_changed,
            updated_at
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM reviews
            WHERE id = ?1
            "#,
            self.id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

pub struct ReviewBuilder {
    base_branch: Option<String>,
    target_branch: Option<String>,
    base_sha: Option<String>,
    target_sha: Option<String>,
    base_sha_changed: Option<String>,
    target_sha_changed: Option<String>,
    base_branch_exists: Option<bool>,
    target_branch_exists: Option<bool>,
}

impl ReviewBuilder {
    pub fn new() -> Self {
        Self {
            base_branch: None,
            target_branch: None,
            base_sha: None,
            target_sha: None,
            base_sha_changed: None,
            target_sha_changed: None,
            base_branch_exists: None,
            target_branch_exists: None,
        }
    }

    pub fn base_branch(mut self, base_branch: impl Into<String>) -> Self {
        self.base_branch = Some(base_branch.into());
        self
    }

    pub fn target_branch(mut self, target_branch: impl Into<String>) -> Self {
        self.target_branch = Some(target_branch.into());
        self
    }

    pub fn base_sha(mut self, base_sha: Option<String>) -> Self {
        self.base_sha = base_sha;
        self
    }

    pub fn base_sha_str(mut self, base_sha: &str) -> Self {
        self.base_sha = Some(base_sha.to_string());
        self
    }

    pub fn target_sha(mut self, target_sha: Option<String>) -> Self {
        self.target_sha = target_sha;
        self
    }

    pub fn target_sha_str(mut self, target_sha: &str) -> Self {
        self.target_sha = Some(target_sha.to_string());
        self
    }

    pub fn base_sha_changed(mut self, base_sha_changed: Option<String>) -> Self {
        self.base_sha_changed = base_sha_changed;
        self
    }

    pub fn target_sha_changed(mut self, target_sha_changed: Option<String>) -> Self {
        self.target_sha_changed = target_sha_changed;
        self
    }

    pub fn base_branch_exists(mut self, base_branch_exists: Option<bool>) -> Self {
        self.base_branch_exists = base_branch_exists;
        self
    }

    pub fn target_branch_exists(mut self, target_branch_exists: Option<bool>) -> Self {
        self.target_branch_exists = target_branch_exists;
        self
    }

    pub fn build(self) -> Review {
        self.build_with_time_provider(&SystemTimeProvider)
    }

    pub fn build_with_time_provider(self, time_provider: &dyn TimeProvider) -> Review {
        let base_branch = self.base_branch.unwrap_or_else(|| "default".to_string());
        let target_branch = self.target_branch.unwrap_or_else(|| "default".to_string());
        let now = time_provider.now();

        Review {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
            base_branch,
            target_branch,
            base_sha: self.base_sha,
            target_sha: self.target_sha,
            base_sha_changed: self.base_sha_changed,
            target_sha_changed: self.target_sha_changed,
            base_branch_exists: self.base_branch_exists,
            target_branch_exists: self.target_branch_exists,
        }
    }
}

impl Default for ReviewBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sqlx::SqlitePool;

    use crate::{test_utils::fixed_time, time_provider::MockTimeProvider};

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    #[test]
    fn test_review_new_with_mock_time() {
        let fixed_time = fixed_time();
        let time_provider = MockTimeProvider::new(fixed_time);

        let review = Review::builder()
            .base_branch("default")
            .target_branch("default")
            .build_with_time_provider(&time_provider);

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

        let review = Review::builder()
            .base_branch(base_branch.clone())
            .target_branch(target_branch.clone())
            .build_with_time_provider(&time_provider);

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
        let review = Review::builder().build();
        assert_eq!(review.title(), "default (unknown) -> default (unknown)");
        let custom_review = Review::builder()
            .base_branch("main")
            .target_branch("feature/test")
            .base_sha(Some("abcd1234".to_string()))
            .target_sha(Some("efgh5678".to_string()))
            .build();
        assert_eq!(
            custom_review.title(),
            "main (abcd123) -> feature/test (efgh567)"
        );

        let changed_review = Review::builder()
            .base_branch("main")
            .target_branch("feature/test")
            .base_sha(Some("abcd1234".to_string()))
            .base_sha_changed(Some("123456789".to_string()))
            .target_sha(Some("efgh5678".to_string()))
            .target_sha_changed(Some("ijkl9012".to_string()))
            .build();
        assert_eq!(
            changed_review.title(),
            "main (abcd123 -> 1234567) -> feature/test (efgh567 -> ijkl901)"
        );
    }

    #[tokio::test]
    async fn test_review_save_and_list() {
        let pool = create_test_pool().await;
        let review = Review::builder().build();

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
        let review = Review::builder()
            .base_branch(&base_branch)
            .target_branch(&target_branch)
            .build();

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

        let review1 = Review::builder()
            .base_branch("main")
            .build_with_time_provider(&time_provider1);
        let review2 = Review::builder()
            .base_branch("dev")
            .build_with_time_provider(&time_provider2);

        // Save in order
        review1.save(&pool).await.unwrap();
        review2.save(&pool).await.unwrap();

        let reviews = Review::list_all(&pool).await.unwrap();

        assert_eq!(reviews.len(), 2);
        // Should be ordered by created_at DESC, so newest first
        assert_eq!(reviews[0].base_branch, "dev");
        assert_eq!(reviews[1].base_branch, "main");
        assert!(reviews[0].created_at > reviews[1].created_at);
    }

    #[tokio::test]
    async fn test_review_save_duplicate_id_fails() {
        let pool = create_test_pool().await;

        let review1 = Review::builder().build();
        let mut review2 = Review::builder().build();

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
        let review = Review::builder().build();

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
        let review = Review::builder().build();

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

        let review1 = Review::builder()
            .base_branch("main")
            .build_with_time_provider(&time_provider1);
        let mut review2 = Review::builder()
            .base_branch("dev")
            .build_with_time_provider(&time_provider2);

        // Make review2 have the same ID as review1
        review2.id = review1.id.clone();

        // Should be equal despite different base_branch and timestamps
        assert_eq!(review1, review2);
    }

    #[test]
    fn test_review_eq_different_id() {
        let fixed_time = fixed_time();
        let time_provider = MockTimeProvider::new(fixed_time);

        let review1 = Review::builder()
            .base_branch("main")
            .build_with_time_provider(&time_provider);
        let review2 = Review::builder()
            .base_branch("main")
            .build_with_time_provider(&time_provider);

        // Should not be equal despite same base_branch and timestamps because IDs are different
        assert_ne!(review1, review2);
    }

    #[test]
    fn test_review_eq_self() {
        let review = Review::builder().build();

        // Should be equal to itself
        assert_eq!(review, review);
    }

    #[test]
    fn test_review_eq_clone() {
        let review1 = Review::builder().build();
        let review2 = review1.clone();

        // Clone should be equal to original
        assert_eq!(review1, review2);
    }

    #[test]
    fn test_review_test_helper() {
        // Test with all defaults
        let review1 = Review::builder().build();
        assert_eq!(review1.base_branch, "default");
        assert_eq!(review1.target_branch, "default");

        // Test with custom base_branch only
        let review2 = Review::builder().base_branch("Custom Title").build();
        assert_eq!(review2.base_branch, "Custom Title");
        assert_eq!(review2.target_branch, "default");

        // Test with all custom values
        let review3 = Review::builder()
            .base_branch("main")
            .target_branch("feature/test")
            .build();
        assert_eq!(review3.base_branch, "main");
        assert_eq!(review3.target_branch, "feature/test");

        // Test using Default trait
        let review4 = Review::builder().build();
        assert_eq!(review4.base_branch, "default");
        assert_eq!(review4.target_branch, "default");
    }

    #[test]
    fn test_review_eq_ignores_other_fields() {
        let time1 = fixed_time();
        let time2 = time1 + chrono::Duration::days(30);

        let time_provider1 = MockTimeProvider::new(time1);

        let review1 = Review::builder()
            .base_branch("default")
            .target_branch("default")
            .build_with_time_provider(&time_provider1);
        let review2 = Review {
            id: review1.id.clone(),                         // Same ID
            created_at: time2,                              // Different created_at
            updated_at: time2,                              // Different updated_at
            base_branch: "different-base".to_string(),      // Different base_branch
            target_branch: "different-target".to_string(),  // Different target_branch
            base_sha: Some("abc123".to_string()),           // Different base_sha
            target_sha: Some("def456".to_string()),         // Different target_sha
            base_sha_changed: Some("xyz789".to_string()),   // Different base_sha_changed
            target_sha_changed: Some("uvw321".to_string()), // Different target_sha_changed
            base_branch_exists: Some(false),                // Different base_branch_exists
            target_branch_exists: Some(true),               // Different target_branch_exists
        };

        // Should be equal because only ID matters for equality
        assert_eq!(review1, review2);
    }

    #[tokio::test]
    async fn test_review_with_shas() {
        let pool = create_test_pool().await;
        let base_sha = Some("abc123def456789".to_string());
        let target_sha = Some("987654321fedcba".to_string());

        let review = Review::builder()
            .base_branch("main")
            .target_branch("feature/test")
            .base_sha(base_sha.clone())
            .target_sha(target_sha.clone())
            .build();

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

    #[tokio::test]
    async fn test_update_branch_status() {
        let pool = create_test_pool().await;
        let mut review = Review::builder()
            .base_branch("main")
            .target_branch("feature/test")
            .base_sha_str("abcd1234")
            .target_sha_str("efgh5678")
            .build();
        review.save(&pool).await.unwrap();

        let new_time = review.updated_at + chrono::Duration::minutes(5);
        review.updated_at = new_time;

        review
            .update_branch_status(
                &pool,
                Some("abcd9876".to_string()),
                None,
                Some(true),
                Some(false),
            )
            .await
            .unwrap();

        let updated = Review::find_by_id(&pool, &review.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.base_sha_changed.as_deref(), Some("abcd9876"));
        assert_eq!(updated.target_sha_changed, None);
        assert_eq!(updated.base_branch_exists, Some(true));
        assert_eq!(updated.target_branch_exists, Some(false));
        assert_eq!(updated.updated_at, new_time);
    }

    #[test]
    fn test_review_new_with_shas() {
        let base_sha = Some("abc123".to_string());
        let target_sha = Some("def456".to_string());

        let review = Review::builder()
            .base_branch("main")
            .target_branch("feature/test")
            .base_sha(base_sha.clone())
            .target_sha(target_sha.clone())
            .build();

        assert_eq!(review.base_branch, "main");
        assert_eq!(review.target_branch, "feature/test");
        assert_eq!(review.base_sha, base_sha);
        assert_eq!(review.target_sha, target_sha);
        assert!(!review.id.is_empty());
    }

    #[test]
    fn test_review_test_helper_with_shas() {
        let review = Review::builder()
            .base_branch("main")
            .target_branch("feature/test")
            .base_sha(Some("abc123".to_string()))
            .target_sha(Some("def456".to_string()))
            .build();

        assert_eq!(review.base_branch, "main");
        assert_eq!(review.target_branch, "feature/test");
        assert_eq!(review.base_sha, Some("abc123".to_string()));
        assert_eq!(review.target_sha, Some("def456".to_string()));
    }
}
