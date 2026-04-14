use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use sqlx::Database;

use crate::{
    filter::{Filter, FilterRepository, FilterService},
    metadata::MetadataParser,
    song_filter::{SongFilterRepository, SongFilterService},
    song_mutation::{SongMutationRepository, SongMutationService},
};

pub struct SongMetadataFilterSyncService<R, DB>
where
    R: FilterRepository<DB> + SongFilterRepository<DB> + SongMutationRepository<DB>,
    DB: Database,
{
    filter: FilterService<R, DB>,
    song_filter: SongFilterService<R, DB>,
    song_mutation: SongMutationService<R, DB>,
    metadata_parser: MetadataParser,
}

impl<R, DB> SongMetadataFilterSyncService<R, DB>
where
    R: FilterRepository<DB> + SongFilterRepository<DB> + SongMutationRepository<DB> + Clone,
    DB: Database,
{
    pub fn new(repos: R) -> Self {
        Self {
            filter: FilterService::new(repos.clone()),
            song_filter: SongFilterService::new(repos.clone()),
            song_mutation: SongMutationService::new(repos),
            metadata_parser: MetadataParser::new(),
        }
    }

    pub fn read_song_filters_metadata(&self, file_path: &Path) -> anyhow::Result<Vec<Filter>> {
        self.metadata_parser.get_song_filters_metadata(file_path)
    }

    pub async fn load_filters_by_name(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
    ) -> anyhow::Result<HashMap<String, Filter>> {
        Ok(self
            .filter
            .get_all(&mut *tx)
            .await?
            .into_iter()
            .map(|filter| (filter.name.clone(), filter))
            .collect())
    }

    pub async fn sync_song_filters_from_metadata(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        song_id: i32,
        file_path: &Path,
    ) -> anyhow::Result<bool> {
        let metadata_filters = self.read_song_filters_metadata(file_path)?;
        let mut filters_by_name = self.load_filters_by_name(&mut *tx).await?;
        self.sync_song_filters_with_cache(tx, song_id, metadata_filters, &mut filters_by_name)
            .await
    }

    pub async fn sync_song_filters(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        song_id: i32,
        metadata_filters: Vec<Filter>,
    ) -> anyhow::Result<bool> {
        let mut filters_by_name = self.load_filters_by_name(&mut *tx).await?;
        self.sync_song_filters_with_cache(tx, song_id, metadata_filters, &mut filters_by_name)
            .await
    }

    pub async fn sync_song_filters_with_cache(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        song_id: i32,
        metadata_filters: Vec<Filter>,
        filters_by_name: &mut HashMap<String, Filter>,
    ) -> anyhow::Result<bool> {
        let desired_filter_ids = self
            .resolve_desired_filter_ids(&mut *tx, metadata_filters, filters_by_name)
            .await?;

        let existing_song_filters = self.song_filter.get_by_song(&mut *tx, song_id).await?;
        let mut existing_song_filter_ids = HashSet::new();
        let mut song_filter_ids_to_remove = Vec::new();

        for existing_song_filter in existing_song_filters {
            if desired_filter_ids.contains(&existing_song_filter.filter_id) {
                existing_song_filter_ids.insert(existing_song_filter.filter_id);
            } else {
                song_filter_ids_to_remove.push(existing_song_filter.id);
            }
        }

        let mut changed = false;

        for song_filter_id in song_filter_ids_to_remove {
            self.song_filter.remove(&mut *tx, song_filter_id).await?;
            changed = true;
        }

        for desired_filter_id in desired_filter_ids {
            if existing_song_filter_ids.insert(desired_filter_id) {
                self.song_filter
                    .add(&mut *tx, song_id, desired_filter_id)
                    .await?;
                changed = true;
            }
        }

        if changed {
            self.song_mutation
                .refresh_song_search_blob(&mut *tx, song_id)
                .await?;
        }

        Ok(changed)
    }

    async fn resolve_desired_filter_ids(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        metadata_filters: Vec<Filter>,
        filters_by_name: &mut HashMap<String, Filter>,
    ) -> anyhow::Result<HashSet<i32>> {
        let desired_names = metadata_filters
            .into_iter()
            .map(|filter| filter.name.trim().to_string())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();

        let mut desired_filter_ids = HashSet::new();
        let mut seen_names = HashSet::new();

        for desired_name in desired_names {
            if !seen_names.insert(desired_name.clone()) {
                continue;
            }

            let desired_filter_id = match filters_by_name.get(&desired_name) {
                Some(filter) => filter.id,
                None => {
                    self.filter.add(&mut *tx, &desired_name).await?;
                    let filter = self.filter.get_by_name(&mut *tx, &desired_name).await?;
                    let filter_id = filter.id;
                    filters_by_name.insert(filter.name.clone(), filter);
                    filter_id
                }
            };

            desired_filter_ids.insert(desired_filter_id);
        }

        Ok(desired_filter_ids)
    }
}
