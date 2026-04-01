use anyhow::Context;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
};
use std::str::FromStr;

use crate::{
    filter::{Filter, FilterRepository},
    setting::{Setting, SettingRepository},
    song::{Song, SongRepository},
    song_filter::{SongFilter, SongFilterRepository},
};

#[derive(Clone)]
pub struct SqliteDb {
    pool: SqlitePool,
}

impl SqliteDb {
    pub async fn new(path: &str) -> anyhow::Result<SqliteDb> {
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::from_str(path)
                .with_context(|| format!("invalid database path {}", path))?
                .create_if_missing(true)
                .foreign_keys(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal),
        )
        .await
        .with_context(|| format!("failed to open database at {}", path))?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(SqliteDb { pool })
    }
}

impl SongRepository for SqliteDb {
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
        .execute(&self.pool)
        .await;
        }
    }

    async fn get_all(&self) -> Vec<Song> {
        sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration FROM song"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap()
    }

    //An in memory search alternative to the search_by_db query
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

    async fn search_by_db(&self, words: Vec<&str>, max_results: i32) -> Vec<Song> {
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
                qb.push(" AND ");
            }
        }

        qb.push(" LIMIT ");
        qb.push_bind(max_results);

        let query = qb.build_query_as::<Song>();
        query.fetch_all(&self.pool).await.unwrap()
    }

    async fn get_by_id(&self, id: i32) -> Song {
        sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration FROM song WHERE id = ?"
            )
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn get_by_title_artist(&self, title: String, artist: String) -> Song {
        sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration FROM song WHERE title = ? AND artist = ?"
            )
            .bind(title)
            .bind(artist)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

impl FilterRepository for SqliteDb {
    async fn add(&self, name: &str) {
        let _ = sqlx::query("INSERT INTO filter (name) VALUES ($1);")
            .bind(name)
            .execute(&self.pool)
            .await;
    }
    async fn get_all(&self) -> Vec<Filter> {
        sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter")
            .fetch_all(&self.pool)
            .await
            .unwrap()
    }

    //TODO: add name index?
    async fn get_by_name(&self, name: &str) -> Filter {
        sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn get_by_id(&self, filter_id: i32) -> Filter {
        sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter WHERE id = ?")
            .bind(filter_id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

impl SongFilterRepository for SqliteDb {
    async fn add(&self, song_filter: SongFilter) {
        let _ = sqlx::query(
            "INSERT INTO song_filter (song_id, filter_id)
                    VALUES ($1, $2)",
        )
        .bind(song_filter.song_id)
        .bind(song_filter.filter_id)
        .execute(&self.pool)
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
            .execute(&self.pool)
            .await;
        }
    }

    async fn get_by_id(&self, id: i32) -> SongFilter {
        sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    //TODO: add filter_id index?
    async fn get_by_filter(&self, filter_id: i32) -> Vec<SongFilter> {
        sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE filter_id = ?",
        )
        .bind(filter_id)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    //TODO: add song_id index?
    async fn get_by_song(&self, song_id: i32) -> Vec<SongFilter> {
        sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE song_id = ?",
        )
        .bind(song_id)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    async fn get_all(&self) -> Vec<SongFilter> {
        sqlx::query_as::<sqlx::Sqlite, SongFilter>("SELECT id, song_id, filter_id FROM song_filter")
            .fetch_all(&self.pool)
            .await
            .unwrap()
    }
}

impl SettingRepository for SqliteDb {
    async fn set(&self, key: &str, value: &str) {
        let _ = sqlx::query("INSERT INTO setting (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = ($2) WHERE key = ($1);")
                    .bind(key)
                    .bind(value)
                    .execute(&self.pool)
                    .await;
    }

    async fn get(&self, key: &str) -> anyhow::Result<Setting> {
        Ok(sqlx::query_as::<sqlx::Sqlite, Setting>(
            "SELECT id, key, value FROM setting WHERE key = ?",
        )
        .bind(key)
        .fetch_one(&self.pool)
        .await?)
    }
}
