use anyhow::Context;
use async_trait::async_trait;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    Acquire, Database, Pool, Sqlite, SqlitePool,
};
use std::str::FromStr;

use crate::{
    filter::{Filter, FilterRepository},
    setting::{Setting, SettingRepository},
    song::{Song, SongRepository},
    song_filter::{SongFilter, SongFilterRepository},
};

pub trait RepositoryDb: Database + Send + Sync + 'static {}

pub trait HasPool<DB: RepositoryDb> {
    fn pool(&self) -> &Pool<DB>;
}

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

impl RepositoryDb for Sqlite {}

impl HasPool<Sqlite> for SqliteDb {
    fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }
}

#[async_trait]
impl SongRepository<Sqlite> for SqliteDb {
    async fn add_all<'a, A>(&self, acquiree: A, songs: Vec<Song>) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        for val in songs {
            let _ = sqlx::query(
            "INSERT OR IGNORE INTO song (title, artist, release_year, album, remix, search_blob, file_path, duration)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(val.title.clone())
        .bind(val.artist.clone())
        .bind(val.release_year.clone())
        .bind(val.album.clone())
        .bind(val.remix.clone())
        .bind(val.search_blob.clone())
        .bind(val.file_path.clone())
        .bind(val.duration.clone())
        .execute(&mut *conn)
        .await
        .expect(format!("failed inserting: {:#?}", val).as_str());
        }

        Ok(())
    }

    async fn get_all<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let songs = sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration FROM song"
            )
            .fetch_all(conn)
            .await?;
        Ok(songs)
    }

    //An in memory search alternative to the search_by_db query
    async fn search_by(
        &self,
        songs: &[Song],
        search: &[&str],
        max_results: usize,
    ) -> anyhow::Result<Vec<Song>> {
        Ok(songs
            .iter()
            .filter(|s| {
                search
                    .iter()
                    .all(|x| !x.is_empty() && s.search_blob.contains(x))
            })
            .take(max_results)
            .cloned()
            .collect())
    }

    async fn search_by_db_alternative<'a, A>(
        &self,
        acquiree: A,
        words: &[&str],
        max_results: i32,
    ) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT id, title, artist, release_year, album, remix, 
            search_blob, file_path, duration FROM song WHERE ",
        );
        //TODO: after using the app, verify if searching by substrings actually happens
        //I have a feeling we can improve the performance by moving to a prefix search
        //without any side effects at all
        for (i, word) in words.iter().enumerate() {
            qb.push("title LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" AND ");
            }
        }
        qb.push(" OR ");

        for (i, word) in words.iter().enumerate() {
            qb.push("artist LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" AND ");
            }
        }
        qb.push(" OR ");

        for (i, word) in words.iter().enumerate() {
            qb.push("release_year LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" AND ");
            }
        }

        qb.push(" OR ");

        for (i, word) in words.iter().enumerate() {
            qb.push("album LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" AND ");
            }
        }

        qb.push(" LIMIT ");
        qb.push_bind(max_results);

        let conn = &mut *acquiree.acquire().await?;

        let query = qb.build_query_as::<Song>();
        let songs = query.fetch_all(conn).await?;

        Ok(songs)
    }

    async fn search_by_db<'a, A>(
        &self,
        acquiree: A,
        words: &[&str],
        max_results: i32,
    ) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
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

        let conn = &mut *acquiree.acquire().await?;

        let query = qb.build_query_as::<Song>();
        let songs = query.fetch_all(conn).await?;

        Ok(songs)
    }

    async fn get_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song = sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration FROM song WHERE id = ?"
            )
            .bind(id)
            .fetch_one(conn)
            .await?;
        Ok(song)
    }

    async fn get_by_title_artist<'a, A>(
        &self,
        acquiree: A,
        title: &str,
        artist: &str,
    ) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song = sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration FROM song WHERE title = ? AND artist = ?"
            )
            .bind(title)
            .bind(artist)
            .fetch_one(conn)
            .await?;

        Ok(song)
    }
}

#[async_trait]
impl FilterRepository<Sqlite> for SqliteDb {
    async fn add<'a, A>(&self, acquiree: A, name: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let _ = sqlx::query("INSERT INTO filter (name) VALUES ($1);")
            .bind(name)
            .execute(conn)
            .await?;

        Ok(())
    }
    async fn get_all<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Filter>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;
        let filter = sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter")
            .fetch_all(conn)
            .await?;

        Ok(filter)
    }

    //TODO: add name index?
    async fn get_by_name<'a, A>(&self, acquiree: A, name: &str) -> anyhow::Result<Filter>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let filter =
            sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter WHERE name = ?")
                .bind(name)
                .fetch_one(conn)
                .await?;

        Ok(filter)
    }

    async fn get_by_id<'a, A>(&self, acquiree: A, filter_id: i32) -> anyhow::Result<Filter>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let filter =
            sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter WHERE id = ?")
                .bind(filter_id)
                .fetch_one(conn)
                .await?;

        Ok(filter)
    }
}

#[async_trait]
impl SongFilterRepository<Sqlite> for SqliteDb {
    async fn add<'a, A>(&self, acquiree: A, song_filter: SongFilter) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        sqlx::query(
            "INSERT INTO song_filter (song_id, filter_id)
                    VALUES ($1, $2)",
        )
        .bind(song_filter.song_id)
        .bind(song_filter.filter_id)
        .execute(conn)
        .await?;

        Ok(())
    }
    async fn add_multiple<'a, A>(
        &self,
        acquiree: A,
        song_filters: Vec<SongFilter>,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        for val in song_filters {
            let _ = sqlx::query(
                "INSERT INTO song_filter (song_id, filter_id)
                    VALUES ($1, $2)",
            )
            .bind(val.song_id)
            .bind(val.filter_id)
            .execute(&mut *conn)
            .await?;
        }

        Ok(())
    }

    async fn get_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<SongFilter>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song_filter = sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE id = ?",
        )
        .bind(id)
        .fetch_one(conn)
        .await?;

        Ok(song_filter)
    }

    //TODO: add filter_id index?
    async fn get_by_filter<'a, A>(
        &self,
        acquiree: A,
        filter_id: i32,
    ) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song_filter = sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE filter_id = ?",
        )
        .bind(filter_id)
        .fetch_all(conn)
        .await?;

        Ok(song_filter)
    }

    //TODO: add song_id index?
    async fn get_by_song<'a, A>(&self, acquiree: A, song_id: i32) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song_filter = sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE song_id = ?",
        )
        .bind(song_id)
        .fetch_all(conn)
        .await?;

        Ok(song_filter)
    }

    async fn get_all<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song_filters = sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter",
        )
        .fetch_all(conn)
        .await?;

        Ok(song_filters)
    }
}

#[async_trait]
impl SettingRepository<Sqlite> for SqliteDb {
    async fn set<'a, A>(&self, acquiree: A, key: &str, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let mut conn = acquiree.acquire().await?;
        let _ = sqlx::query("INSERT INTO setting (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = ($2) WHERE key = ($1);")
                    .bind(key)
                    .bind(value)
                    .execute(&mut *conn)
                    .await;
        Ok(())
    }

    async fn get<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<Setting>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let mut conn = acquiree.acquire().await?;
        Ok(sqlx::query_as::<sqlx::Sqlite, Setting>(
            "SELECT id, key, value FROM setting WHERE key = ?",
        )
        .bind(key)
        .fetch_one(&mut *conn)
        .await?)
    }
}
