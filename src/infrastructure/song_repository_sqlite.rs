use crate::database::SqliteDb;
use crate::domain::{Song, SongRepository};
use std::sync::Arc;

pub struct SqliteSongRepository {
    db: Arc<SqliteDb>,
}

impl SongRepository for SqliteSongRepository {
    fn new(db: Arc<SqliteDb>) -> Self {
        Self { db }
    }

    async fn add_all(&self, songs: Vec<Song>) {
        for val in songs {
            let _ = sqlx::query(
            "INSERT INTO song (title, artist, release_year, album, remix, search_blob, file_path)
                    VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING;",
        )
        .bind(val.title)
        .bind(val.artist)
        .bind(val.release_year)
        .bind(val.album)
        .bind(val.remix)
        .bind(val.search_blob)
        .bind(val.file_path)
        .execute(&self.db.pool)
        .await;
        }
    }

    async fn get_all(&self) -> Vec<Song> {
        sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path FROM song"
            )
            .fetch_all(&self.db.pool)
            .await
            .unwrap()
    }

    //TODO: change location - temporary for testing
    //implement full sqlite alternative for comparison
    async fn search_by(
        &self,
        songs: &Vec<Song>,
        search: Vec<&str>,
        max_results: usize,
    ) -> Vec<Song> {
        songs
            .iter()
            .filter(|s| {
                search
                    .iter()
                    .all(|x| !x.is_empty() && s.search_blob.contains(x))
            })
            .take(max_results)
            .cloned()
            .collect()
    }
}
