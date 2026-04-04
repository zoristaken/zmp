use crate::sqlite::SqliteDb;

pub struct AppState {
    pub db: SqliteDb,
}
