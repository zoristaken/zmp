use crate::database::SqliteDb;
use crate::domain::{SongFilter, SongFilterRepository};
use std::sync::Arc;

pub struct SqliteSongFilterRepository {
    db: Arc<SqliteDb>,
}

impl SqliteSongFilterRepository {
    pub fn new(db: Arc<SqliteDb>) -> Self {
        Self { db }
    }
}

impl SongFilterRepository for SqliteSongFilterRepository {
    async fn add(&self, song_filter: SongFilter) {
        let _ = sqlx::query(
            "INSERT INTO song_filter (song_id, filter_id)
                    VALUES ($1, $2)",
        )
        .bind(song_filter.song_id)
        .bind(song_filter.filter_id)
        .execute(&self.db.pool)
        .await;
    }
    async fn add_multiple(&self, song_filters: Vec<SongFilter>) {
        for val in song_filters {
            let _ = sqlx::query(
                "INSERT INTO song_filter (song_id, filter_id)
                    VALUES ($1, $2)",
            )
            .bind(val.song_id)
            .bind(val.filter_id)
            .execute(&self.db.pool)
            .await;
        }
    }

    async fn get_by_id(&self, id: i32) -> SongFilter {
        sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.db.pool)
        .await
        .unwrap()
    }

    //TODO: add filter_id index?
    async fn get_by_filter(&self, filter_id: i32) -> Vec<SongFilter> {
        sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE filter_id = ?",
        )
        .bind(filter_id)
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
    }

    //TODO: add song_id index?
    async fn get_by_song(&self, song_id: i32) -> Vec<SongFilter> {
        sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE song_id = ?",
        )
        .bind(song_id)
        .fetch_all(&self.db.pool)
        .await
        .unwrap()
    }

    async fn get_all(&self) -> Vec<SongFilter> {
        sqlx::query_as::<sqlx::Sqlite, SongFilter>("SELECT id, song_id, filter_id FROM song_filter")
            .fetch_all(&self.db.pool)
            .await
            .unwrap()
    }
}
