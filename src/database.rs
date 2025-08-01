use sqlx::{SqlitePool, migrate, sqlite::SqliteConnectOptions};

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

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(self) -> color_eyre::Result<()> {
        self.pool.close().await;
        Ok(())
    }
}
