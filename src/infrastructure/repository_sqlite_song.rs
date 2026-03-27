use crate::database::SqliteDb;
use crate::domain::{Song, SongRepository};
use std::sync::Arc;

pub struct SqliteSongRepository {
    db: Arc<SqliteDb>,
}

impl SqliteSongRepository {
    pub fn new(db: Arc<SqliteDb>) -> Self {
        Self { db }
    }
}

impl SongRepository for SqliteSongRepository {
    async fn add_all(&self, songs: Vec<Song>) {
        for val in songs {
            let _ = sqlx::query(
            "INSERT INTO song (title, artist, release_year, album, remix, search_blob, file_path, duration)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING;",
        )
        .bind(val.title)
        .bind(val.artist)
        .bind(val.release_year)
        .bind(val.album)
        .bind(val.remix)
        .bind(val.search_blob)
        .bind(val.file_path)
        .bind(val.duration)
        .execute(&self.db.pool)
        .await;
        }
    }

    async fn get_all(&self) -> Vec<Song> {
        sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration FROM song"
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

    async fn search_by_db(&self, words: Vec<&str>, max_results: i64) -> Vec<Song> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration FROM song WHERE ",
        );
        //TODO: after using the app, verify if searching by substrings actually happens
        //I have a feeling we can improve the performance by moving to a prefix search
        //without any side effects at all
        for (i, word) in words.iter().enumerate() {
            qb.push("search_blob LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" OR ");
            }
        }

        qb.push(" LIMIT ");
        qb.push_bind(max_results);

        let query = qb.build_query_as::<Song>();
        query.fetch_all(&self.db.pool).await.unwrap()
    }
}
