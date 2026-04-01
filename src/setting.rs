use anyhow::Context;

const MUSIC_FOLDER_PATH_KEY: &str = "music_folder_path";
const PROCESSED_MUSIC_FLAG: &str = "processed_music_flag";
const RANDOM_PLAY_FLAG: &str = "random_play_flag";
const SETTINGS_KEYBIND: &str = "settings_kb";
const PLAY_STOP_KEYBIND: &str = "play_stop_kb";
const PREVIOUS_KEYBIND: &str = "previous_kb";
const NEXT_KEYBIND: &str = "next_kb";
const RANDOM_KEYBIND: &str = "random_kb";

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Setting {
    pub id: i32,
    pub key: String,
    pub value: String,
}

pub trait SettingRepository: Send + Sync {
    async fn set(&self, key: &str, value: &str);
    async fn get(&self, key: &str) -> anyhow::Result<Setting>;
}

pub struct SettingService<R: SettingRepository> {
    repo: R,
}

impl<R: SettingRepository> SettingService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn get_music_folder_path(&self) -> String {
        match self.get(MUSIC_FOLDER_PATH_KEY).await {
            Ok(setting) => setting.value,
            Err(_) => "".to_string(),
        }
    }

    pub async fn set_music_folder_path(&self, path: &str) {
        self.set(MUSIC_FOLDER_PATH_KEY, path).await;
    }

    pub async fn set_processed_music_folder(&self, flag: bool) {
        if flag {
            self.set(PROCESSED_MUSIC_FLAG, "true").await
        } else {
            self.set(PROCESSED_MUSIC_FLAG, "false").await
        }
    }

    pub async fn has_processed_music_folder(&self) -> bool {
        match self.get(PROCESSED_MUSIC_FLAG).await {
            Ok(setting) => {
                if setting.value == "true" {
                    return true;
                } else {
                    return false;
                }
            }
            Err(_) => return false,
        };
    }

    pub async fn set_settings_keybind(&self, key: &str) {
        self.set(SETTINGS_KEYBIND, key).await
    }

    pub async fn get_settings_keybind(&self) -> anyhow::Result<String> {
        let setting = self
            .get(SETTINGS_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", SETTINGS_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_random_keybind(&self, key: &str) {
        self.set(RANDOM_KEYBIND, key).await
    }

    pub async fn get_random_keybind(&self) -> anyhow::Result<String> {
        let setting = self
            .get(RANDOM_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", RANDOM_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_previous_keybind(&self, key: &str) {
        self.set(PREVIOUS_KEYBIND, key).await
    }

    pub async fn get_previous_keybind(&self) -> anyhow::Result<String> {
        let setting = self
            .get(PREVIOUS_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", PREVIOUS_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_next_keybind(&self, key: &str) {
        self.set(NEXT_KEYBIND, key).await
    }

    pub async fn get_next_keybind(&self) -> anyhow::Result<String> {
        let setting = self
            .get(NEXT_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", NEXT_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_play_stop_keybind(&self, key: &str) {
        self.set(PLAY_STOP_KEYBIND, key).await
    }

    pub async fn get_play_stop_keybind(&self) -> anyhow::Result<String> {
        let setting = self
            .get(PLAY_STOP_KEYBIND)
            .await
            .with_context(|| format!("Failed to get {} keybind", PLAY_STOP_KEYBIND))?;

        Ok(setting.value)
    }

    pub async fn set_random_play(&self, flag: bool) {
        if flag {
            self.set(RANDOM_PLAY_FLAG, "true").await
        } else {
            self.set(RANDOM_PLAY_FLAG, "false").await
        }
    }

    pub async fn is_random_play(&self) -> bool {
        match self.get(RANDOM_PLAY_FLAG).await {
            Ok(setting) => {
                if setting.value == "true" {
                    return true;
                }
                return false;
            }
            Err(_) => return false,
        }
    }

    async fn set(&self, key: &str, value: &str) {
        self.repo.set(key, value).await
    }

    async fn get(&self, key: &str) -> anyhow::Result<Setting> {
        self.repo.get(key).await
    }
}
