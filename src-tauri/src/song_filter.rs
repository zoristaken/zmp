use async_trait::async_trait;
use sqlx::{Acquire, Database};

use crate::manager::HasPool;

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone, PartialEq)]
pub struct SongFilter {
    pub id: i32,
    pub song_id: i32,
    pub filter_id: i32,
}

#[async_trait]
pub trait SongFilterRepository<DB>
where
    DB: Database,
{
    async fn add<'a, A>(&self, acquiree: A, song_filter: SongFilter) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn add_multiple<'a, A>(
        &self,
        acquiree: A,
        song_filters: Vec<SongFilter>,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get_all_song_filters<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get_song_filter_by_id<'a, A>(
        &self,
        acquiree: A,
        id: i32,
    ) -> anyhow::Result<SongFilter>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get_by_filter<'a, A>(
        &self,
        acquiree: A,
        filter_id: i32,
    ) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get_by_song<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
    ) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB> + Send;
}

pub struct SongFilterService<R, DB>
where
    R: SongFilterRepository<DB>,
    DB: Database,
{
    pub pool: sqlx::Pool<DB>,
    repo: R,
}

impl<R, DB> SongFilterService<R, DB>
where
    R: SongFilterRepository<DB> + HasPool<DB>,
    DB: Database,
{
    pub fn new(repo: R) -> Self {
        Self {
            pool: repo.pool().clone(),
            repo,
        }
    }

    pub async fn add<'a, A>(&self, acquiree: A, song_filter: SongFilter) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.add(acquiree, song_filter).await
    }

    pub async fn add_multiple<'a, A>(
        &self,
        acquiree: A,
        song_filters: Vec<SongFilter>,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.add_multiple(acquiree, song_filters).await
    }

    pub async fn get_all<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.get_all_song_filters(acquiree).await
    }

    pub async fn get_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<SongFilter>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.get_song_filter_by_id(acquiree, id).await
    }

    pub async fn get_by_filter<'a, A>(
        &self,
        acquiree: A,
        filter_id: i32,
    ) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.get_by_filter(acquiree, filter_id).await
    }

    pub async fn get_by_song<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
    ) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.get_by_song(acquiree, song_id).await
    }
}
