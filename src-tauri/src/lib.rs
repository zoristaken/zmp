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
pub mod song_mutation;
pub mod song_query;
pub mod sqlite;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::process_music_folder,
            commands::load,
            commands::search_songs,
            commands::get_music_folder_path,
            commands::set_music_folder_path,
            commands::has_processed_music_folder,
            commands::set_processed_music_folder,
            commands::get_current_index,
            commands::get_current_song,
            commands::get_saved_search_blob,
            commands::get_song_list_limit,
            commands::set_song_list_limit,
            commands::get_loaded_songs,
            commands::play_song_at,
            commands::next_song,
            commands::previous_song,
            commands::get_current_song_seek,
            commands::set_current_song_seek,
            commands::save_current_song_seek,
            commands::get_is_player_paused,
            commands::get_play_pause,
            commands::set_play_pause,
            commands::get_volume,
            commands::set_volume,
            commands::get_random,
            commands::set_random,
            commands::get_repeat,
            commands::set_repeat,
            commands::get_mute_keybind,
            commands::set_mute_keybind,
            commands::get_next_keybind,
            commands::set_next_keybind,
            commands::get_repeat_keybind,
            commands::set_repeat_keybind,
            commands::get_shuffle_keybind,
            commands::set_shuffle_keybind,
            commands::get_previous_keybind,
            commands::set_previous_keybind,
            commands::get_settings_keybind,
            commands::set_settings_keybind,
            commands::get_play_pause_keybind,
            commands::set_play_pause_keybind,
            commands::get_focus_search_keybind,
            commands::set_focus_search_keybind,
            commands::create_filter,
            commands::get_filters,
            commands::remove_filter,
            commands::add_filter_to_song,
            commands::get_filters_for_song,
            commands::remove_filter_from_song,
        ])
        .setup(|app| {
            tauri::async_runtime::block_on(async move {
                let config = config::Config::new(app).await.unwrap();
                let path = config.db_path().await.unwrap();
                let sqlite = SqliteDb::new(&path).await.unwrap();
                app.manage(AppState {
                    zmp: PlayerManager::new(sqlite.clone()).await,
                });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
