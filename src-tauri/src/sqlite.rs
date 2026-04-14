use std::str::FromStr;

use anyhow::Context;
use async_trait::async_trait;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    Acquire, QueryBuilder, Row, Sqlite, SqlitePool,
};

use crate::{
    filter::{Filter, FilterRepository},
    search_blob::build_search_blob,
    setting::{Setting, SettingRepository},
    song::{Song, SongRepository},
    song_filter::{SongFilter, SongFilterRepository},
    song_mutation::SongMutationRepository,
    song_query::{group_song_rows, SongQueryRepository, SongWithFilterRow, SongWithFilters},
};

const SONG_SORT_ORDER: &str = "s.title, s.artist, s.album, s.release_year, s.id";
const SQLITE_BIND_LIMIT: usize = 32766;
const SONG_INSERT_FIELDS_PER_ROW: usize = 11;
const SONG_INSERT_BATCH_SIZE: usize = SQLITE_BIND_LIMIT / SONG_INSERT_FIELDS_PER_ROW;

fn push_song_insert_values<'a>(query: &mut QueryBuilder<'a, Sqlite>, songs: &'a [Song]) {
    query.push_values(songs, |mut row, song| {
        row.push_bind(&song.title)
            .push_bind(&song.artist)
            .push_bind(song.release_year)
            .push_bind(&song.album)
            .push_bind(&song.remix)
            .push_bind(&song.search_blob)
            .push_bind(&song.file_path)
            .push_bind(song.duration)
            .push_bind(&song.extension)
            .push_bind(song.file_size)
            .push_bind(song.file_modified_millis);
    });
}

pub async fn new(path: &str) -> anyhow::Result<SqlitePool> {
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

    Ok(pool)
}

#[derive(Clone)]
pub struct SqliteImpl {}

#[async_trait]
impl SongRepository<Sqlite> for SqliteImpl {
    async fn add_song<'a, A>(&self, acquiree: A, song: Song) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let saved_song = sqlx::query_as::<sqlx::Sqlite, Song>(
            "INSERT INTO song (title, artist, release_year, album, remix, search_blob, file_path, duration, extension, file_size, file_modified_millis)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                RETURNING id, title, artist, release_year, album, remix, search_blob, file_path, duration, extension, file_size, file_modified_millis",
        )
        .bind(&song.title)
        .bind(&song.artist)
        .bind(song.release_year)
        .bind(&song.album)
        .bind(&song.remix)
        .bind(&song.search_blob)
        .bind(&song.file_path)
        .bind(song.duration)
        .bind(&song.extension)
        .bind(song.file_size)
        .bind(song.file_modified_millis)
        .fetch_one(&mut *conn)
        .await
        .with_context(|| format!("failed inserting song: {:?}", song))?;

        Ok(saved_song)
    }

    async fn update_song<'a, A>(&self, acquiree: A, song: Song) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let result = sqlx::query(
            "UPDATE song
                SET title = ?, artist = ?, release_year = ?, album = ?, remix = ?, search_blob = ?, file_path = ?, duration = ?, extension = ?, file_size = ?, file_modified_millis = ?
                WHERE id = ?",
        )
        .bind(&song.title)
        .bind(&song.artist)
        .bind(song.release_year)
        .bind(&song.album)
        .bind(&song.remix)
        .bind(&song.search_blob)
        .bind(&song.file_path)
        .bind(song.duration)
        .bind(&song.extension)
        .bind(song.file_size)
        .bind(song.file_modified_millis)
        .bind(song.id)
        .execute(&mut *conn)
        .await
        .with_context(|| format!("failed updating song: {:?}", song))?;

        Ok(result.rows_affected() > 0)
    }

    async fn remove_song<'a, A>(&self, acquiree: A, song_id: i32) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let result = sqlx::query("DELETE FROM song WHERE id = ?")
            .bind(song_id)
            .execute(&mut *conn)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn replace_all<'a, A>(&self, acquiree: A, songs: Vec<Song>) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        sqlx::query("DELETE FROM song").execute(&mut *conn).await?;

        for chunk in songs.chunks(SONG_INSERT_BATCH_SIZE) {
            let mut query = QueryBuilder::<Sqlite>::new(
                "INSERT INTO song (title, artist, release_year, album, remix, search_blob, file_path, duration, extension, file_size, file_modified_millis) ",
            );
            push_song_insert_values(&mut query, chunk);

            query.build().execute(&mut *conn).await.with_context(|| {
                format!("failed inserting replacement song batch of {}", chunk.len())
            })?;
        }

        Ok(())
    }

    async fn add_all<'a, A>(&self, acquiree: A, songs: Vec<Song>) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        for chunk in songs.chunks(SONG_INSERT_BATCH_SIZE) {
            let mut query = QueryBuilder::<Sqlite>::new(
                "INSERT INTO song (title, artist, release_year, album, remix, search_blob, file_path, duration, extension, file_size, file_modified_millis) ",
            );
            push_song_insert_values(&mut query, chunk);

            query
                .build()
                .execute(&mut *conn)
                .await
                .with_context(|| format!("failed inserting song batch of {}", chunk.len()))?;
        }

        Ok(())
    }

    async fn get_all_songs<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let songs = sqlx::query_as::<sqlx::Sqlite, Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration, extension, file_size, file_modified_millis FROM song ORDER BY id"
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
        let mut songs: Vec<Song> = songs
            .iter()
            .filter(|s| search.iter().all(|x| s.search_blob.contains(x)))
            .cloned()
            .collect();

        songs.sort_by(|a, b| {
            (&a.title, &a.artist, &a.album, a.release_year, a.id).cmp(&(
                &b.title,
                &b.artist,
                &b.album,
                b.release_year,
                b.id,
            ))
        });

        songs.truncate(max_results);

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
            "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration, extension, file_size, file_modified_millis FROM song WHERE ",
        );
        //TODO (zor): after using the app, verify if searching by substrings actually happens
        //I have a feeling we can improve the performance by moving to a fts5 prefix search
        //without any side effects at all
        for (i, word) in words.iter().enumerate() {
            qb.push("search_blob LIKE ");
            qb.push_bind(format!("%{}%", word));
            if i != words.len() - 1 {
                qb.push(" AND ");
            }
        }

        qb.push(" ORDER BY title, artist, album, release_year, id LIMIT ");
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
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration, extension, file_size, file_modified_millis FROM song WHERE id = ?"
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
        "SELECT id, title, artist, release_year, album, remix, search_blob, file_path, duration, extension, file_size, file_modified_millis FROM song WHERE title = ? AND artist = ?"
            )
            .bind(title)
            .bind(artist)
            .fetch_one(conn)
            .await?;

        Ok(song)
    }
}

#[async_trait]
impl FilterRepository<Sqlite> for SqliteImpl {
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
        let filter =
            sqlx::query_as::<sqlx::Sqlite, Filter>("SELECT id, name FROM filter ORDER BY id")
                .fetch_all(conn)
                .await?;

        Ok(filter)
    }

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
            Ok(result) => Ok(result.rows_affected() > 0),
            Err(_) => Ok(false),
        }
    }
}

#[async_trait]
impl SongFilterRepository<Sqlite> for SqliteImpl {
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
            Ok(result) => Ok(result.rows_affected() > 0),
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
            "SELECT id, song_id, filter_id FROM song_filter WHERE filter_id = ? ORDER BY id",
        )
        .bind(filter_id)
        .fetch_all(conn)
        .await?;

        Ok(song_filter)
    }

    async fn get_by_song<'a, A>(&self, acquiree: A, song_id: i32) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        let conn = &mut *acquiree.acquire().await?;

        let song_filter = sqlx::query_as::<sqlx::Sqlite, SongFilter>(
            "SELECT id, song_id, filter_id FROM song_filter WHERE song_id = ? ORDER BY id",
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
            "SELECT id, song_id, filter_id FROM song_filter ORDER BY id",
        )
        .fetch_all(conn)
        .await?;

        Ok(song_filters)
    }
}

#[async_trait]
impl SettingRepository<Sqlite> for SqliteImpl {
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
        Ok(
            sqlx::query_as::<sqlx::Sqlite, Setting>("SELECT key, value FROM setting WHERE key = ?")
                .bind(key)
                .fetch_one(&mut *conn)
                .await?,
        )
    }

    async fn get_many<'a, A>(&self, acquiree: A, keys: &[&str]) -> anyhow::Result<Vec<Setting>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = acquiree.acquire().await?;
        let mut query =
            QueryBuilder::<Sqlite>::new("SELECT key, value FROM setting WHERE key IN (");
        let mut separated = query.separated(", ");

        for key in keys {
            separated.push_bind(key);
        }

        query.push(")");

        Ok(query
            .build_query_as::<Setting>()
            .fetch_all(&mut *conn)
            .await?)
    }
}

#[async_trait]
impl SongQueryRepository<Sqlite> for SqliteImpl {
    async fn search_by_db_with_filters<'a, A>(
        &self,
        acquiree: A,
        words: &[&str],
        max_results: i32,
        pinned_song_id: Option<i32>,
    ) -> anyhow::Result<Vec<SongWithFilters>>
    where
        A: Acquire<'a, Database = Sqlite> + Send,
    {
        fn push_search_blob_conditions(
            query: &mut QueryBuilder<'_, Sqlite>,
            alias: &str,
            words: &[&str],
        ) {
            for (i, word) in words.iter().enumerate() {
                if i > 0 {
                    query.push(" AND ");
                }

                query
                    .push(alias)
                    .push(".search_blob LIKE ")
                    .push_bind(format!("%{word}%"));
            }
        }

        if max_results <= 0 {
            return Ok(vec![]);
        }

        let mut conn = acquiree.acquire().await?;
        let mut query = QueryBuilder::<Sqlite>::new(
            r#"
            WITH limited_songs AS (
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
                    s.file_size,
                    s.file_modified_millis
                FROM song s
            "#,
        );

        if !words.is_empty() {
            query.push(" WHERE ");
            push_search_blob_conditions(&mut query, "s", words);
        }

        query
            .push(" ORDER BY ")
            .push(SONG_SORT_ORDER)
            .push(
                r#"
                LIMIT
            "#,
            )
            .push_bind(max_results)
            .push(
                r#"
            ),
            selected_songs AS (
                SELECT * FROM limited_songs
            "#,
            );

        if let Some(pinned_song_id) = pinned_song_id {
            query.push(
                r#"
                UNION ALL
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
                    s.file_size,
                    s.file_modified_millis
                FROM song s
                WHERE s.id =
                "#,
            );
            query.push_bind(pinned_song_id);

            if !words.is_empty() {
                query.push(" AND ");
                push_search_blob_conditions(&mut query, "s", words);
            }

            query.push(
                r#"
                AND NOT EXISTS (
                    SELECT 1
                    FROM limited_songs limited
                    WHERE limited.id = s.id
                )
                "#,
            );
        }

        query.push(
            r#"
            )
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
                s.file_size,
                s.file_modified_millis,
                f.id AS filter_id,
                f.name AS filter_name
            FROM selected_songs s
            LEFT JOIN song_filter sf ON sf.song_id = s.id
            LEFT JOIN filter f ON f.id = sf.filter_id
            ORDER BY
            "#,
        );
        query.push(SONG_SORT_ORDER);

        let rows = query
            .build_query_as::<SongWithFilterRow>()
            .fetch_all(&mut *conn)
            .await?;
        Ok(group_song_rows(rows))
    }
}

#[async_trait]
impl SongMutationRepository<Sqlite> for SqliteImpl {
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

        let search_blob = build_search_blob(parts);

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
