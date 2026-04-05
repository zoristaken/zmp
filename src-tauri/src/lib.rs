use std::sync::Mutex;

use tauri::Manager;

use crate::{
    manager::{AppState, PlayerManager},
    sqlite::SqliteDb,
};

mod commands;
mod config;
pub mod filter;
mod manager;
pub mod metadata;
pub mod player;
pub mod setting;
pub mod song;
pub mod song_filter;
pub mod sqlite;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::init,
            commands::load,
            commands::next_song,
            commands::previous_song,
            commands::play_pause,
            commands::set_volume,
            commands::set_random,
            commands::set_repeat,
        ])
        .setup(|app| {
            tauri::async_runtime::block_on(async move {
                let config = config::Config::new(&app).await.unwrap();
                let path = config.db_path().await.unwrap();
                let sqlite = SqliteDb::new(&path).await.unwrap();
                app.manage(AppState {
                    loaded_songs: Mutex::new(Vec::new()),
                    zmp: PlayerManager::new(sqlite.clone()),
                });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
