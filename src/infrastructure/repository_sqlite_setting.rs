use crate::database::SqliteDb;
use crate::domain::{Setting, SettingRepository};
use std::sync::Arc;

pub struct SqliteSettingRepository {
    db: Arc<SqliteDb>,
}

impl SqliteSettingRepository {
    pub fn new(db: Arc<SqliteDb>) -> Self {
        Self { db }
    }
}

impl SettingRepository for SqliteSettingRepository {
    async fn set(&self, key: &str, value: &str) {
        let _ = sqlx::query("INSERT INTO setting (key, value) VALUES ($1, $2);")
            .bind(key)
            .bind(value)
            .execute(&self.db.pool)
            .await;
    }

    async fn get(&self, key: &str) -> Setting {
        sqlx::query_as::<sqlx::Sqlite, Setting>("SELECT id, key, value FROM setting WHERE key = ?")
            .bind(key)
            .fetch_one(&self.db.pool)
            .await
            .unwrap()
    }
}
