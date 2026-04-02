use anyhow::Context;
use sqlx::Acquire;

use crate::sqlite::RepositoryDb;

const MUSIC_FOLDER_PATH_KEY: &str = "music_folder_path";
const PROCESSED_MUSIC_FLAG: &str = "processed_music_flag";
const VOLUME_VALUE: &str = "volume_value";
const SEARCH_BLOB: &str = "search_blob";
const REPEAT_FLAG: &str = "repeat_flag";
const RANDOM_PLAY_FLAG: &str = "random_play_flag";
const SETTINGS_KEYBIND: &str = "settings_kb";
const PLAY_STOP_KEYBIND: &str = "play_stop_kb";
const PREVIOUS_KEYBIND: &str = "previous_kb";
const NEXT_KEYBIND: &str = "next_kb";
const RANDOM_KEYBIND: &str = "random_kb";
const DEFAULT_VOLUME: rodio::Float = 0.5;

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Setting {
    pub id: i32,
    pub key: String,
    pub value: String,
}

pub trait SettingRepository<DB>
where
    DB: RepositoryDb + Send + Sync,
{
    async fn set<'a, A>(&self, acquiree: A, key: &str, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>;
    async fn get<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<Setting>
    where
        A: Acquire<'a, Database = DB>;
}

pub struct SettingService<R, DB>
where
    R: SettingRepository<DB>,
    DB: RepositoryDb,
{
    repo: R,
    _db: std::marker::PhantomData<DB>,
}

impl<R, DB> SettingService<R, DB>
where
    R: SettingRepository<DB>,
    DB: RepositoryDb,
{
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            _db: Default::default(),
        }
    }

    pub async fn get_music_folder_path<'a, A>(&self, acquiree: A) -> String
    where
        A: Acquire<'a, Database = DB>,
    {
        match self.get(acquiree, MUSIC_FOLDER_PATH_KEY).await {
            Ok(setting) => setting.value,
            Err(_) => "".to_string(),
        }
    }

    pub async fn set_music_folder_path<'a, A>(&self, acquiree: A, path: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.set(acquiree, MUSIC_FOLDER_PATH_KEY, path).await
    }

    pub async fn set_repeat_flag<'a, A>(&self, acquiree: A, flag: bool) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        if flag {
            self.set(acquiree, REPEAT_FLAG, "true").await
        } else {
            self.set(acquiree, REPEAT_FLAG, "false").await
        }
    }

    pub async fn is_repeat_flag<'a, A>(&self, acquiree: A) -> bool
    where
        A: Acquire<'a, Database = DB>,
    {
        match self.get(acquiree, REPEAT_FLAG).await {
            Ok(setting) => {
                if setting.value == "true" {
                    return true;
                }
                return false;
            }
            Err(_) => return false,
        };
    }

    pub async fn get_saved_search_blob<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB>,
    {
        let setting = self
            .get(acquiree, SEARCH_BLOB)
            .await
            .with_context(|| format!("Failed to get {} value", SEARCH_BLOB))?;

        Ok(setting.value)
    }

    pub async fn set_saved_search_blob<'a, A>(
        &self,
        acquiree: A,
        search_blob: &str,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.set(acquiree, SEARCH_BLOB, search_blob).await
    }

    pub async fn get_saved_volume_value<'a, A>(&self, acquiree: A) -> rodio::Float
    where
        A: Acquire<'a, Database = DB>,
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
        A: Acquire<'a, Database = DB>,
    {
        self.set(acquiree, VOLUME_VALUE, volume.to_string().as_str())
            .await
    }

    pub async fn set_processed_music_folder<'a, A>(
        &self,
        acquiree: A,
        flag: bool,
    ) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        if flag {
            self.set(acquiree, PROCESSED_MUSIC_FLAG, "true").await
        } else {
            self.set(acquiree, PROCESSED_MUSIC_FLAG, "false").await
        }
    }

    pub async fn has_processed_music_folder<'a, A>(&self, acquiree: A) -> bool
    where
        A: Acquire<'a, Database = DB>,
    {
        match self.get(acquiree, PROCESSED_MUSIC_FLAG).await {
            Ok(setting) => {
                if setting.value == "true" {
                    return true;
                }
                return false;
            }
            Err(_) => return false,
        };
    }

    pub async fn set_settings_keybind<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.set(acquiree, SETTINGS_KEYBIND, key).await
    }

    pub async fn get_settings_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB>,
    {
        let setting = self
            .get(acquiree, SETTINGS_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", SETTINGS_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_random_keybind<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.set(acquiree, RANDOM_KEYBIND, key).await
    }

    pub async fn get_random_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB>,
    {
        let setting = self
            .get(acquiree, RANDOM_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", RANDOM_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_previous_keybind<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.set(acquiree, PREVIOUS_KEYBIND, key).await
    }

    pub async fn get_previous_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB>,
    {
        let setting = self
            .get(acquiree, PREVIOUS_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", PREVIOUS_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_next_keybind<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.set(acquiree, NEXT_KEYBIND, key).await
    }

    pub async fn get_next_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB>,
    {
        let setting = self
            .get(acquiree, NEXT_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", NEXT_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_play_stop_keybind<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.set(acquiree, PLAY_STOP_KEYBIND, key).await
    }

    pub async fn get_play_stop_keybind<'a, A>(&self, acquiree: A) -> anyhow::Result<String>
    where
        A: Acquire<'a, Database = DB>,
    {
        let setting = self
            .get(acquiree, PLAY_STOP_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", PLAY_STOP_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_random_play<'a, A>(&self, acquiree: A, flag: bool) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        if flag {
            self.set(acquiree, RANDOM_PLAY_FLAG, "true").await
        } else {
            self.set(acquiree, RANDOM_PLAY_FLAG, "false").await
        }
    }

    pub async fn is_random_play<'a, A>(&self, acquiree: A) -> bool
    where
        A: Acquire<'a, Database = DB>,
    {
        match self.get(acquiree, RANDOM_PLAY_FLAG).await {
            Ok(setting) => {
                if setting.value == "true" {
                    return true;
                }
                return false;
            }
            Err(_) => return false,
        }
    }

    async fn set<'a, A>(&self, acquiree: A, key: &str, value: &str) -> anyhow::Result<()>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.repo.set(acquiree, key, value).await
    }

    async fn get<'a, A>(&self, acquiree: A, key: &str) -> anyhow::Result<Setting>
    where
        A: Acquire<'a, Database = DB>,
    {
        self.repo.get(acquiree, key).await
    }
}
