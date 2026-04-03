use std::marker::PhantomData;

use async_trait::async_trait;
use serde::Serialize;
use sqlx::Acquire;

use crate::sqlite::RepositoryDb;

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
pub struct Filter {
    pub id: i32,
    pub name: String,
}

#[async_trait]
pub trait FilterRepository<DB> {
    async fn add<'a, A>(&self, acquiree: A, name: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get_all<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Filter>>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get_by_name<'a, A>(&self, acquiree: A, name: &str) -> anyhow::Result<Filter>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get_by_id<'a, A>(&self, acquiree: A, filter_id: i32) -> anyhow::Result<Filter>
    where
        A: Acquire<'a, Database = DB> + Send;
}

pub struct FilterService<R, DB>
where
    R: FilterRepository<DB>,
    DB: RepositoryDb,
{
    repo: R,
    _db: std::marker::PhantomData<DB>,
}

impl<R, DB> FilterService<R, DB>
where
    R: FilterRepository<DB>,
    DB: RepositoryDb,
{
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            _db: PhantomData,
        }
    }

    pub async fn add<'a, A>(&self, acquiree: A, name: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.add(acquiree, name).await?;

        Ok(())
    }

    pub async fn get_all<'a, A>(&self, acquiree: A) -> anyhow::Result<Vec<Filter>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        Ok(self.repo.get_all(acquiree).await?)
    }

    pub async fn get_by_name<'a, A>(&self, acquiree: A, name: &str) -> anyhow::Result<Filter>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        Ok(self.repo.get_by_name(acquiree, name).await?)
    }

    pub async fn get_by_id<'a, A>(&self, acquiree: A, filter_id: i32) -> anyhow::Result<Filter>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        Ok(self.repo.get_by_id(acquiree, filter_id).await?)
    }
}
