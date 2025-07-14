use crate::{database::Database, event::ReviewCreateData, models::review::Review};

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
