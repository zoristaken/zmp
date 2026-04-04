use std::marker::PhantomData;

use async_trait::async_trait;
use sqlx::Acquire;

use crate::sqlite::RepositoryDb;

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Song {
    pub id: i32,
    pub title: String,
    pub artist: String,
    pub release_year: i32,
    pub album: String,
    pub remix: String,
    pub search_blob: String,
    pub file_path: String,
    pub duration: i64,
}

#[async_trait]
pub trait SongRepository<DB>
where
    DB: RepositoryDb,
{
    async fn add_all<'a, A>(&self, acquiree: A, songs: Vec<Song>) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn get_all<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn get_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn get_by_title_artist<'a, A>(
        &self,
        acquiree: A,
        title: &str,
        artist: &str,
    ) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn search_by(
        &self,
        songs: &[Song],
        search: &[&str],
        max_results: usize,
    ) -> anyhow::Result<Vec<Song>>;

    async fn search_by_db<'a, A>(
        &self,
        acquiree: A,
        words: &[&str],
        max_results: i32,
    ) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn search_by_db_alternative<'a, A>(
        &self,
        acquiree: A,
        words: &[&str],
        max_results: i32,
    ) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = DB> + Send;
}

pub struct SongService<R, DB>
where
    R: SongRepository<DB>,
    DB: RepositoryDb,
{
    repo: R,
    _db: std::marker::PhantomData<DB>,
}

impl<R, DB> SongService<R, DB>
where
    R: SongRepository<DB>,
    DB: RepositoryDb,
{
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            _db: PhantomData,
        }
    }

    pub async fn add_songs<'a, A>(&self, acquireee: A, songs: Vec<Song>) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.add_all(acquireee, songs).await
    }

    pub async fn list_songs<'a, A>(&self, acquireee: A) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.get_all(acquireee).await
    }

    pub async fn get_by_id<'a, A>(&self, acquireee: A, id: i32) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.get_by_id(acquireee, id).await
    }

    pub async fn get_by_title_artist<'a, A>(
        &self,
        acquireee: A,
        title: &str,
        artist: &str,
    ) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo
            .get_by_title_artist(acquireee, title, artist)
            .await
    }

    pub async fn search_by(
        &self,
        songs: &[Song],
        search: &[&str],
        max_results: usize,
    ) -> anyhow::Result<Vec<Song>> {
        self.repo.search_by(songs, search, max_results).await
    }

    pub async fn search_by_db<'a, A>(
        &self,
        acquireee: A,
        words: &[&str],
        max_results: i32,
    ) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        if words.len() == 0 {
            return Ok(vec![]);
        }

        self.repo.search_by_db(acquireee, words, max_results).await
    }
}
