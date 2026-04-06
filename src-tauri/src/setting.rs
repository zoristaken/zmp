use anyhow::Context;
use async_trait::async_trait;
use sqlx::{Acquire, Database};

use crate::manager::HasPool;

const MUSIC_FOLDER_PATH_KEY: &str = "music_folder_path";
const PROCESSED_MUSIC_FLAG: &str = "processed_music_flag";
const VOLUME_VALUE: &str = "volume_value";
const SEARCH_BLOB: &str = "search_blob";
const REPEAT_FLAG: &str = "repeat_flag";
const RANDOM_PLAY_FLAG: &str = "random_play_flag";
const PLAY_PAUSE_FLAG: &str = "play_pause_flag";
const SETTINGS_KEYBIND: &str = "settings_kb";
const PLAY_STOP_KEYBIND: &str = "play_stop_kb";
const PREVIOUS_KEYBIND: &str = "previous_kb";
const NEXT_KEYBIND: &str = "next_kb";
const RANDOM_KEYBIND: &str = "random_kb";
const INDEX_VALUE: &str = "index_value";
const CURRENT_SEEK_VALUE: &str = "current_seek_value";
pub const DEFAULT_VOLUME: rodio::Float = 0.5;

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Setting {
    pub id: i32,
    pub key: String,
    pub value: String,
}

#[async_trait]
pub trait SettingRepository<DB>
where
    DB: Database,
{
    async fn set<'a, A>(&self, executor: A, key: &str, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send;
    async fn get<'a, A>(&self, executor: A, key: &str) -> anyhow::Result<Setting>
    where
        A: Acquire<'a, Database = DB> + Send;
}

pub struct SettingService<R, DB>
where
    R: SettingRepository<DB>,
    DB: Database,
{
    pub pool: sqlx::Pool<DB>,
    repo: R,
}

impl<R, DB> SettingService<R, DB>
where
    R: SettingRepository<DB> + HasPool<DB>,
    DB: Database,
{
    pub fn new(repo: R) -> Self {
        Self {
            pool: repo.pool().clone(),
            repo,
        }
    }

    pub async fn get_music_folder_path<'a, A>(&self, executor: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        Ok(self.get(executor, MUSIC_FOLDER_PATH_KEY).await?.value)
    }

    pub async fn set_music_folder_path<'a, A>(&self, executor: A, path: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, MUSIC_FOLDER_PATH_KEY, path).await
    }

    pub async fn set_repeat_flag<'a, A>(&self, executor: A, flag: bool) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        if flag {
            self.set(executor, REPEAT_FLAG, "true").await
        } else {
            self.set(executor, REPEAT_FLAG, "false").await
        }
    }

    pub async fn is_repeat_flag<'a, A>(&self, executor: A) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(executor, REPEAT_FLAG).await {
            Ok(setting) => setting.value == "true",
            Err(_) => false,
        }
    }

    pub async fn get_saved_search_blob<'a, A>(&self, executor: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let setting = self.get(executor, SEARCH_BLOB).await?;

        Ok(setting.value)
    }

    pub async fn set_saved_search_blob<'a, A>(
        &self,
        executor: A,
        search_blob: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, SEARCH_BLOB, search_blob).await
    }

    pub async fn get_saved_index<'a, A>(&self, executor: A) -> usize
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(executor, INDEX_VALUE).await {
            Ok(setting) => setting.value.parse::<usize>().unwrap_or(0),
            Err(_) => 0,
        }
    }

    pub async fn set_saved_index<'a, A>(&self, executor: A, index: usize) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, INDEX_VALUE, &index.to_string()).await
    }

    pub async fn get_current_song_seek<'a, A>(&self, executor: A) -> usize
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(executor, CURRENT_SEEK_VALUE).await {
            Ok(setting) => setting.value.parse::<usize>().unwrap_or(0),
            Err(_) => 0,
        }
    }

    pub async fn set_current_song_seek<'a, A>(
        &self,
        executor: A,
        value: usize,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, CURRENT_SEEK_VALUE, &value.to_string())
            .await
    }

    pub async fn get_saved_volume_value<'a, A>(&self, executor: A) -> rodio::Float
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(executor, VOLUME_VALUE).await {
            Ok(setting) => setting
                .value
                .parse::<rodio::Float>()
                .unwrap_or(DEFAULT_VOLUME),
            Err(_) => DEFAULT_VOLUME,
        }
    }

    pub async fn set_saved_volume_value<'a, A>(
        &self,
        executor: A,
        volume: rodio::Float,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, VOLUME_VALUE, &volume.to_string()).await
    }

    pub async fn set_processed_music_folder<'a, A>(
        &self,
        executor: A,
        flag: bool,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        if flag {
            self.set(executor, PROCESSED_MUSIC_FLAG, "true").await
        } else {
            self.set(executor, PROCESSED_MUSIC_FLAG, "false").await
        }
    }

    pub async fn has_processed_music_folder<'a, A>(&self, executor: A) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(executor, PROCESSED_MUSIC_FLAG).await {
            Ok(setting) => setting.value == "true",
            Err(_) => false,
        }
    }

    pub async fn set_settings_keybind<'a, A>(&self, executor: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, SETTINGS_KEYBIND, key).await
    }

    pub async fn get_settings_keybind<'a, A>(&self, executor: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let setting = self.get(executor, SETTINGS_KEYBIND).await?;

        Ok(setting.value)
    }

    pub async fn set_play_pause_flag<'a, A>(&self, executor: A, playing: bool) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        if playing {
            self.set(executor, PLAY_PAUSE_FLAG, "true").await
        } else {
            self.set(executor, PLAY_PAUSE_FLAG, "false").await
        }
    }

    pub async fn is_playing<'a, A>(&self, executor: A) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(executor, PLAY_PAUSE_FLAG).await {
            Ok(setting) => setting.value == "true",
            Err(_) => false,
        }
    }

    pub async fn set_random_keybind<'a, A>(&self, executor: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, RANDOM_KEYBIND, key).await
    }

    pub async fn get_random_keybind<'a, A>(&self, executor: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let setting = self.get(executor, RANDOM_KEYBIND).await?;

        Ok(setting.value)
    }

    pub async fn set_previous_keybind<'a, A>(&self, executor: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, PREVIOUS_KEYBIND, key).await
    }

    pub async fn get_previous_keybind<'a, A>(&self, executor: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let setting = self.get(executor, PREVIOUS_KEYBIND).await?;

        Ok(setting.value)
    }

    pub async fn set_next_keybind<'a, A>(&self, executor: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, NEXT_KEYBIND, key).await
    }

    pub async fn get_next_keybind<'a, A>(&self, executor: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let setting = self.get(executor, NEXT_KEYBIND).await?;

        Ok(setting.value)
    }

    pub async fn set_play_stop_keybind<'a, A>(&self, executor: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.set(executor, PLAY_STOP_KEYBIND, key).await
    }

    pub async fn get_play_stop_keybind<'a, A>(&self, executor: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        let setting = self.get(executor, PLAY_STOP_KEYBIND).await?;

        Ok(setting.value)
    }

    pub async fn set_random_play<'a, A>(&self, executor: A, flag: bool) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        if flag {
            self.set(executor, RANDOM_PLAY_FLAG, "true").await
        } else {
            self.set(executor, RANDOM_PLAY_FLAG, "false").await
        }
    }

    pub async fn is_random_play<'a, A>(&self, executor: A) -> bool
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        match self.get(executor, RANDOM_PLAY_FLAG).await {
            Ok(setting) => setting.value == "true",
            Err(_) => false,
        }
    }

    async fn set<'a, A>(&self, executor: A, key: &str, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo
            .set(executor, key, value)
            .await
            .with_context(|| format!("Failed to set (key, value): ({},{})", key, value))
    }

    async fn get<'a, A>(&self, executor: A, key: &str) -> anyhow::Result<Setting>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo
            .get(executor, key)
            .await
            .with_context(|| format!("Failed to get key: {}", key))
    }
}
