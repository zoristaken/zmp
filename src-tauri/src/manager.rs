use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Mutex,
};

use serde::Serialize;
use sqlx::{Database, Pool};

use crate::{
    filter::{Filter, FilterRepository, FilterService},
    metadata::MetadataParser,
    player::{Player, QueueSyncResult},
    setting::{AppSettingsSnapshot, SettingRepository, SettingService},
    song::{Song, SongRepository, SongService},
    song_filter::{SongFilter, SongFilterRepository, SongFilterService},
    song_metadata_filter_sync::SongMetadataFilterSyncService,
    song_mutation::{SongMutationRepository, SongMutationService},
    song_query::{SongQueryRepository, SongQueryService, SongWithFilters},
};

//TODO (zor): current fails in playback change are not persisted in the database.
//Should be a somewhat simple fix by adding a playable flag to the song table and
//making the backend sync the flag state before sending to the frontend

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackChange {
    pub current_index: Option<usize>,
    pub failed_song_ids: Vec<i32>,
    pub should_emit_track_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongListChange {
    pub count: usize,
    pub current_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerLoadState {
    pub count: usize,
    pub current_index: Option<usize>,
    pub volume: rodio::Float,
    pub shuffle: bool,
    pub repeat: bool,
    pub failed_song_ids: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueMutation {
    pub changed: bool,
    pub current_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
struct PreviewSearch {
    query: String,
    songs: Vec<SongWithFilters>,
}

pub struct PlayerManager<R, DB>
where
    DB: Database,
    R: SettingRepository<DB>
        + SongRepository<DB>
        + FilterRepository<DB>
        + SongFilterRepository<DB>
        + SongQueryRepository<DB>
        + SongMutationRepository<DB>,
{
    pub setting: SettingService<R, DB>,
    pub song: SongService<R, DB>,
    pub song_query: SongQueryService<R, DB>,
    pub song_mutation: SongMutationService<R, DB>,
    pub song_filter: SongFilterService<R, DB>,
    pub filter: FilterService<R, DB>,
    song_metadata_filter_sync: SongMetadataFilterSyncService<R, DB>,
    pub metadata_parser: MetadataParser,
    pub player: Mutex<Player>,
    preview_search: Mutex<Option<PreviewSearch>>,
    pub pool: sqlx::Pool<DB>,
}

macro_rules! keybind_manager_methods {
    ($(($getter:ident, $setter:ident)),+ $(,)?) => {
        $(
            pub async fn $getter(&self) -> Option<String> {
                self.setting.$getter(&self.pool).await.ok()
            }

            pub async fn $setter(&self, keybind: &str) -> anyhow::Result<()> {
                self.setting.$setter(&self.pool, keybind).await
            }
        )+
    };
}

impl<R, DB> PlayerManager<R, DB>
where
    DB: Database,
    R: SettingRepository<DB>
        + SongRepository<DB>
        + FilterRepository<DB>
        + SongFilterRepository<DB>
        + SongQueryRepository<DB>
        + SongMutationRepository<DB>
        + Clone,
{
    pub async fn new(pool: Pool<DB>, repos: R) -> Self {
        Self {
            setting: SettingService::new(repos.clone()),
            song: SongService::new(repos.clone()),
            song_query: SongQueryService::new(repos.clone()),
            song_mutation: SongMutationService::new(repos.clone()),
            song_filter: SongFilterService::new(repos.clone()),
            filter: FilterService::new(repos.clone()),
            song_metadata_filter_sync: SongMetadataFilterSyncService::new(repos.clone()),
            metadata_parser: MetadataParser::new(),
            player: Mutex::new(Player::new()),
            preview_search: Mutex::new(None),
            pool,
        }
    }

    pub async fn replace_library_and_reset_state(&self, songs: Vec<Song>) -> anyhow::Result<()> {
        self.clear_preview_search()?;
        let mut tx = self.pool.begin().await?;
        self.song.replace_songs(&mut tx, songs).await?;
        self.setting.reset_library_state(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn process_music_folder(&self) -> anyhow::Result<()> {
        self.flush_deferred_song_filters_metadata().await?;

        let pending_song_metadata_sync_paths = self
            .setting
            .get_pending_song_metadata_sync_paths(&self.pool)
            .await;
        let pending_song_db_filters = self
            .collect_pending_song_db_filters(&pending_song_metadata_sync_paths)
            .await?;

        let folder_path = self.setting.get_music_folder_path(&self.pool).await?;
        let songs = self
            .metadata_parser
            .parse_song_metadata(Path::new(&folder_path))?;

        let song_metadata_filters =
            self.collect_song_metadata_filters(&songs, &pending_song_db_filters)?;

        self.replace_library_with_metadata_filters_and_reset_state(songs, song_metadata_filters)
            .await
    }

    async fn collect_pending_song_db_filters(
        &self,
        pending_song_metadata_sync_paths: &[String],
    ) -> anyhow::Result<HashMap<String, Vec<Filter>>> {
        if pending_song_metadata_sync_paths.is_empty() {
            return Ok(HashMap::new());
        }

        let pending_song_metadata_sync_paths = pending_song_metadata_sync_paths
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();

        let mut tx = self.pool.begin().await?;
        let songs = self.song.list_songs(&mut tx).await?;
        let mut filters_by_path = HashMap::new();

        for song in songs {
            if !pending_song_metadata_sync_paths.contains(song.file_path.as_str()) {
                continue;
            }

            filters_by_path.insert(
                song.file_path,
                self.load_song_filters_in_tx(&mut tx, song.id).await?,
            );
        }

        tx.commit().await?;

        Ok(filters_by_path)
    }

    fn collect_song_metadata_filters(
        &self,
        songs: &[Song],
        pending_song_db_filters: &HashMap<String, Vec<Filter>>,
    ) -> anyhow::Result<Vec<(String, Vec<Filter>)>> {
        songs
            .iter()
            .map(|song| {
                Ok((
                    song.file_path.clone(),
                    pending_song_db_filters
                        .get(&song.file_path)
                        .cloned()
                        .unwrap_or(
                            self.song_metadata_filter_sync
                                .read_song_filters_metadata(Path::new(&song.file_path))?,
                        ),
                ))
            })
            .collect()
    }

    async fn replace_library_with_metadata_filters_and_reset_state(
        &self,
        songs: Vec<Song>,
        song_metadata_filters: Vec<(String, Vec<Filter>)>,
    ) -> anyhow::Result<()> {
        self.clear_preview_search()?;
        let mut tx = self.pool.begin().await?;

        self.song.replace_songs(&mut tx, songs).await?;
        self.import_song_metadata_filters(&mut tx, song_metadata_filters)
            .await?;
        self.setting.reset_library_state(&mut tx).await?;

        tx.commit().await?;
        Ok(())
    }

    async fn import_song_metadata_filters(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        song_metadata_filters: Vec<(String, Vec<Filter>)>,
    ) -> anyhow::Result<()> {
        if song_metadata_filters.is_empty() {
            return Ok(());
        }

        let songs = self.song.list_songs(&mut *tx).await?;
        let mut song_ids_by_path = HashMap::new();
        for song in songs {
            song_ids_by_path.insert(song.file_path, song.id);
        }
        let mut filters_by_name = self
            .song_metadata_filter_sync
            .load_filters_by_name(&mut *tx)
            .await?;

        for (file_path, metadata_filters) in song_metadata_filters {
            let song_id = *song_ids_by_path.get(&file_path).ok_or_else(|| {
                anyhow::anyhow!("Imported song not found for file path {}", file_path)
            })?;

            self.song_metadata_filter_sync
                .sync_song_filters_with_cache(
                    &mut *tx,
                    song_id,
                    metadata_filters,
                    &mut filters_by_name,
                )
                .await?;
        }

        Ok(())
    }

    async fn sync_song_filters_metadata(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        song_id: i32,
    ) -> anyhow::Result<()> {
        let song = self.song.get_song_by_id(&mut *tx, song_id).await?;
        let filters = self.load_song_filters_in_tx(&mut *tx, song_id).await?;
        self.metadata_parser
            .add_song_filters_metadata(Path::new(&song.file_path), filters)?;

        Ok(())
    }

    async fn load_song_filters_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        song_id: i32,
    ) -> anyhow::Result<Vec<Filter>> {
        let song_filters = self.song_filter.get_by_song(&mut *tx, song_id).await?;
        let mut filters = Vec::with_capacity(song_filters.len());

        for song_filter in song_filters {
            filters.push(
                self.filter
                    .get_by_id(&mut *tx, song_filter.filter_id)
                    .await?,
            );
        }

        Ok(filters)
    }

    async fn sync_song_filters_metadata_by_file_paths(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        file_paths: Vec<String>,
    ) -> anyhow::Result<()> {
        if file_paths.is_empty() {
            return Ok(());
        }

        let songs = self.song.list_songs(&mut *tx).await?;
        let song_ids_by_path = songs
            .into_iter()
            .map(|song| (song.file_path, song.id))
            .collect::<HashMap<_, _>>();

        for file_path in file_paths {
            if let Some(song_id) = song_ids_by_path.get(&file_path).copied() {
                self.sync_song_filters_metadata(&mut *tx, song_id).await?;
            }
        }

        Ok(())
    }

    async fn defer_song_filters_metadata_sync_paths(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        file_paths: Vec<String>,
    ) -> anyhow::Result<()> {
        if file_paths.is_empty() {
            return Ok(());
        }

        let mut pending_file_paths = self
            .setting
            .get_pending_song_metadata_sync_paths(&mut *tx)
            .await;
        pending_file_paths.extend(file_paths);
        self.setting
            .set_pending_song_metadata_sync_paths(&mut *tx, &pending_file_paths)
            .await
    }

    fn partition_song_metadata_sync_paths(
        file_paths: Vec<String>,
        current_song_file_path: Option<&str>,
    ) -> (Vec<String>, Vec<String>) {
        let mut ready_to_sync = Vec::new();
        let mut deferred = Vec::new();
        let mut seen = HashSet::new();

        for file_path in file_paths {
            if !seen.insert(file_path.clone()) {
                continue;
            }

            if current_song_file_path == Some(file_path.as_str()) {
                deferred.push(file_path);
            } else {
                ready_to_sync.push(file_path);
            }
        }

        (ready_to_sync, deferred)
    }

    async fn flush_deferred_song_filters_metadata(&self) -> anyhow::Result<()> {
        let current_song_file_path = self.current_song_file_path()?;
        let mut tx = self.pool.begin().await?;
        let pending_file_paths = self
            .setting
            .get_pending_song_metadata_sync_paths(&mut tx)
            .await;
        let (ready_to_sync, deferred) = Self::partition_song_metadata_sync_paths(
            pending_file_paths,
            current_song_file_path.as_deref(),
        );

        if ready_to_sync.is_empty() {
            return Ok(());
        }

        self.sync_song_filters_metadata_by_file_paths(&mut tx, ready_to_sync)
            .await?;
        self.setting
            .set_pending_song_metadata_sync_paths(&mut tx, &deferred)
            .await?;
        tx.commit().await?;

        Ok(())
    }

    fn current_song_file_path(&self) -> anyhow::Result<Option<String>> {
        let player = self
            .player
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        Ok(player
            .current_song()
            .map(|song| song.song.file_path.clone()))
    }

    async fn reload_song_list(&self) -> anyhow::Result<SongListChange> {
        let mut tx = self.pool.begin().await?;
        let pinned_song_id = self.current_song_id()?;
        let songs = self.load_song_list_in_tx(&mut tx, pinned_song_id).await?;
        let count = songs.len();

        tx.commit().await?;

        let sync = self.sync_song_list(songs)?;
        let current_index = self.persist_queue_sync(sync).await?;

        Ok(SongListChange {
            count,
            current_index,
        })
    }

    pub async fn reload_song_list_after_library_change(&self) -> anyhow::Result<SongListChange> {
        self.clear_preview_search()?;
        self.reload_song_list().await
    }

    fn current_song_id(&self) -> anyhow::Result<Option<i32>> {
        let player = self
            .player
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        player.current_song_id()
    }

    fn cache_preview_search(&self, query: &str, songs: Vec<SongWithFilters>) -> anyhow::Result<()> {
        let mut preview_search = self
            .preview_search
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        *preview_search = Some(PreviewSearch {
            query: query.to_string(),
            songs,
        });

        Ok(())
    }

    fn take_cached_preview_search(
        &self,
        query: &str,
    ) -> anyhow::Result<Option<Vec<SongWithFilters>>> {
        let mut preview_search = self
            .preview_search
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        let matches = preview_search
            .as_ref()
            .map(|preview| preview.query == query)
            .unwrap_or(false);

        if matches {
            Ok(preview_search.take().map(|preview| preview.songs))
        } else {
            Ok(None)
        }
    }

    fn clear_preview_search(&self) -> anyhow::Result<()> {
        let mut preview_search = self
            .preview_search
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        *preview_search = None;
        Ok(())
    }

    async fn load_song_list_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        pinned_song_id: Option<i32>,
    ) -> anyhow::Result<Vec<SongWithFilters>> {
        let query_settings = self.setting.get_song_list_query_settings(&mut *tx).await?;

        self.song_query
            .query_song_list(
                &mut *tx,
                &query_settings.saved_search_blob,
                query_settings.song_list_limit,
                pinned_song_id,
            )
            .await
    }

    fn lock_player(&self) -> anyhow::Result<std::sync::MutexGuard<'_, Player>> {
        self.player
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    fn current_player_seek_seconds(&self) -> anyhow::Result<usize> {
        let player = self.lock_player()?;

        Ok(player.seek_pos().as_secs() as usize)
    }

    fn sync_song_list(&self, songs: Vec<SongWithFilters>) -> anyhow::Result<QueueSyncResult> {
        let mut player = self.lock_player()?;
        player.set_queue(songs)
    }

    async fn persist_queue_sync(&self, sync: QueueSyncResult) -> anyhow::Result<Option<usize>> {
        self.setting
            .persist_queue_sync(&self.pool, sync.current_index, sync.cleared_current_song)
            .await?;

        Ok(sync.current_index)
    }

    pub async fn load(&self) -> anyhow::Result<PlayerLoadState> {
        self.flush_deferred_song_filters_metadata().await?;

        let mut tx = self.pool.begin().await?;
        let pinned_song_id = self.current_song_id()?;

        let songs = self.load_song_list_in_tx(&mut tx, pinned_song_id).await?;
        let player_settings = self.setting.get_player_settings_snapshot(&mut tx).await?;
        let count = songs.len();

        tx.commit().await?;

        let (current_index, playback) = {
            let mut player = self.lock_player()?;
            player.set_volume(player_settings.volume);
            let playback = player.load_saved_state(
                player_settings.is_random,
                player_settings.is_repeat,
                player_settings.saved_index,
                player_settings.saved_seek,
                player_settings.is_playing && !player_settings.always_start_paused,
                songs,
            )?;
            (player.current_index(), playback)
        };

        if !playback.failed_song_ids.is_empty() && current_index.is_none() {
            self.setting.persist_started_track(&self.pool, None).await?;
        } else {
            self.setting
                .set_saved_index(&self.pool, current_index.unwrap_or(0))
                .await?;
        }
        self.flush_deferred_song_filters_metadata().await?;

        Ok(PlayerLoadState {
            count,
            current_index,
            volume: player_settings.volume,
            shuffle: player_settings.is_random,
            repeat: player_settings.is_repeat,
            failed_song_ids: playback.failed_song_ids,
        })
    }

    pub async fn get_app_settings_snapshot(&self) -> anyhow::Result<AppSettingsSnapshot> {
        self.setting.get_app_settings_snapshot(&self.pool).await
    }

    pub async fn get_music_folder_path(&self) -> String {
        self.setting
            .get_music_folder_path(&self.pool)
            .await
            .unwrap_or_default()
    }

    pub async fn set_music_folder_path(&self, path: &str) -> anyhow::Result<()> {
        self.setting.set_music_folder_path(&self.pool, path).await
    }

    pub async fn has_processed_music_folder(&self) -> bool {
        self.setting.has_processed_music_folder(&self.pool).await
    }

    pub async fn set_processed_music_folder(&self, flag: bool) -> anyhow::Result<()> {
        self.setting
            .set_processed_music_folder(&self.pool, flag)
            .await
    }

    pub async fn get_saved_search_blob(&self) -> String {
        self.setting
            .get_saved_search_blob(&self.pool)
            .await
            .unwrap_or_default()
    }

    pub async fn get_song_list_limit(&self) -> i32 {
        self.setting.get_song_list_limit(&self.pool).await
    }

    pub async fn set_song_list_limit(&self, limit: i32) -> anyhow::Result<()> {
        self.setting.set_song_list_limit(&self.pool, limit).await
    }

    pub fn get_current_index(&self) -> anyhow::Result<Option<usize>> {
        Ok(self.lock_player()?.current_index())
    }

    pub fn get_current_song(&self) -> anyhow::Result<Option<SongWithFilters>> {
        Ok(self.lock_player()?.current_song().cloned())
    }

    pub fn get_loaded_songs(&self) -> anyhow::Result<Vec<SongWithFilters>> {
        Ok(self.lock_player()?.queue().to_vec())
    }

    pub fn get_is_player_paused(&self) -> anyhow::Result<bool> {
        Ok(self.lock_player()?.is_paused())
    }

    pub async fn play_song_at(&self, index: usize) -> anyhow::Result<PlaybackChange> {
        let result = {
            let mut player = self.lock_player()?;
            let playback = player.play_song_at(index, true, true)?;

            PlaybackChange {
                current_index: player.current_index(),
                failed_song_ids: playback.failed_song_ids,
                should_emit_track_changed: playback.should_emit_track_changed,
            }
        };

        if result.should_emit_track_changed {
            self.setting
                .persist_started_track(&self.pool, result.current_index)
                .await?;
        } else if result.current_index.is_none() {
            self.setting.persist_started_track(&self.pool, None).await?;
        }
        self.flush_deferred_song_filters_metadata().await?;

        Ok(result)
    }

    pub async fn query_song_list(&self, query: &str) -> anyhow::Result<Vec<SongWithFilters>> {
        let pinned_song_id = self.current_song_id()?;
        let song_list_limit = self.setting.get_song_list_limit(&self.pool).await;

        self.song_query
            .query_song_list(&self.pool, query, song_list_limit, pinned_song_id)
            .await
    }

    pub async fn commit_preview_search(&self, query: &str) -> anyhow::Result<SongListChange> {
        let trimmed = query.trim();

        self.setting
            .set_saved_search_blob(&self.pool, trimmed)
            .await?;

        let songs = self
            .take_cached_preview_search(trimmed)?
            .ok_or_else(|| anyhow::anyhow!("no cached preview search for query: {}", trimmed))?;
        let count = songs.len();
        let sync = self.sync_song_list(songs)?;
        let current_index = self.persist_queue_sync(sync).await?;
        self.flush_deferred_song_filters_metadata().await?;

        Ok(SongListChange {
            count,
            current_index,
        })
    }

    pub async fn preview_search_songs(&self, query: &str) -> anyhow::Result<Vec<SongWithFilters>> {
        let trimmed = query.trim();
        let songs = self.query_song_list(trimmed).await?;
        self.cache_preview_search(trimmed, songs.clone())?;
        Ok(songs)
    }

    pub async fn next_song(&self) -> anyhow::Result<PlaybackChange> {
        let result = {
            let mut player = self.lock_player()?;
            let playback = player.next_song()?;

            PlaybackChange {
                current_index: player.current_index(),
                failed_song_ids: playback.failed_song_ids,
                should_emit_track_changed: playback.should_emit_track_changed,
            }
        };

        if result.should_emit_track_changed {
            self.setting
                .persist_started_track(&self.pool, result.current_index)
                .await?;
        } else if result.current_index.is_none() {
            self.setting.persist_started_track(&self.pool, None).await?;
        }
        self.flush_deferred_song_filters_metadata().await?;

        Ok(result)
    }

    pub async fn previous_song(&self) -> anyhow::Result<PlaybackChange> {
        let result = {
            let mut player = self.lock_player()?;
            let playback = player.previous_song()?;

            PlaybackChange {
                current_index: player.current_index(),
                failed_song_ids: playback.failed_song_ids,
                should_emit_track_changed: playback.should_emit_track_changed,
            }
        };

        if result.should_emit_track_changed {
            self.setting
                .persist_started_track(&self.pool, result.current_index)
                .await?;
        } else if result.current_index.is_none() {
            self.setting.persist_started_track(&self.pool, None).await?;
        }
        self.flush_deferred_song_filters_metadata().await?;

        Ok(result)
    }

    pub async fn get_current_song_seek(&self) -> usize {
        match self.current_player_seek_seconds() {
            Ok(seek_value) => seek_value,
            Err(_) => self.setting.get_current_song_seek(&self.pool).await,
        }
    }

    pub async fn save_current_song_seek(&self, seek_value: usize) -> anyhow::Result<usize> {
        let seek_value = self.current_player_seek_seconds().unwrap_or(seek_value);

        self.setting
            .set_current_song_seek(&self.pool, seek_value)
            .await?;

        Ok(seek_value)
    }

    pub async fn increase_current_song_seek_by_seconds(&self, seconds: u64) -> anyhow::Result<()> {
        let seek_value = { self.lock_player()?.seek_pos() };
        let new_seek_value = seek_value.as_secs().saturating_add(seconds);
        self.set_current_song_seek(new_seek_value as usize).await
    }

    pub async fn decrease_current_song_seek_by_seconds(&self, seconds: u64) -> anyhow::Result<()> {
        let seek_value = { self.lock_player()?.seek_pos() };
        let new_seek_value = seek_value.as_secs().saturating_sub(seconds);
        self.set_current_song_seek(new_seek_value as usize).await
    }

    pub async fn set_current_song_seek(&self, seek_value: usize) -> anyhow::Result<()> {
        self.setting
            .set_current_song_seek(&self.pool, seek_value)
            .await?;

        self.lock_player()?.seek_to_seconds(seek_value as u64)
    }

    pub async fn get_play_pause(&self) -> bool {
        self.setting.is_playing(&self.pool).await
    }

    pub async fn set_play_pause(&self, is_playing: bool) -> anyhow::Result<()> {
        self.setting
            .set_play_pause_flag(&self.pool, is_playing)
            .await?;

        self.lock_player()?.play_pause(is_playing);

        Ok(())
    }

    pub async fn get_always_start_paused(&self) -> bool {
        self.setting.should_always_start_paused(&self.pool).await
    }

    pub async fn set_always_start_paused(&self, flag: bool) -> anyhow::Result<()> {
        self.setting.set_always_start_paused(&self.pool, flag).await
    }

    pub async fn get_volume(&self) -> rodio::Float {
        self.setting.get_saved_volume_value(&self.pool).await
    }

    pub async fn increase_volume_by(&self, value: rodio::Float) -> anyhow::Result<()> {
        let current_volume = { self.lock_player()?.get_volume() };

        let new_volume = current_volume + value;

        self.set_volume(new_volume).await
    }

    pub async fn decrease_volume_by(&self, value: rodio::Float) -> anyhow::Result<()> {
        let current_volume = { self.lock_player()?.get_volume() };

        let new_volume = current_volume - value;

        self.set_volume(new_volume).await
    }

    pub async fn set_volume(&self, volume: rodio::Float) -> anyhow::Result<()> {
        self.setting
            .set_saved_volume_value(&self.pool, volume)
            .await?;

        self.lock_player()?.set_volume(volume);

        Ok(())
    }

    pub async fn get_repeat(&self) -> bool {
        self.setting.is_repeat_flag(&self.pool).await
    }

    pub async fn set_repeat(&self) -> anyhow::Result<()> {
        let is_repeat = !self.setting.is_repeat_flag(&self.pool).await;

        self.setting.set_repeat_flag(&self.pool, is_repeat).await?;
        self.lock_player()?.set_repeat(is_repeat);

        Ok(())
    }

    pub async fn get_random(&self) -> bool {
        self.setting.is_random_play(&self.pool).await
    }

    pub async fn set_random(&self) -> anyhow::Result<()> {
        let is_random = !self.setting.is_random_play(&self.pool).await;

        self.setting.set_random_play(&self.pool, is_random).await?;
        self.lock_player()?.set_shuffle(is_random);

        Ok(())
    }

    pub async fn create_filter(&self, filter_name: &str) -> bool {
        self.filter.add(&self.pool, filter_name).await.is_ok()
    }

    pub async fn remove_filter(&self, filter_id: i32) -> anyhow::Result<QueueMutation> {
        let mut tx = self.pool.begin().await?;
        let pinned_song_id = self.current_song_id()?;
        let current_song_file_path = self.current_song_file_path()?;
        let affected_song_ids = self
            .song_filter
            .get_by_filter(&mut tx, filter_id)
            .await?
            .into_iter()
            .map(|song_filter| song_filter.song_id)
            .collect::<HashSet<_>>();
        let affected_song_file_paths = self
            .song
            .list_songs(&mut tx)
            .await?
            .into_iter()
            .filter(|song| affected_song_ids.contains(&song.id))
            .map(|song| song.file_path)
            .collect::<Vec<_>>();

        let removed = self.filter.remove(&mut tx, filter_id).await?;

        if removed {
            let (ready_to_sync, deferred) = Self::partition_song_metadata_sync_paths(
                affected_song_file_paths,
                current_song_file_path.as_deref(),
            );

            self.sync_song_filters_metadata_by_file_paths(&mut tx, ready_to_sync)
                .await?;
            self.defer_song_filters_metadata_sync_paths(&mut tx, deferred)
                .await?;

            //TODO (zor): optimization - it should only refresh the song blobs of songs that had the
            //filter
            self.song_mutation
                .refresh_all_song_search_blobs(&mut tx)
                .await?;
        }

        let songs = if removed {
            Some(self.load_song_list_in_tx(&mut tx, pinned_song_id).await?)
        } else {
            None
        };

        tx.commit().await?;

        let current_index = if let Some(songs) = songs {
            let sync = self.sync_song_list(songs)?;
            self.persist_queue_sync(sync).await?
        } else {
            None
        };
        self.flush_deferred_song_filters_metadata().await?;

        Ok(QueueMutation {
            changed: removed,
            current_index,
        })
    }

    pub async fn get_filters(&self) -> anyhow::Result<Vec<Filter>> {
        self.filter.get_all(&self.pool).await
    }

    pub async fn get_filters_for_song(&self, song_id: i32) -> anyhow::Result<Vec<SongFilter>> {
        self.song_filter.get_by_song(&self.pool, song_id).await
    }

    pub async fn add_filter_to_song(
        &self,
        song_id: i32,
        filter_id: i32,
    ) -> anyhow::Result<QueueMutation> {
        let mut tx = self.pool.begin().await?;
        let pinned_song_id = self.current_song_id()?;
        let current_song_file_path = self.current_song_file_path()?;

        let saved = self
            .song_mutation
            .add_filter_to_song_and_reindex(&mut tx, song_id, filter_id)
            .await?;

        if saved {
            let song_file_path = self.song.get_song_by_id(&mut tx, song_id).await?.file_path;
            let (ready_to_sync, deferred) = Self::partition_song_metadata_sync_paths(
                vec![song_file_path],
                current_song_file_path.as_deref(),
            );

            self.sync_song_filters_metadata_by_file_paths(&mut tx, ready_to_sync)
                .await?;
            self.defer_song_filters_metadata_sync_paths(&mut tx, deferred)
                .await?;
        }

        let songs = if saved {
            Some(self.load_song_list_in_tx(&mut tx, pinned_song_id).await?)
        } else {
            None
        };

        tx.commit().await?;

        let current_index = if let Some(songs) = songs {
            let sync = self.sync_song_list(songs)?;
            self.persist_queue_sync(sync).await?
        } else {
            None
        };
        self.flush_deferred_song_filters_metadata().await?;

        Ok(QueueMutation {
            changed: saved,
            current_index,
        })
    }

    pub async fn remove_filter_from_song(
        &self,
        song_filter_id: i32,
    ) -> anyhow::Result<QueueMutation> {
        let mut tx = self.pool.begin().await?;
        let pinned_song_id = self.current_song_id()?;
        let current_song_file_path = self.current_song_file_path()?;
        let song_id = self
            .song_filter
            .get_by_id(&mut tx, song_filter_id)
            .await?
            .song_id;
        let song_file_path = self.song.get_song_by_id(&mut tx, song_id).await?.file_path;

        let removed = self
            .song_mutation
            .remove_filter_from_song_and_reindex(&mut tx, song_filter_id)
            .await?;

        if removed {
            let (ready_to_sync, deferred) = Self::partition_song_metadata_sync_paths(
                vec![song_file_path],
                current_song_file_path.as_deref(),
            );

            self.sync_song_filters_metadata_by_file_paths(&mut tx, ready_to_sync)
                .await?;
            self.defer_song_filters_metadata_sync_paths(&mut tx, deferred)
                .await?;
        }

        let songs = if removed {
            Some(self.load_song_list_in_tx(&mut tx, pinned_song_id).await?)
        } else {
            None
        };

        tx.commit().await?;

        let current_index = if let Some(songs) = songs {
            let sync = self.sync_song_list(songs)?;
            self.persist_queue_sync(sync).await?
        } else {
            None
        };
        self.flush_deferred_song_filters_metadata().await?;

        Ok(QueueMutation {
            changed: removed,
            current_index,
        })
    }

    //Generates the getter and setter keybind methods for the PlayerManager, using the SetttingService which
    //fetches and sets in the database the key value pair corresponding to each keybind
    keybind_manager_methods!(
        (get_focus_search_keybind, set_focus_search_keybind),
        (get_settings_keybind, set_settings_keybind),
        (get_mute_keybind, set_mute_keybind),
        (get_shuffle_keybind, set_shuffle_keybind),
        (get_repeat_keybind, set_repeat_keybind),
        (get_next_keybind, set_next_keybind),
        (get_previous_keybind, set_previous_keybind),
        (get_play_pause_keybind, set_play_pause_keybind),
        (get_increase_volume_keybind, set_increase_volume_keybind),
        (get_decrease_volume_keybind, set_decrease_volume_keybind),
        (get_seek_forward_keybind, set_seek_forward_keybind),
        (get_seek_backward_keybind, set_seek_backward_keybind),
        (get_filter_menu_keybind, set_filter_menu_keybind),
        (get_song_filter_menu_keybind, set_song_filter_menu_keybind),
        (get_keybind_settings_keybind, set_keybind_settings_keybind),
        (
            get_switch_song_filter_pane_keybind,
            set_switch_song_filter_pane_keybind
        ),
        (
            get_apply_selected_filter_keybind,
            set_apply_selected_filter_keybind
        ),
    );
}
