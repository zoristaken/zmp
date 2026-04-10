use std::marker::PhantomData;

use async_trait::async_trait;
use sqlx::{Acquire, Database};

#[async_trait]
pub trait SongMutationRepository<DB>
where
    DB: Database,
{
    async fn refresh_song_search_blob<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn add_filter_to_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
        filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn remove_filter_from_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn refresh_all_song_search_blobs<'a, A>(&self, acquiree: A) -> anyhow::Result<usize>
    where
        A: Acquire<'a, Database = DB> + Send;
}

pub struct SongMutationService<R, DB>
where
    R: SongMutationRepository<DB>,
    DB: Database,
{
    _db: PhantomData<DB>,
    repo: R,
}

impl<R, DB> SongMutationService<R, DB>
where
    R: SongMutationRepository<DB>,
    DB: Database,
{
    pub fn new(repo: R) -> Self {
        Self {
            _db: PhantomData::default(),
            repo,
        }
    }

    pub async fn refresh_song_search_blob<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.refresh_song_search_blob(acquiree, song_id).await
    }

    pub async fn add_filter_to_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
        filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo
            .add_filter_to_song_and_reindex(acquiree, song_id, filter_id)
            .await
    }

    pub async fn remove_filter_from_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo
            .remove_filter_from_song_and_reindex(acquiree, song_filter_id)
            .await
    }

    pub async fn refresh_all_song_search_blobs<'a, A>(&self, acquiree: A) -> anyhow::Result<usize>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.refresh_all_song_search_blobs(acquiree).await
    }
}
