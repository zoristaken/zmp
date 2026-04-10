use anyhow::Context;
use async_trait::async_trait;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    Acquire, Pool, Row, Sqlite, SqlitePool,
};
use std::str::FromStr;

use crate::{
    filter::{Filter, FilterRepository},
    manager::HasPool,
    setting::{Setting, SettingRepository},
    song::{Song, SongRepository},
    song_filter::{SongFilter, SongFilterRepository},
    song_mutation::SongMutationRepository,
    song_query::{group_rows, SongQueryRepository, SongWithFilterRow, SongWithFilters},
};

#[derive(Clone)]
pub struct SqliteDb {
    pub pool: SqlitePool,
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
            sqlx::query(
            "INSERT OR IGNORE INTO song (title, artist, release_year, album, remix, search_blob, file_path, duration, extension)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&val.title)
        .bind(&val.artist)
        .bind(val.release_year)
        .bind(&val.album)
        .bind(&val.remix)
        .bind(&val.search_blob)
        .bind(&val.file_path)
        .bind(val.duration)
        .bind(&val.extension)
        .execute(&mut *conn)
        .await
        .with_context(|| format!("failed inserting: {:?}", val))?;
        }

        Ok(())
    }

    async fn get_all_songs<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let songs = sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration, extension FROM song"
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
            .filter(|s| search.iter().all(|x| s.search_blob.contains(x)))
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
            search_blob, file_path, duration, extension FROM song WHERE ",
        );

        qb.push("(");
        for (i, word) in words.iter().enumerate() {
            qb.push("title LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" AND ");
            }
        }
        qb.push(")");

        qb.push(" OR (");
        for (i, word) in words.iter().enumerate() {
            qb.push("artist LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" AND ");
            }
        }
        qb.push(")");

        qb.push(" OR (");
        for (i, word) in words.iter().enumerate() {
            qb.push("release_year LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" AND ");
            }
        }
        qb.push(")");

        qb.push(" OR (");
        for (i, word) in words.iter().enumerate() {
            qb.push("album LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" AND ");
            }
        }
        qb.push(")");

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
            "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration, extension FROM song WHERE ",
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

    async fn get_song_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song = sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration, extension FROM song WHERE id = ?"
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
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration, extension FROM song WHERE title = ? AND artist = ?"
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

        sqlx::query("INSERT INTO filter (name) VALUES (?)")
            .bind(name)
            .execute(conn)
            .await?;

        Ok(())
    }
    async fn get_all_filters<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Filter>>
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

    async fn get_filter_by_id<'a, A>(&self, acquiree: A, filter_id: i32) -> anyhow::Result<Filter>
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

    async fn remove_filter<'a, A>(&self, acquiree: A, filter_id: i32) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let result = sqlx::query("DELETE FROM filter WHERE id = ?")
            .bind(filter_id)
            .execute(conn)
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[async_trait]
impl SongFilterRepository<Sqlite> for SqliteDb {
    async fn add<'a, A>(&self, acquiree: A, song_id: i32, filter_id: i32) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        sqlx::query(
            "INSERT INTO song_filter (song_id, filter_id)
                    VALUES (?, ?)",
        )
        .bind(song_id)
        .bind(filter_id)
        .execute(conn)
        .await?;

        Ok(())
    }

    async fn remove_song_filter<'a, A>(
        &self,
        acquiree: A,
        song_filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let result = sqlx::query("DELETE FROM song_filter WHERE id = ?")
            .bind(song_filter_id)
            .execute(conn)
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
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
            sqlx::query(
                "INSERT INTO song_filter (song_id, filter_id)
                    VALUES (?, ?)",
            )
            .bind(val.song_id)
            .bind(val.filter_id)
            .execute(&mut *conn)
            .await?;
        }

        Ok(())
    }

    async fn get_song_filter_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<SongFilter>
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

    async fn get_all_song_filters<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<SongFilter>>
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
        sqlx::query("INSERT INTO setting (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value")
                    .bind(key)
                    .bind(value)
                    .execute(&mut *conn)
                    .await?;
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

#[async_trait]
impl SongQueryRepository<Sqlite> for SqliteDb {
    async fn list_songs_with_filters<'a, A>(
        &self,
        acquiree: A,
    ) -> anyhow::Result<Vec<SongWithFilters>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let mut conn = acquiree.acquire().await?;

        let rows = sqlx::query_as::<_, crate::song_query::SongWithFilterRow>(
            r#"
        SELECT
            s.id,
            s.title,
            s.artist,
            s.release_year,
            s.album,
            s.remix,
            s.search_blob,
            s.file_path,
            s.duration,
            s.extension,
            f.id AS filter_id,
            f.name AS filter_name
        FROM song s
        LEFT JOIN song_filter sf ON sf.song_id = s.id
        LEFT JOIN filter f ON f.id = sf.filter_id
        ORDER BY s.title, f.name
        "#,
        )
        .fetch_all(&mut *conn)
        .await?;

        Ok(crate::song_query::group_rows(rows))
    }

    async fn search_by_db_with_filters<'a, A>(
        &self,
        acquiree: A,
        words: &[&str],
        max_results: i32,
    ) -> anyhow::Result<Vec<SongWithFilters>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        if words.is_empty() {
            return Ok(vec![]);
        }

        let mut conn = acquiree.acquire().await?;

        let mut query = String::from(
            r#"
            SELECT
                s.id,
                s.title,
                s.artist,
                s.release_year,
                s.album,
                s.remix,
                s.search_blob,
                s.file_path,
                s.duration,
                s.extension,
                f.id AS filter_id,
                f.name AS filter_name
            FROM song s
            LEFT JOIN song_filter sf ON sf.song_id = s.id
            LEFT JOIN filter f ON f.id = sf.filter_id
            WHERE
            "#,
        );

        for (i, _) in words.iter().enumerate() {
            if i > 0 {
                query.push_str(" AND ");
            }
            query.push_str("s.search_blob LIKE ?");
        }

        query.push_str(" ORDER BY s.title, f.name LIMIT ?");

        let mut q = sqlx::query_as::<sqlx::Sqlite, SongWithFilterRow>(&query);

        for word in words {
            q = q.bind(format!("%{}%", word));
        }

        q = q.bind(max_results);

        let rows = q.fetch_all(&mut *conn).await?;
        Ok(group_rows(rows))
    }
}

fn normalize_search_blob(parts: impl IntoIterator<Item = String>) -> String {
    parts
        .into_iter()
        .flat_map(|part| {
            part.split_whitespace()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[async_trait]
impl SongMutationRepository<Sqlite> for SqliteDb {
    async fn refresh_song_search_blob<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song_row = sqlx::query(
            r#"
            SELECT
                title,
                artist,
                release_year,
                album,
                remix
            FROM song
            WHERE id = ?
            "#,
        )
        .bind(song_id)
        .fetch_one(&mut *conn)
        .await?;

        let title: String = song_row.try_get("title")?;
        let artist: String = song_row.try_get("artist")?;
        let release_year: i32 = song_row.try_get("release_year")?;
        let album: String = song_row.try_get("album")?;
        let remix: String = song_row.try_get("remix")?;

        let filter_rows = sqlx::query(
            r#"
            SELECT f.name
            FROM song_filter sf
            INNER JOIN filter f ON f.id = sf.filter_id
            WHERE sf.song_id = ?
            ORDER BY f.name
            "#,
        )
        .bind(song_id)
        .fetch_all(&mut *conn)
        .await?;

        let mut parts = vec![title, artist, album, remix, release_year.to_string()];

        for row in filter_rows {
            let filter_name: String = row.try_get("name")?;
            parts.push(filter_name);
        }

        let search_blob = normalize_search_blob(parts);

        let result = sqlx::query(
            r#"
            UPDATE song
            SET search_blob = ?
            WHERE id = ?
            "#,
        )
        .bind(search_blob)
        .bind(song_id)
        .execute(&mut *conn)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn refresh_all_song_search_blobs<'a, A>(&self, acquiree: A) -> anyhow::Result<usize>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song_rows = sqlx::query(
            r#"
            SELECT id
            FROM song
            ORDER BY id
            "#,
        )
        .fetch_all(&mut *conn)
        .await?;

        let mut refreshed = 0usize;

        for row in song_rows {
            let song_id: i32 = row.try_get("id")?;

            let did_refresh = self.refresh_song_search_blob(&mut *conn, song_id).await?;
            if did_refresh {
                refreshed += 1;
            }
        }

        Ok(refreshed)
    }

    async fn add_filter_to_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
        filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let insert_result = sqlx::query(
            r#"
            INSERT INTO song_filter (song_id, filter_id)
            VALUES (?, ?)
            "#,
        )
        .bind(song_id)
        .bind(filter_id)
        .execute(&mut *conn)
        .await?;

        if insert_result.rows_affected() == 0 {
            return Ok(false);
        }

        self.refresh_song_search_blob(&mut *conn, song_id).await
    }

    async fn remove_filter_from_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song_filter_row = sqlx::query(
            r#"
            SELECT song_id
            FROM song_filter
            WHERE id = ?
            "#,
        )
        .bind(song_filter_id)
        .fetch_one(&mut *conn)
        .await?;

        let song_id: i32 = song_filter_row.try_get("song_id")?;

        let delete_result = sqlx::query(
            r#"
            DELETE FROM song_filter
            WHERE id = ?
            "#,
        )
        .bind(song_filter_id)
        .execute(&mut *conn)
        .await?;

        if delete_result.rows_affected() == 0 {
            return Ok(false);
        }

        self.refresh_song_search_blob(&mut *conn, song_id).await
    }
}
