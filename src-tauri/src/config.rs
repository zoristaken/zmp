use anyhow::Context;
use std::path::PathBuf;
use tauri::App;
use tauri::Manager;

const APP_NAME: &str = "zmp";
const APP_NAME_DEV: &str = "zmp_dev";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub config_path: PathBuf,
}

impl Config {
    pub async fn new(app: &App) -> anyhow::Result<Config> {
        Ok(Config {
            config_path: app
                .path()
                .app_data_dir()
                .with_context(|| "failed to get data_dir")?,
        })
    }

    pub async fn sqlite_path(&self) -> anyhow::Result<String> {
        let path = self
            .config_path
            .to_str()
            .with_context(|| "failed to get app path")?;

        if cfg!(debug_assertions) {
            return Ok(format!("sqlite:///{}/{}.db", path, APP_NAME_DEV));
        }
        Ok(format!("sqlite:///{}/{}.db", path, APP_NAME))
    }
}
