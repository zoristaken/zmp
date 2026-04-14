use std::marker::PhantomData;

use async_trait::async_trait;
use serde::Serialize;
use sqlx::{Acquire, Database};

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone, PartialEq, Serialize)]
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
    pub extension: String,
    pub file_size: i64,
    pub file_modified_millis: i64,
}

#[async_trait]
pub trait SongRepository<DB>
where
    DB: Database,
{
    async fn add_song<'a, A>(&self, acquiree: A, song: Song) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn update_song<'a, A>(&self, acquiree: A, song: Song) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn remove_song<'a, A>(&self, acquiree: A, song_id: i32) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn replace_all<'a, A>(&self, acquiree: A, songs: Vec<Song>) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn add_all<'a, A>(&self, acquiree: A, songs: Vec<Song>) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn get_all_songs<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn get_song_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<Song>
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
}

pub struct SongService<R, DB>
where
    R: SongRepository<DB>,
    DB: Database,
{
    _db: PhantomData<DB>,
    repo: R,
}

impl<R, DB> SongService<R, DB>
where
    R: SongRepository<DB>,
    DB: Database,
{
    pub fn new(repo: R) -> Self {
        Self {
            _db: PhantomData,
            repo,
        }
    }

    pub async fn add_songs<'a, A>(&self, acquiree: A, songs: Vec<Song>) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.add_all(acquiree, songs).await
    }

    pub async fn replace_songs<'a, A>(&self, acquiree: A, songs: Vec<Song>) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.replace_all(acquiree, songs).await
    }

    pub async fn add_song<'a, A>(&self, acquiree: A, song: Song) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.add_song(acquiree, song).await
    }

    pub async fn update_song<'a, A>(&self, acquiree: A, song: Song) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.update_song(acquiree, song).await
    }

    pub async fn remove_song<'a, A>(&self, acquiree: A, song_id: i32) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.remove_song(acquiree, song_id).await
    }

    pub async fn list_songs<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.get_all_songs(acquiree).await
    }

    pub async fn get_song_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.get_song_by_id(acquiree, id).await
    }

    pub async fn get_by_title_artist<'a, A>(
        &self,
        acquiree: A,
        title: &str,
        artist: &str,
    ) -> anyhow::Result<Song>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.get_by_title_artist(acquiree, title, artist).await
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
        acquiree: A,
        words: &[&str],
        max_results: i32,
    ) -> anyhow::Result<Vec<Song>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        if words.is_empty() {
            return Ok(vec![]);
        }

        self.repo.search_by_db(acquiree, words, max_results).await
    }
}
