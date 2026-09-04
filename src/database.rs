use sqlx::{SqlitePool, migrate, sqlite::SqliteConnectOptions};

#[cfg(test)]
use sqlx::sqlite::SqlitePoolOptions;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> color_eyre::Result<Self> {
        std::fs::create_dir_all("tmp")?;

        let options = SqliteConnectOptions::new()
            .filename("tmp/reviews.db")
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;

        migrate!().run(&pool).await?;

        log::info!("Database initialized at tmp/reviews.db with migrations");

        Ok(Self { pool })
    }

    #[cfg(test)]
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[cfg(test)]
    pub async fn new_for_test() -> color_eyre::Result<Self> {
        let options = SqliteConnectOptions::new().in_memory(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(self) -> color_eyre::Result<()> {
        self.pool.close().await;
        Ok(())
    }
}
