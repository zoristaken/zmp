use std::{path::Path, sync::Mutex};

use sqlx::{Database, Pool, Sqlite};

use crate::{
    filter::{Filter, FilterRepository, FilterService},
    metadata::MetadataParser,
    player::{Player, QueueSyncResult},
    setting::{SettingRepository, SettingService},
    song::{Song, SongRepository, SongService},
    song_filter::{SongFilter, SongFilterRepository, SongFilterService},
    song_mutation::{SongMutationRepository, SongMutationService},
    song_query::{SongQueryRepository, SongQueryService, SongWithFilters},
    sqlite::SqliteDb,
};

pub struct AppState {
    pub zmp: PlayerManager<SqliteDb, Sqlite>,
}

pub trait HasPool<DB: Database> {
    fn pool(&self) -> &Pool<DB>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackChange {
    pub current_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SongListChange {
    pub count: usize,
    pub current_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueMutation {
    pub changed: bool,
    pub current_index: Option<usize>,
}

pub struct PlayerManager<R, DB>
where
    DB: Database,
    R: SettingRepository<DB>
        + SongRepository<DB>
        + FilterRepository<DB>
        + SongFilterRepository<DB>
        + SongQueryRepository<DB>
        + SongMutationRepository<DB>
        + HasPool<DB>,
{
    pub setting: SettingService<R, DB>,
    pub song: SongService<R, DB>,
    pub song_query: SongQueryService<R, DB>,
    pub song_mutation: SongMutationService<R, DB>,
    pub song_filter: SongFilterService<R, DB>,
    pub filter: FilterService<R, DB>,
    pub metadata_parser: MetadataParser,
    pub player: Mutex<Player>,
    pub pool: sqlx::Pool<DB>,
}

async fn replace_library_and_reset_state<R, DB>(
    song_service: &SongService<R, DB>,
    setting_service: &SettingService<R, DB>,
    songs: Vec<Song>,
) -> anyhow::Result<()>
where
    DB: Database,
    R: SongRepository<DB> + SettingRepository<DB> + HasPool<DB>,
{
    let mut tx = song_service.pool.begin().await?;
    song_service.replace_songs(&mut tx, songs).await?;
    setting_service.reset_library_state(&mut tx).await?;
    tx.commit().await?;

    Ok(())
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
        + HasPool<DB>
        + Clone,
{
    pub async fn new(repos: R) -> Self {
        let setting = SettingService::new(repos.clone());
        let index = setting.get_saved_index(&setting.pool).await;
        let shuffle = setting.is_random_play(&setting.pool).await;
        let repeat = setting.is_repeat_flag(&setting.pool).await;
        let volume = setting.get_saved_volume_value(&setting.pool).await;

        Self {
            setting,
            song: SongService::new(repos.clone()),
            song_query: SongQueryService::new(repos.clone()),
            song_mutation: SongMutationService::new(repos.clone()),
            song_filter: SongFilterService::new(repos.clone()),
            filter: FilterService::new(repos.clone()),
            metadata_parser: MetadataParser::new(),
            player: Mutex::new(Player::new(Some(index), shuffle, repeat, volume)),
            pool: repos.pool().clone(),
        }
    }

    pub async fn process_music_folder(&self) -> anyhow::Result<()> {
        let folder_path = self.setting.get_music_folder_path(&self.pool).await?;
        let songs = self
            .metadata_parser
            .parse_song_metadata(Path::new(&folder_path))?;

        replace_library_and_reset_state(&self.song, &self.setting, songs).await
    }

    fn current_song_id(&self) -> anyhow::Result<Option<i32>> {
        let player = self
            .player
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        player.current_song_id()
    }

    async fn load_song_list_in_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        pinned_song_id: Option<i32>,
    ) -> anyhow::Result<Vec<SongWithFilters>> {
        let saved_search = self
            .setting
            .get_saved_search_blob(&mut *tx)
            .await
            .unwrap_or_default();
        let song_list_limit = self.setting.get_song_list_limit(&mut *tx).await;

        self.song_query
            .query_song_list(&mut *tx, &saved_search, song_list_limit, pinned_song_id)
            .await
    }

    fn lock_player(&self) -> anyhow::Result<std::sync::MutexGuard<'_, Player>> {
        self.player
            .lock()
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    fn sync_song_list(&self, songs: Vec<SongWithFilters>) -> anyhow::Result<QueueSyncResult> {
        let mut player = self.lock_player()?;
        player.set_queue(songs)
    }

    async fn persist_queue_sync(&self, sync: QueueSyncResult) -> anyhow::Result<Option<usize>> {
        self.setting
            .persist_queue_sync(sync.current_index, sync.cleared_current_song)
            .await?;

        Ok(sync.current_index)
    }

    pub async fn load(&self) -> anyhow::Result<SongListChange> {
        let mut tx = self.pool.begin().await?;
        let pinned_song_id = self.current_song_id()?;

        let songs = self.load_song_list_in_tx(&mut tx, pinned_song_id).await?;
        let is_shuffle = self.setting.is_random_play(&mut tx).await;
        let is_repeat = self.setting.is_repeat_flag(&mut tx).await;
        let saved_index = self.setting.get_saved_index(&mut tx).await;
        let saved_seek = self.setting.get_current_song_seek(&mut tx).await;
        let saved_play_pause_flag = self.setting.is_playing(&mut tx).await;
        let count = songs.len();

        tx.commit().await?;

        let current_index = {
            let mut player = self.lock_player()?;
            player.load_saved_state(
                is_shuffle,
                is_repeat,
                saved_index,
                saved_seek,
                saved_play_pause_flag,
                songs,
            )?
        };

        self.setting
            .set_saved_index(&self.pool, current_index.unwrap_or(0))
            .await?;

        Ok(SongListChange {
            count,
            current_index,
        })
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
        let current_index = {
            let mut player = self.lock_player()?;
            player.play_song_at(index, true, true)?;
            player.current_index()
        };

        self.setting.persist_started_track(current_index).await?;

        Ok(PlaybackChange { current_index })
    }

    pub async fn query_song_list(&self, query: &str) -> anyhow::Result<Vec<SongWithFilters>> {
        let pinned_song_id = self.current_song_id()?;
        let song_list_limit = self.setting.get_song_list_limit(&self.pool).await;

        self.song_query
            .query_song_list(&self.pool, query, song_list_limit, pinned_song_id)
            .await
    }

    pub async fn search_songs(&self, query: &str) -> anyhow::Result<SongListChange> {
        let trimmed = query.trim();

        self.setting
            .set_saved_search_blob(&self.pool, trimmed)
            .await?;

        let songs = self.query_song_list(trimmed).await?;
        let count = songs.len();
        let sync = self.sync_song_list(songs)?;
        let current_index = self.persist_queue_sync(sync).await?;

        Ok(SongListChange {
            count,
            current_index,
        })
    }

    pub async fn preview_search_songs(&self, query: &str) -> anyhow::Result<Vec<SongWithFilters>> {
        self.query_song_list(query).await
    }

    pub async fn next_song(&self) -> anyhow::Result<PlaybackChange> {
        let current_index = {
            let mut player = self.lock_player()?;
            player.next_song()?;
            player.current_index()
        };

        self.setting.persist_started_track(current_index).await?;

        Ok(PlaybackChange { current_index })
    }

    pub async fn previous_song(&self) -> anyhow::Result<PlaybackChange> {
        let current_index = {
            let mut player = self.lock_player()?;
            player.previous()?;
            player.current_index()
        };

        self.setting.persist_started_track(current_index).await?;

        Ok(PlaybackChange { current_index })
    }

    pub async fn get_current_song_seek(&self) -> usize {
        self.setting.get_current_song_seek(&self.pool).await
    }

    pub async fn save_current_song_seek(&self, seek_value: usize) -> anyhow::Result<()> {
        self.setting
            .set_current_song_seek(&self.pool, seek_value)
            .await
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

    pub async fn get_volume(&self) -> rodio::Float {
        self.setting.get_saved_volume_value(&self.pool).await
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

        let removed = self.filter.remove(&mut tx, filter_id).await?;

        if removed {
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

        let saved = self
            .song_mutation
            .add_filter_to_song_and_reindex(&mut tx, song_id, filter_id)
            .await?;

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

        let removed = self
            .song_mutation
            .remove_filter_from_song_and_reindex(&mut tx, song_filter_id)
            .await?;

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

        Ok(QueueMutation {
            changed: removed,
            current_index,
        })
    }

    //Generates the getter and setter method for the PlayerManager, using the SetttingService
    keybind_manager_methods!(
        (get_focus_search_keybind, set_focus_search_keybind),
        (get_settings_keybind, set_settings_keybind),
        (get_mute_keybind, set_mute_keybind),
        (get_shuffle_keybind, set_shuffle_keybind),
        (get_repeat_keybind, set_repeat_keybind),
        (get_next_keybind, set_next_keybind),
        (get_previous_keybind, set_previous_keybind),
        (get_play_pause_keybind, set_play_pause_keybind),
    );
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use crate::{
        filter::FilterService, setting::SettingService, song::SongService,
        song_filter::SongFilterService, sqlite::SqliteDb,
    };

    use super::replace_library_and_reset_state;

    fn song(title: &str, artist: &str, path: &str) -> crate::song::Song {
        crate::song::Song {
            id: 0,
            title: title.to_string(),
            artist: artist.to_string(),
            release_year: 1998,
            album: "Album".to_string(),
            remix: String::new(),
            search_blob: format!(
                "{} {} album 1998",
                title.to_lowercase(),
                artist.to_lowercase()
            ),
            file_path: path.to_string(),
            duration: 180,
            extension: "flac".to_string(),
        }
    }

    async fn setup_db() -> SqliteDb {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        SqliteDb { pool }
    }

    #[tokio::test]
    async fn replace_library_and_reset_state_replaces_songs_and_resets_playback_settings() {
        let sqlite = setup_db().await;
        let song_service = SongService::new(sqlite.clone());
        let setting_service = SettingService::new(sqlite.clone());

        song_service
            .add_songs(
                &song_service.pool,
                vec![song("Teardrop", "Massive Attack", "/music/teardrop.flac")],
            )
            .await
            .unwrap();

        setting_service
            .set_saved_search_blob(&setting_service.pool, "massive attack")
            .await
            .unwrap();
        setting_service
            .set_saved_index(&setting_service.pool, 7)
            .await
            .unwrap();
        setting_service
            .set_current_song_seek(&setting_service.pool, 123)
            .await
            .unwrap();
        setting_service
            .set_play_pause_flag(&setting_service.pool, true)
            .await
            .unwrap();
        setting_service
            .set_processed_music_folder(&setting_service.pool, false)
            .await
            .unwrap();

        replace_library_and_reset_state(
            &song_service,
            &setting_service,
            vec![song("Archangel", "Burial", "/music/archangel.flac")],
        )
        .await
        .unwrap();

        let songs = song_service.list_songs(&song_service.pool).await.unwrap();

        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].title, "Archangel");
        assert_eq!(songs[0].artist, "Burial");
        assert_eq!(
            setting_service
                .get_saved_search_blob(&setting_service.pool)
                .await
                .unwrap(),
            ""
        );
        assert_eq!(
            setting_service.get_saved_index(&setting_service.pool).await,
            0
        );
        assert_eq!(
            setting_service
                .get_current_song_seek(&setting_service.pool)
                .await,
            0
        );
        assert!(!setting_service.is_playing(&setting_service.pool).await);
        assert!(
            setting_service
                .has_processed_music_folder(&setting_service.pool)
                .await
        );
    }

    #[tokio::test]
    async fn replace_library_and_reset_state_clears_song_filter_links_but_keeps_filters() {
        let sqlite = setup_db().await;
        let song_service = SongService::new(sqlite.clone());
        let setting_service = SettingService::new(sqlite.clone());
        let filter_service = FilterService::new(sqlite.clone());
        let song_filter_service = SongFilterService::new(sqlite.clone());

        song_service
            .add_songs(
                &song_service.pool,
                vec![song("Teardrop", "Massive Attack", "/music/teardrop.flac")],
            )
            .await
            .unwrap();

        filter_service
            .add(&filter_service.pool, "ambient")
            .await
            .unwrap();
        song_filter_service
            .add(&song_filter_service.pool, 1, 1)
            .await
            .unwrap();

        replace_library_and_reset_state(
            &song_service,
            &setting_service,
            vec![song("Archangel", "Burial", "/music/archangel.flac")],
        )
        .await
        .unwrap();

        let filters = filter_service.get_all(&filter_service.pool).await.unwrap();
        let song_filters = song_filter_service
            .get_all(&song_filter_service.pool)
            .await
            .unwrap();

        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].name, "ambient");
        assert!(song_filters.is_empty());
    }
}
