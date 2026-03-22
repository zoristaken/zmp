use anyhow::Context;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteQueryResult},
};
use std::str::FromStr;

pub struct SqliteDb {
    pub pool: SqlitePool,
}

impl SqliteDb {
    pub async fn new(path: &str) -> Result<SqliteDb, anyhow::Error> {
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::from_str(path)
                .with_context(|| format!("invalid database path {}", path))?
                .create_if_missing(true)
                .pragma("foreign_keys", "ON"),
        )
        .await
        .with_context(|| format!("failed to open database at {}", path))?;
        match Self::create_schema(&pool).await {
            Ok(_) => println!("Database created Sucessfully"),
            Err(e) => panic!("{}", e),
        }

        Ok(SqliteDb { pool })
    }

    async fn create_schema(pool: &SqlitePool) -> Result<SqliteQueryResult, sqlx::Error> {
        let create = "
            CREATE TABLE IF NOT EXISTS song
            (
                id                      INTEGER PRIMARY KEY AUTOINCREMENT,
                title                    TEXT                NOT NULL,
                artist                  TEXT,
                release_year            INT2,
                album                   TEXT,
                remix                   TEXT,
                search_blob             TEXT,
                UNIQUE(title, artist, remix)
            );

            CREATE TABLE IF NOT EXISTS filters
            (
                id                      INTEGER PRIMARY KEY AUTOINCREMENT,
                name                    TEXT,
                UNIQUE(name)
            );

            CREATE TABLE IF NOT EXISTS song_filter
            (
                id                      INTEGER PRIMARY KEY AUTOINCREMENT,
                song_id                 INT,
                filter_id               INT,
                FOREIGN KEY (song_id)   REFERENCES song (id) ON UPDATE SET NULL ON DELETE SET NULL,
                FOREIGN KEY (filter_id) REFERENCES filters (id) ON UPDATE SET NULL ON DELETE SET NULL,
                UNIQUE(song_id, filter_id)
            )";
        sqlx::query(create).execute(pool).await
    }
}
