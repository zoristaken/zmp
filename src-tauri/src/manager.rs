use sqlx::{Database, Pool};

use crate::sqlite::SqliteDb;

pub struct AppState {
    pub db: SqliteDb,
}

pub trait HasPool<DB: Database> {
    fn pool(&self) -> &Pool<DB>;
}
