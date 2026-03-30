use anyhow::anyhow;
use std::env;
use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "zmp";
const APP_NAME_DEV: &str = "zmp_dev";
const HOME: &str = "HOME";
const WINDOWS_DEFAULT_PATH: &str = "APPDATA";
const MAC_DEFAULT_PATH: &str = "Library/Application Support";
const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";
const LINUX_CONFIG: &str = ".config";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub config_path: String,
    pub database_url: String,
}

impl Config {
    //if multiple databases could be used in the future, refactor code
    //to take the type of dabase as an argument so you can generate
    //the appropriate path or url
    pub fn new() -> anyhow::Result<Config> {
        let config_path = Config::get_or_create_config_dir()?;
        let database_url: String;
        if cfg!(debug_assertions) {
            database_url = format!("sqlite:///{}/{}.db", config_path, APP_NAME_DEV);
        } else {
            database_url = format!("sqlite:///{}/{}.db", config_path, APP_NAME);
        }

        Ok(Config {
            config_path: config_path,
            database_url: database_url,
        })
    }

    fn get_or_create_config_dir() -> anyhow::Result<String> {
        let config_dir = match env::consts::OS {
            "windows" => env::var(WINDOWS_DEFAULT_PATH)
                .map(PathBuf::from)
                .map_err(|_| anyhow!("{} environment variable not set", WINDOWS_DEFAULT_PATH))?,
            "macos" => env::var(HOME)
                .map(|h| PathBuf::from(h).join(MAC_DEFAULT_PATH))
                .map_err(|_| anyhow!("{} environment variable not set", HOME))?,
            _ => {
                // Linux/Unix
                env::var(XDG_CONFIG_HOME)
                    .map(PathBuf::from)
                    .or_else(|_| env::var(HOME).map(|h| PathBuf::from(h).join(LINUX_CONFIG)))
                    .map_err(|_| anyhow!("Could not determine config directory"))?
            }
        }
        .join(APP_NAME);

        fs::create_dir_all(&config_dir)
            .map_err(|e| anyhow!("Failed to create config directory: {}", e))?;

        Ok(config_dir.display().to_string())
    }
}
