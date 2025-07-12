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
