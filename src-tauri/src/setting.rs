use std::{collections::HashMap, marker::PhantomData};

use anyhow::Context;
use async_trait::async_trait;
use serde::Serialize;
use sqlx::{Acquire, Database};

const MUSIC_FOLDER_PATH_KEY: &str = "music_folder_path";
const PROCESSED_MUSIC_FLAG: &str = "processed_music_flag";
const VOLUME_VALUE: &str = "volume_value";
const SEARCH_BLOB: &str = "search_blob";
const SONG_LIST_LIMIT: &str = "song_list_limit";
const REPEAT_FLAG: &str = "repeat_flag";
const RANDOM_PLAY_FLAG: &str = "random_play_flag";
const PLAY_PAUSE_FLAG: &str = "play_pause_flag";
const ALWAYS_START_PAUSED_FLAG: &str = "always_start_paused_flag";
const PLAY_PAUSE_KEYBIND: &str = "play_pause_kb";
const PREVIOUS_KEYBIND: &str = "previous_kb";
const NEXT_KEYBIND: &str = "next_kb";
const REPEAT_KEYBIND: &str = "repeat_kb";
const SHUFFLE_KEYBIND: &str = "shuffle_kb";
const MUTE_KEYBIND: &str = "mute_kb";
const FOCUS_SEARCH_KEYBIND: &str = "focus_search_kb";
const SETTINGS_KEYBIND: &str = "settings_kb";
const INCREASE_VOLUME_KEYBIND: &str = "increase_volume_kb";
const DECREASE_VOLUME_KEYBIND: &str = "decrease_volume_kb";
const SEEK_FORWARD_KEYBIND: &str = "seek_forward_kb";
const SEEK_BACKWARD_KEYBIND: &str = "seek_backward_kb";
const FILTER_MENU_KEYBIND: &str = "filter_menu_kb";
const SONG_FILTER_MENU_KEYBIND: &str = "song_filter_menu_kb";
const KEYBIND_SETTINGS_KEYBIND: &str = "keybind_settings_kb";
const SWITCH_SONG_FILTER_PANE_KEYBIND: &str = "switch_song_filter_pane_kb";
const APPLY_SELECTED_FILTER_KEYBIND: &str = "apply_selected_filter_kb";
const INDEX_VALUE: &str = "index_value";
const CURRENT_SEEK_VALUE: &str = "current_seek_value";
const PENDING_SONG_METADATA_SYNC_PATHS: &str = "pending_song_metadata_sync_paths";
pub const DEFAULT_VOLUME: rodio::Float = 0.5;
pub const DEFAULT_SONG_LIST_LIMIT: i32 = 10_000;

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeybindSettings {
    pub play_pause: String,
    pub previous: String,
    pub next: String,
    pub repeat: String,
    pub shuffle: String,
    pub mute: String,
    pub increase_volume: String,
    pub decrease_volume: String,
    pub seek_forward: String,
    pub seek_backward: String,
    pub toggle_search: String,
    pub toggle_settings: String,
    pub open_keybind_settings: String,
    pub toggle_filter_menu: String,
    pub toggle_song_filter_menu: String,
    pub switch_song_filter_pane: String,
    pub apply_selected_filter: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsSnapshot {
    pub music_folder_path: String,
    pub has_processed_music_folder: bool,
    pub saved_search_blob: String,
    pub song_list_limit: i32,
    pub always_start_paused: bool,
    pub keybinds: KeybindSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSettingsSnapshot {
    pub saved_index: usize,
    pub saved_seek: usize,
    pub is_playing: bool,
    pub is_repeat: bool,
    pub is_random: bool,
    pub always_start_paused: bool,
    pub volume: rodio::Float,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MusicFolderSyncSettings {
    pub music_folder_path: String,
    pub has_processed_music_folder: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SongListQuerySettings {
    pub saved_search_blob: String,
    pub song_list_limit: i32,
}

#[async_trait]
pub trait SettingRepository<DB>
where
    DB: Database,
{
    async fn set<'a, A>(&self, acquiree: A, key: &str, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<Setting>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get_many<'a, A>(&self, acquiree: A, keys: &[&str]) -> anyhow::Result<Vec<Setting>>
    where
        A: Acquire<'a, Database = DB> + Send;
}

pub struct SettingService<R, DB>
where
    R: SettingRepository<DB>,
    DB: Database,
{
    _db: PhantomData<DB>,
    repo: R,
}

impl<R, DB> SettingService<R, DB>
where
    R: SettingRepository<DB>,
    DB: Database,
{
    pub fn new(repo: R) -> Self {
        Self {
            _db: PhantomData,
            repo,
        }
    }

    pub async fn get_app_settings_snapshot<'a, A>(
        &self,
        acquiree: A,
    ) -> anyhow::Result<AppSettingsSnapshot>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let values = self
            .get_values(
                acquiree,
                &[
                    MUSIC_FOLDER_PATH_KEY,
                    PROCESSED_MUSIC_FLAG,
                    SEARCH_BLOB,
                    SONG_LIST_LIMIT,
                    ALWAYS_START_PAUSED_FLAG,
                    PLAY_PAUSE_KEYBIND,
                    PREVIOUS_KEYBIND,
                    NEXT_KEYBIND,
                    REPEAT_KEYBIND,
                    SHUFFLE_KEYBIND,
                    MUTE_KEYBIND,
                    INCREASE_VOLUME_KEYBIND,
                    DECREASE_VOLUME_KEYBIND,
                    SEEK_FORWARD_KEYBIND,
                    SEEK_BACKWARD_KEYBIND,
                    FOCUS_SEARCH_KEYBIND,
                    SETTINGS_KEYBIND,
                    KEYBIND_SETTINGS_KEYBIND,
                    FILTER_MENU_KEYBIND,
                    SONG_FILTER_MENU_KEYBIND,
                    SWITCH_SONG_FILTER_PANE_KEYBIND,
                    APPLY_SELECTED_FILTER_KEYBIND,
                ],
            )
            .await?;

        Ok(AppSettingsSnapshot {
            music_folder_path: self.string_value(&values, MUSIC_FOLDER_PATH_KEY),
            has_processed_music_folder: self.bool_value(&values, PROCESSED_MUSIC_FLAG),
            saved_search_blob: self.string_value(&values, SEARCH_BLOB),
            song_list_limit: self.positive_i32_value(
                &values,
                SONG_LIST_LIMIT,
                DEFAULT_SONG_LIST_LIMIT,
            ),
            always_start_paused: self.bool_value(&values, ALWAYS_START_PAUSED_FLAG),
            keybinds: KeybindSettings {
                play_pause: self.string_value(&values, PLAY_PAUSE_KEYBIND),
                previous: self.string_value(&values, PREVIOUS_KEYBIND),
                next: self.string_value(&values, NEXT_KEYBIND),
                repeat: self.string_value(&values, REPEAT_KEYBIND),
                shuffle: self.string_value(&values, SHUFFLE_KEYBIND),
                mute: self.string_value(&values, MUTE_KEYBIND),
                increase_volume: self.string_value(&values, INCREASE_VOLUME_KEYBIND),
                decrease_volume: self.string_value(&values, DECREASE_VOLUME_KEYBIND),
                seek_forward: self.string_value(&values, SEEK_FORWARD_KEYBIND),
                seek_backward: self.string_value(&values, SEEK_BACKWARD_KEYBIND),
                toggle_search: self.string_value(&values, FOCUS_SEARCH_KEYBIND),
                toggle_settings: self.string_value(&values, SETTINGS_KEYBIND),
                open_keybind_settings: self.string_value(&values, KEYBIND_SETTINGS_KEYBIND),
                toggle_filter_menu: self.string_value(&values, FILTER_MENU_KEYBIND),
                toggle_song_filter_menu: self.string_value(&values, SONG_FILTER_MENU_KEYBIND),
                switch_song_filter_pane: self
                    .string_value(&values, SWITCH_SONG_FILTER_PANE_KEYBIND),
                apply_selected_filter: self.string_value(&values, APPLY_SELECTED_FILTER_KEYBIND),
            },
        })
    }

    pub async fn get_player_settings_snapshot<'a, A>(
        &self,
        acquiree: A,
    ) -> anyhow::Result<PlayerSettingsSnapshot>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let values = self
            .get_values(
                acquiree,
                &[
                    INDEX_VALUE,
                    CURRENT_SEEK_VALUE,
                    PLAY_PAUSE_FLAG,
                    REPEAT_FLAG,
                    RANDOM_PLAY_FLAG,
                    ALWAYS_START_PAUSED_FLAG,
                    VOLUME_VALUE,
                ],
            )
            .await?;

        Ok(PlayerSettingsSnapshot {
            saved_index: self.usize_value(&values, INDEX_VALUE, 0),
            saved_seek: self.usize_value(&values, CURRENT_SEEK_VALUE, 0),
            is_playing: self.bool_value(&values, PLAY_PAUSE_FLAG),
            is_repeat: self.bool_value(&values, REPEAT_FLAG),
            is_random: self.bool_value(&values, RANDOM_PLAY_FLAG),
            always_start_paused: self.bool_value(&values, ALWAYS_START_PAUSED_FLAG),
            volume: self.float_value(&values, VOLUME_VALUE, DEFAULT_VOLUME),
        })
    }

    pub async fn get_music_folder_sync_settings<'a, A>(
        &self,
        acquiree: A,
    ) -> anyhow::Result<MusicFolderSyncSettings>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let values = self
            .get_values(acquiree, &[MUSIC_FOLDER_PATH_KEY, PROCESSED_MUSIC_FLAG])
            .await?;

        Ok(MusicFolderSyncSettings {
            music_folder_path: self.string_value(&values, MUSIC_FOLDER_PATH_KEY),
            has_processed_music_folder: self.bool_value(&values, PROCESSED_MUSIC_FLAG),
        })
    }

    pub async fn get_song_list_query_settings<'a, A>(
        &self,
        acquiree: A,
    ) -> anyhow::Result<SongListQuerySettings>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let values = self
            .get_values(acquiree, &[SEARCH_BLOB, SONG_LIST_LIMIT])
            .await?;

        Ok(SongListQuerySettings {
            saved_search_blob: self.string_value(&values, SEARCH_BLOB),
            song_list_limit: self.positive_i32_value(
                &values,
                SONG_LIST_LIMIT,
                DEFAULT_SONG_LIST_LIMIT,
            ),
        })
    }

    pub async fn get_music_folder_path<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, MUSIC_FOLDER_PATH_KEY).await
    }

    pub async fn set_music_folder_path<'a, A>(&self, acquiree: A, path: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, MUSIC_FOLDER_PATH_KEY, path).await
    }

    pub async fn set_repeat_flag<'a, A>(&self, acquiree: A, flag: bool) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set_bool(acquiree, REPEAT_FLAG, flag).await
    }

    pub async fn is_repeat_flag<'a, A>(&self, acquiree: A) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_bool(acquiree, REPEAT_FLAG).await
    }

    pub async fn get_saved_search_blob<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, SEARCH_BLOB).await
    }

    pub async fn set_saved_search_blob<'a, A>(
        &self,
        acquiree: A,
        search_blob: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, SEARCH_BLOB, search_blob).await
    }

    pub async fn persist_started_track<'a, A>(
        &self,
        acquiree: A,
        current_index: Option<usize>,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let mut tx = acquiree.begin().await?;
        self.persist_started_track_in(&mut tx, current_index)
            .await?;
        tx.commit().await?;

        Ok(())
    }

    pub async fn persist_queue_sync<'a, A>(
        &self,
        acquiree: A,
        current_index: Option<usize>,
        cleared_current_song: bool,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let mut tx = acquiree.begin().await?;
        self.persist_queue_sync_in(&mut tx, current_index, cleared_current_song)
            .await?;
        tx.commit().await?;

        Ok(())
    }

    pub async fn reset_library_state(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
    ) -> anyhow::Result<()> {
        self.set_saved_search_blob(&mut *tx, "").await?;
        self.set_saved_index(&mut *tx, 0).await?;
        self.set_current_song_seek(&mut *tx, 0).await?;
        self.set_play_pause_flag(&mut *tx, false).await?;
        self.set_processed_music_folder(&mut *tx, true).await?;

        Ok(())
    }

    pub async fn get_song_list_limit<'a, A>(&self, acquiree: A) -> i32
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(acquiree, SONG_LIST_LIMIT).await {
            Ok(setting) => match setting.value.parse::<i32>() {
                Ok(limit) if limit > 0 => limit,
                _ => DEFAULT_SONG_LIST_LIMIT,
            },
            Err(_) => DEFAULT_SONG_LIST_LIMIT,
        }
    }

    pub async fn set_song_list_limit<'a, A>(&self, acquiree: A, limit: i32) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        if limit <= 0 {
            anyhow::bail!("song list limit must be greater than 0");
        }

        self.set(acquiree, SONG_LIST_LIMIT, &limit.to_string())
            .await
    }

    pub async fn get_saved_index<'a, A>(&self, acquiree: A) -> usize
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(acquiree, INDEX_VALUE).await {
            Ok(setting) => setting.value.parse::<usize>().unwrap_or(0),
            Err(_) => 0,
        }
    }

    pub async fn set_saved_index<'a, A>(&self, acquiree: A, index: usize) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, INDEX_VALUE, &index.to_string()).await
    }

    pub async fn get_current_song_seek<'a, A>(&self, acquiree: A) -> usize
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(acquiree, CURRENT_SEEK_VALUE).await {
            Ok(setting) => setting.value.parse::<usize>().unwrap_or(0),
            Err(_) => 0,
        }
    }

    pub async fn set_current_song_seek<'a, A>(
        &self,
        acquiree: A,
        value: usize,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, CURRENT_SEEK_VALUE, &value.to_string())
            .await
    }

    pub async fn get_pending_song_metadata_sync_paths<'a, A>(&self, acquiree: A) -> Vec<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self
            .get_value(acquiree, PENDING_SONG_METADATA_SYNC_PATHS)
            .await
        {
            Ok(value) => serde_json::from_str::<Vec<String>>(&value).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    pub async fn set_pending_song_metadata_sync_paths<'a, A>(
        &self,
        acquiree: A,
        file_paths: &[String],
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let mut normalized = file_paths
            .iter()
            .map(|path| path.trim())
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();

        let value = serde_json::to_string(&normalized)?;
        self.set(acquiree, PENDING_SONG_METADATA_SYNC_PATHS, &value)
            .await
    }

    pub async fn get_saved_volume_value<'a, A>(&self, acquiree: A) -> rodio::Float
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(acquiree, VOLUME_VALUE).await {
            Ok(setting) => setting
                .value
                .parse::<rodio::Float>()
                .unwrap_or(DEFAULT_VOLUME),
            Err(_) => DEFAULT_VOLUME,
        }
    }

    pub async fn set_saved_volume_value<'a, A>(
        &self,
        acquiree: A,
        volume: rodio::Float,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, VOLUME_VALUE, &volume.to_string()).await
    }

    pub async fn set_processed_music_folder<'a, A>(
        &self,
        acquiree: A,
        flag: bool,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set_bool(acquiree, PROCESSED_MUSIC_FLAG, flag).await
    }

    pub async fn has_processed_music_folder<'a, A>(&self, acquiree: A) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_bool(acquiree, PROCESSED_MUSIC_FLAG).await
    }

    pub async fn set_settings_keybind<'a, A>(&self, acquiree: A, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, SETTINGS_KEYBIND, value).await
    }

    pub async fn get_settings_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, SETTINGS_KEYBIND).await
    }

    pub async fn set_play_pause_flag<'a, A>(&self, acquiree: A, playing: bool) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set_bool(acquiree, PLAY_PAUSE_FLAG, playing).await
    }

    pub async fn is_playing<'a, A>(&self, acquiree: A) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_bool(acquiree, PLAY_PAUSE_FLAG).await
    }

    pub async fn set_random_play<'a, A>(&self, acquiree: A, flag: bool) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set_bool(acquiree, RANDOM_PLAY_FLAG, flag).await
    }

    pub async fn is_random_play<'a, A>(&self, acquiree: A) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_bool(acquiree, RANDOM_PLAY_FLAG).await
    }

    pub async fn set_always_start_paused<'a, A>(
        &self,
        acquiree: A,
        flag: bool,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set_bool(acquiree, ALWAYS_START_PAUSED_FLAG, flag)
            .await
    }

    pub async fn should_always_start_paused<'a, A>(&self, acquiree: A) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_bool(acquiree, ALWAYS_START_PAUSED_FLAG).await
    }

    pub async fn get_focus_search_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, FOCUS_SEARCH_KEYBIND).await
    }

    pub async fn set_focus_search_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, FOCUS_SEARCH_KEYBIND, value).await
    }

    pub async fn get_mute_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, MUTE_KEYBIND).await
    }

    pub async fn set_mute_keybind<'a, A>(&self, acquiree: A, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, MUTE_KEYBIND, value).await
    }

    pub async fn get_repeat_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, REPEAT_KEYBIND).await
    }

    pub async fn set_repeat_keybind<'a, A>(&self, acquiree: A, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, REPEAT_KEYBIND, value).await
    }

    pub async fn get_shuffle_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, SHUFFLE_KEYBIND).await
    }

    pub async fn set_shuffle_keybind<'a, A>(&self, acquiree: A, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, SHUFFLE_KEYBIND, value).await
    }

    pub async fn set_previous_keybind<'a, A>(&self, acquiree: A, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, PREVIOUS_KEYBIND, value).await
    }

    pub async fn get_previous_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, PREVIOUS_KEYBIND).await
    }

    pub async fn set_next_keybind<'a, A>(&self, acquiree: A, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, NEXT_KEYBIND, value).await
    }

    pub async fn get_next_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, NEXT_KEYBIND).await
    }

    pub async fn set_play_pause_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, PLAY_PAUSE_KEYBIND, value).await
    }

    pub async fn get_play_pause_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, PLAY_PAUSE_KEYBIND).await
    }

    pub async fn set_increase_volume_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, INCREASE_VOLUME_KEYBIND, value).await
    }

    pub async fn get_increase_volume_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, INCREASE_VOLUME_KEYBIND).await
    }

    pub async fn set_decrease_volume_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, DECREASE_VOLUME_KEYBIND, value).await
    }

    pub async fn get_decrease_volume_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, DECREASE_VOLUME_KEYBIND).await
    }

    pub async fn set_seek_forward_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, SEEK_FORWARD_KEYBIND, value).await
    }

    pub async fn get_seek_forward_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, SEEK_FORWARD_KEYBIND).await
    }

    pub async fn set_seek_backward_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, SEEK_BACKWARD_KEYBIND, value).await
    }

    pub async fn get_seek_backward_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, SEEK_BACKWARD_KEYBIND).await
    }

    pub async fn set_filter_menu_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, FILTER_MENU_KEYBIND, value).await
    }

    pub async fn get_filter_menu_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, FILTER_MENU_KEYBIND).await
    }

    pub async fn set_song_filter_menu_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, SONG_FILTER_MENU_KEYBIND, value).await
    }

    pub async fn get_song_filter_menu_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, SONG_FILTER_MENU_KEYBIND).await
    }

    pub async fn set_keybind_settings_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, KEYBIND_SETTINGS_KEYBIND, value).await
    }

    pub async fn get_keybind_settings_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, KEYBIND_SETTINGS_KEYBIND).await
    }

    pub async fn set_switch_song_filter_pane_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, SWITCH_SONG_FILTER_PANE_KEYBIND, value)
            .await
    }

    pub async fn get_switch_song_filter_pane_keybind<'a, A>(
        &self,
        acquiree: A,
    ) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, SWITCH_SONG_FILTER_PANE_KEYBIND)
            .await
    }

    pub async fn set_apply_selected_filter_keybind<'a, A>(
        &self,
        acquiree: A,
        value: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, APPLY_SELECTED_FILTER_KEYBIND, value)
            .await
    }

    pub async fn get_apply_selected_filter_keybind<'a, A>(
        &self,
        acquiree: A,
    ) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.get_value(acquiree, APPLY_SELECTED_FILTER_KEYBIND)
            .await
    }

    async fn persist_started_track_in(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        current_index: Option<usize>,
    ) -> anyhow::Result<()> {
        self.set_saved_index(&mut *tx, current_index.unwrap_or(0))
            .await?;
        self.set_current_song_seek(&mut *tx, 0).await?;
        self.set_play_pause_flag(&mut *tx, current_index.is_some())
            .await?;

        Ok(())
    }

    async fn persist_queue_sync_in(
        &self,
        tx: &mut sqlx::Transaction<'_, DB>,
        current_index: Option<usize>,
        cleared_current_song: bool,
    ) -> anyhow::Result<()> {
        self.set_saved_index(&mut *tx, current_index.unwrap_or(0))
            .await?;

        if cleared_current_song {
            self.set_current_song_seek(&mut *tx, 0).await?;
            self.set_play_pause_flag(&mut *tx, false).await?;
        }

        Ok(())
    }

    async fn set<'a, A>(&self, acquiree: A, key: &str, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        log::debug!("trying to set key[{key}] -> value[{value}]");

        self.repo
            .set(acquiree, key, value)
            .await
            .with_context(|| format!("Failed to set (key, value): ({},{})", key, value))
    }

    async fn set_bool<'a, A>(&self, acquiree: A, key: &str, value: bool) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(acquiree, key, if value { "true" } else { "false" })
            .await
    }

    async fn get_value<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        Ok(self.get(acquiree, key).await?.value)
    }

    async fn get_bool<'a, A>(&self, acquiree: A, key: &str) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        matches!(self.get_value(acquiree, key).await.as_deref(), Ok("true"))
    }

    async fn get<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<Setting>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        log::debug!("trying to get key[{key}]");

        self.repo
            .get(acquiree, key)
            .await
            .with_context(|| format!("Failed to get key: {}", key))
    }

    async fn get_values<'a, A>(
        &self,
        acquiree: A,
        keys: &[&str],
    ) -> anyhow::Result<HashMap<String, String>>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        log::debug!("trying to get keys[{}]", keys.join(","));

        let settings = self
            .repo
            .get_many(acquiree, keys)
            .await
            .with_context(|| format!("Failed to get keys: {}", keys.join(",")))?;

        Ok(settings
            .into_iter()
            .map(|setting| (setting.key, setting.value))
            .collect())
    }

    fn string_value(&self, values: &HashMap<String, String>, key: &str) -> String {
        values.get(key).cloned().unwrap_or_default()
    }

    fn bool_value(&self, values: &HashMap<String, String>, key: &str) -> bool {
        matches!(values.get(key).map(String::as_str), Some("true"))
    }

    fn positive_i32_value(&self, values: &HashMap<String, String>, key: &str, default: i32) -> i32 {
        values
            .get(key)
            .and_then(|value| value.parse::<i32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(default)
    }

    fn usize_value(&self, values: &HashMap<String, String>, key: &str, default: usize) -> usize {
        values
            .get(key)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(default)
    }

    fn float_value(
        &self,
        values: &HashMap<String, String>,
        key: &str,
        default: rodio::Float,
    ) -> rodio::Float {
        values
            .get(key)
            .and_then(|value| value.parse::<rodio::Float>().ok())
            .unwrap_or(default)
    }
}
