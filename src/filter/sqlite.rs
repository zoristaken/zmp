use crate::database::SqliteDb;
use crate::filter::{Filter, FilterRepository};
use std::sync::Arc;

pub struct SqliteFilterRepository {
    db: Arc<SqliteDb>,
}

impl SqliteFilterRepository {
    pub fn new(db: Arc<SqliteDb>) -> Self {
        Self { db }
    }
}

impl FilterRepository for SqliteFilterRepository {
    async fn add(&self, name: &str) {
        let _ = sqlx::query("INSERT INTO filter (name) VALUES ($1);")
            .bind(name)
            .execute(&self.db.pool)
            .await;
    }
    async fn get_all(&self) -> Vec<Filter> {
        sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter")
            .fetch_all(&self.db.pool)
            .await
            .unwrap()
    }

    //TODO: add name index?
    async fn get_by_name(&self, name: &str) -> Filter {
        sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter WHERE name = ?")
            .bind(name)
            .fetch_one(&self.db.pool)
            .await
            .unwrap()
    }

    async fn get_by_id(&self, filter_id: i32) -> Filter {
        sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter WHERE id = ?")
            .bind(filter_id)
            .fetch_one(&self.db.pool)
            .await
            .unwrap()
    }
}
