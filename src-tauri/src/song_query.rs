use std::{collections::HashMap, marker::PhantomData};

use async_trait::async_trait;
use serde::Serialize;
use sqlx::{Acquire, Database, FromRow};

use crate::{filter::Filter, song::Song};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SongWithFilters {
    pub song: Song,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub(crate) struct SongWithFilterRow {
    id: i32,
    title: String,
    artist: String,
    release_year: i32,
    album: String,
    remix: String,
    search_blob: String,
    file_path: String,
    duration: i64,
    extension: String,
    file_size: i64,
    file_modified_millis: i64,
    filter_id: Option<i32>,
    filter_name: Option<String>,
}

pub(crate) fn group_song_rows(rows: Vec<SongWithFilterRow>) -> Vec<SongWithFilters> {
    let mut grouped = Vec::new();
    let mut positions = HashMap::new();

    for row in rows {
        let filter = match (row.filter_id, row.filter_name) {
            (Some(id), Some(name)) => Some(Filter { id, name }),
            _ => None,
        };

        let position = match positions.get(&row.id).copied() {
            Some(position) => position,
            None => {
                let position = grouped.len();

                let song = Song {
                    id: row.id,
                    title: row.title,
                    artist: row.artist,
                    release_year: row.release_year,
                    album: row.album,
                    remix: row.remix,
                    search_blob: row.search_blob,
                    file_path: row.file_path,
                    duration: row.duration,
                    extension: row.extension,
                    file_size: row.file_size,
                    file_modified_millis: row.file_modified_millis,
                };

                grouped.push(SongWithFilters {
                    song,
                    filters: Vec::new(),
                });

                positions.insert(row.id, position);
                position
            }
        };

        if let Some(filter) = filter {
            grouped[position].filters.push(filter);
        }
    }

    for song in &mut grouped {
        song.filters
            .sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    }

    grouped
}

#[async_trait]
pub trait SongQueryRepository<DB>
where
    DB: Database,
{
    async fn search_by_db_with_filters<'a, A>(
        &self,
        acquiree: A,
        words: &[&str],
        max_results: i32,
        pinned_song_id: Option<i32>,
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
    R: SongQueryRepository<DB>,
    DB: Database,
{
    pub fn new(repo: R) -> Self {
        Self {
            _db: PhantomData,
            repo,
        }
    }

    pub async fn query_song_list<'a, A>(
        &self,
        acquiree: A,
        query: &str,
        max_results: i32,
        pinned_song_id: Option<i32>,
    ) -> anyhow::Result<Vec<SongWithFilters>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let words: Vec<&str> = query.split_whitespace().collect();
        self.repo
            .search_by_db_with_filters(acquiree, &words, max_results, pinned_song_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::{group_song_rows, SongWithFilterRow};

    fn row(
        id: i32,
        title: &str,
        filter_id: Option<i32>,
        filter_name: Option<&str>,
    ) -> SongWithFilterRow {
        SongWithFilterRow {
            id,
            title: title.to_string(),
            artist: "artist".to_string(),
            release_year: 1999,
            album: "album".to_string(),
            remix: String::new(),
            search_blob: title.to_lowercase(),
            file_path: format!("/music/{id}.mp3"),
            duration: 180,
            extension: "mp3".to_string(),
            file_size: 4_096,
            file_modified_millis: 1_700_000_000_000,
            filter_id,
            filter_name: filter_name.map(str::to_string),
        }
    }

    #[test]
    fn group_song_rows_preserves_input_song_order() {
        let rows = vec![
            row(2, "Alpha", Some(1), Some("ambient")),
            row(1, "Bravo", Some(2), Some("favorites")),
            row(3, "Charlie", None, None),
        ];

        let grouped = group_song_rows(rows);

        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped[0].song.id, 2);
        assert_eq!(grouped[1].song.id, 1);
        assert_eq!(grouped[2].song.id, 3);
    }

    #[test]
    fn group_song_rows_keeps_multiple_filters_on_the_same_song() {
        let rows = vec![
            row(7, "Teardrop", Some(1), Some("ambient")),
            row(7, "Teardrop", Some(2), Some("favorites")),
        ];

        let grouped = group_song_rows(rows);

        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].filters.len(), 2);
        assert_eq!(grouped[0].filters[0].name, "ambient");
        assert_eq!(grouped[0].filters[1].name, "favorites");
    }
}
