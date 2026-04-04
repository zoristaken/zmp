use tauri::Manager;

use crate::{manager::AppState, sqlite::SqliteDb};

mod config;
pub mod filter;
mod manager;
pub mod metadata;
pub mod player;
pub mod setting;
pub mod song;
pub mod song_filter;
pub mod sqlite;
mod test;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![test::main_test])
        .setup(|app| {
            tauri::async_runtime::block_on(async move {
                let config = config::Config::new(&app).await.unwrap();
                let path = config.db_path().await.unwrap();
                let sqlite = SqliteDb::new(&path).await.unwrap();

                app.manage(AppState { db: sqlite });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
