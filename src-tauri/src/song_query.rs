use std::{collections::HashMap, marker::PhantomData};

use async_trait::async_trait;
use serde::Serialize;
use sqlx::{Acquire, Database, FromRow};

use crate::{filter::Filter, manager::HasPool, song::Song};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SongWithFilters {
    pub song: Song,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct SongWithFilterRow {
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
    pub filter_id: Option<i32>,
    pub filter_name: Option<String>,
}

pub fn group_rows(rows: Vec<SongWithFilterRow>) -> Vec<SongWithFilters> {
    let mut grouped = Vec::new();
    let mut positions = HashMap::new();

    for row in rows {
        let SongWithFilterRow {
            id,
            title,
            artist,
            release_year,
            album,
            remix,
            search_blob,
            file_path,
            duration,
            extension,
            filter_id,
            filter_name,
        } = row;

        let position = *positions.entry(id).or_insert_with(|| {
            let position = grouped.len();
            grouped.push(SongWithFilters {
                song: Song {
                    id,
                    title,
                    artist,
                    release_year,
                    album,
                    remix,
                    search_blob,
                    file_path,
                    duration,
                    extension,
                },
                filters: Vec::new(),
            });
            position
        });

        if let (Some(filter_id), Some(filter_name)) = (filter_id, filter_name) {
            grouped[position].filters.push(Filter {
                id: filter_id,
                name: filter_name,
            });
        }
    }

    grouped
}

#[async_trait]
pub trait SongQueryRepository<DB>
where
    DB: Database,
{
    async fn list_songs_with_filters<'a, A>(
        &self,
        acquiree: A,
    ) -> anyhow::Result<Vec<SongWithFilters>>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn search_by_db_with_filters<'a, A>(
        &self,
        acquiree: A,
        words: &[&str],
        max_results: i32,
    ) -> anyhow::Result<Vec<SongWithFilters>>
    where
        A: Acquire<'a, Database = DB> + Send;
}

pub struct SongQueryService<R, DB>
where
    R: SongQueryRepository<DB>,
    DB: Database,
{
    _db: PhantomData<DB>,
    repo: R,
}

impl<R, DB> SongQueryService<R, DB>
where
    R: SongQueryRepository<DB> + HasPool<DB>,
    DB: Database,
{
    pub fn new(repo: R) -> Self {
        Self {
            _db: PhantomData::default(),
            repo,
        }
    }

    pub async fn list_songs_with_filters<'a, A>(
        &self,
        acquiree: A,
    ) -> anyhow::Result<Vec<SongWithFilters>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.list_songs_with_filters(acquiree).await
    }

    pub async fn search_by_db_with_filters<'a, A>(
        &self,
        acquiree: A,
        words: &[&str],
        max_results: i32,
    ) -> anyhow::Result<Vec<SongWithFilters>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        if words.is_empty() {
            return Ok(vec![]);
        }

        self.repo
            .search_by_db_with_filters(acquiree, words, max_results)
            .await
    }
}
