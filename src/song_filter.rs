use std::marker::PhantomData;

use sqlx::Acquire;

use crate::sqlite::RepositoryDb;

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct SongFilter {
    pub id: i32,
    pub song_id: i32,
    pub filter_id: i32,
}

pub trait SongFilterRepository<DB>
where
    DB: RepositoryDb,
{
    async fn add<'a, A>(&self, acquiree: A, song_filter: SongFilter) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>;
    async fn add_multiple<'a, A>(
        &self,
        acquiree: A,
        song_filters: Vec<SongFilter>,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>;
    async fn get_all<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB>;
    async fn get_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<SongFilter>
    where
        A: Acquire<'a, Database = DB>;
    async fn get_by_filter<'a, A>(
        &self,
        acquiree: A,
        filter_id: i32,
    ) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB>;
    async fn get_by_song<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
    ) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB>;
}

pub struct SongFilterService<R, DB>
where
    R: SongFilterRepository<DB>,
    DB: RepositoryDb,
{
    repo: R,
    _db: std::marker::PhantomData<DB>,
}

impl<R, DB> SongFilterService<R, DB>
where
    R: SongFilterRepository<DB>,
    DB: RepositoryDb,
{
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            _db: PhantomData,
        }
    }

    pub async fn add<'a, A>(&self, acquiree: A, song_filter: SongFilter) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.repo.add(acquiree, song_filter).await
    }

    pub async fn add_multiple<'a, A>(
        &self,
        acquiree: A,
        song_filters: Vec<SongFilter>,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.repo.add_multiple(acquiree, song_filters).await
    }

    pub async fn get_all<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.repo.get_all(acquiree).await
    }

    pub async fn get_by_id<'a, A>(&self, acquiree: A, id: i32) -> anyhow::Result<SongFilter>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.repo.get_by_id(acquiree, id).await
    }

    pub async fn get_by_filter<'a, A>(
        &self,
        acquiree: A,
        filter_id: i32,
    ) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.repo.get_by_filter(acquiree, filter_id).await
    }

    pub async fn get_by_song<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
    ) -> anyhow::Result<Vec<SongFilter>>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.repo.get_by_song(acquiree, song_id).await
    }
}
