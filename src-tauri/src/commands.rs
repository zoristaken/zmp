use serde::Serialize;
use sqlx::{Acquire, Sqlite};
use tauri::{AppHandle, Emitter};

use crate::{
    filter::Filter,
    manager::{AppState, HasPool},
    player::QueueSyncResult,
    setting::{SettingRepository, SettingService},
    song_filter::SongFilter,
    song_query::SongWithFilters,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackChangedPayload {
    current_index: Option<usize>,
}

fn emit_track_changed(app: &AppHandle, current_index: Option<usize>) -> Result<(), String> {
    app.emit("track-changed", TrackChangedPayload { current_index })
        .map_err(|e| e.to_string())
}

fn current_song_id(state: &AppState) -> Result<Option<i32>, String> {
    let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    Ok(player.current_song().map(|song| song.song.id))
}

async fn persist_current_index<R>(
    setting: &SettingService<R, Sqlite>,
    current_index: Option<usize>,
) -> Result<(), String>
where
    R: SettingRepository<Sqlite> + HasPool<Sqlite>,
{
    setting
        .set_saved_index(&setting.pool, current_index.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())
}

async fn persist_queue_sync<R>(
    setting: &SettingService<R, Sqlite>,
    sync: QueueSyncResult,
) -> Result<(), String>
where
    R: SettingRepository<Sqlite> + HasPool<Sqlite>,
{
    persist_current_index(setting, sync.current_index).await?;

    if sync.cleared_current_song {
        setting
            .set_current_song_seek(&setting.pool, 0)
            .await
            .map_err(|e| e.to_string())?;

        setting
            .set_play_pause_flag(&setting.pool, false)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn persist_started_track<R>(
    setting: &SettingService<R, Sqlite>,
    current_index: Option<usize>,
) -> Result<(), String>
where
    R: SettingRepository<Sqlite> + HasPool<Sqlite>,
{
    persist_current_index(setting, current_index).await?;

    setting
        .set_current_song_seek(&setting.pool, 0)
        .await
        .map_err(|e| e.to_string())?;

    setting
        .set_play_pause_flag(&setting.pool, current_index.is_some())
        .await
        .map_err(|e| e.to_string())
}

async fn persist_loaded_state<R>(
    setting: &SettingService<R, Sqlite>,
    current_index: Option<usize>,
) -> Result<(), String>
where
    R: SettingRepository<Sqlite> + HasPool<Sqlite>,
{
    persist_current_index(setting, current_index).await?;

    if current_index.is_none() {
        setting
            .set_current_song_seek(&setting.pool, 0)
            .await
            .map_err(|e| e.to_string())?;

        setting
            .set_play_pause_flag(&setting.pool, false)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn query_song_list<'a, A>(
    state: &AppState,
    acquiree: A,
    query: &str,
    limit: i32,
) -> Result<Vec<SongWithFilters>, String>
where
    A: Acquire<'a, Database = Sqlite> + Send,
{
    let trimmed = query.trim();

    if trimmed.is_empty() {
        let pinned_song_id = current_song_id(state)?;

        state
            .zmp
            .song_query
            .list_songs_with_filters(acquiree, limit, pinned_song_id)
            .await
            .map_err(|e| e.to_string())
    } else {
        let words: Vec<&str> = trimmed.split_whitespace().collect();

        state
            .zmp
            .song_query
            .search_by_db_with_filters(acquiree, &words, limit)
            .await
            .map_err(|e| e.to_string())
    }
}

async fn load_song_list_in_tx(
    state: &AppState,
    tx: &mut sqlx::Transaction<'_, Sqlite>,
) -> Result<Vec<SongWithFilters>, String> {
    let saved_search = state
        .zmp
        .setting
        .get_saved_search_blob(&mut *tx)
        .await
        .unwrap_or_default();
    let song_list_limit = state.zmp.setting.get_song_list_limit(&mut *tx).await;

    query_song_list(state, &mut *tx, &saved_search, song_list_limit).await
}

fn sync_song_list(
    state: &AppState,
    songs: Vec<SongWithFilters>,
) -> Result<QueueSyncResult, String> {
    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.set_queue(songs).map_err(|e| e.to_string())
}

macro_rules! keybind_commands {
    ($(($get_command:ident, $set_command:ident, $getter:ident, $setter:ident)),+ $(,)?) => {
        $(
            #[tauri::command]
            pub async fn $get_command(
                state: tauri::State<'_, AppState>,
            ) -> Result<Option<String>, String> {
                Ok(state.zmp.setting.$getter(&state.zmp.pool).await.ok())
            }

            #[tauri::command]
            pub async fn $set_command(
                state: tauri::State<'_, AppState>,
                keybind: String,
            ) -> Result<(), String> {
                state
                    .zmp
                    .setting
                    .$setter(&state.zmp.pool, &keybind)
                    .await
                    .map_err(|e| e.to_string())
            }
        )+
    };
}

#[tauri::command]
pub async fn process_music_folder(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .zmp
        .process_music_folder()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn load(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    let mut tx = state.zmp.pool.begin().await.map_err(|e| e.to_string())?;

    let songs = load_song_list_in_tx(&state, &mut tx).await?;

    let is_shuffle = state.zmp.setting.is_random_play(&mut tx).await;
    let is_repeat = state.zmp.setting.is_repeat_flag(&mut tx).await;
    let saved_index = state.zmp.setting.get_saved_index(&mut tx).await;
    let saved_seek = state.zmp.setting.get_current_song_seek(&mut tx).await;

    let saved_play_pause_flag = state.zmp.setting.is_playing(&mut tx).await;

    let count = songs.len();

    tx.commit().await.map_err(|e| e.to_string())?;

    let current_index = {
        let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        player
            .load_saved_state(
                is_shuffle,
                is_repeat,
                saved_index,
                saved_seek,
                saved_play_pause_flag,
                songs,
            )
            .map_err(|e| e.to_string())?
    };

    persist_loaded_state(&state.zmp.setting, current_index).await?;
    emit_track_changed(&app, current_index)?;
    Ok(count)
}

#[tauri::command]
pub async fn get_music_folder_path(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let path = state
        .zmp
        .setting
        .get_music_folder_path(&state.zmp.pool)
        .await
        .unwrap_or_default();

    Ok(path)
}

#[tauri::command]
pub async fn set_music_folder_path(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let result = state
        .zmp
        .setting
        .set_music_folder_path(&state.zmp.pool, &path)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub async fn has_processed_music_folder(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let path = state
        .zmp
        .setting
        .has_processed_music_folder(&state.zmp.pool)
        .await;

    Ok(path)
}

#[tauri::command]
pub async fn set_processed_music_folder(
    state: tauri::State<'_, AppState>,
    flag: bool,
) -> Result<(), String> {
    let result = state
        .zmp
        .setting
        .set_processed_music_folder(&state.zmp.pool, flag)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub async fn get_saved_search_blob(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state
        .zmp
        .setting
        .get_saved_search_blob(&state.zmp.pool)
        .await
        .unwrap_or_default())
}

#[tauri::command]
pub async fn get_song_list_limit(state: tauri::State<'_, AppState>) -> Result<i32, String> {
    Ok(state.zmp.setting.get_song_list_limit(&state.zmp.pool).await)
}

#[tauri::command]
pub async fn set_song_list_limit(
    state: tauri::State<'_, AppState>,
    limit: i32,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_song_list_limit(&state.zmp.pool, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_current_index(state: tauri::State<'_, AppState>) -> Result<Option<usize>, String> {
    let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    Ok(player.current_index())
}

#[tauri::command]
pub async fn get_current_song(
    state: tauri::State<'_, AppState>,
) -> Result<Option<SongWithFilters>, String> {
    let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    Ok(player.current_song().cloned())
}

#[tauri::command]
pub async fn get_loaded_songs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SongWithFilters>, String> {
    let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    let songs = player.queue().to_vec();

    Ok(songs)
}

#[tauri::command]
pub async fn get_is_player_paused(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    Ok(player.is_paused())
}

#[tauri::command]
pub async fn play_song_at(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    index: usize,
) -> Result<(), String> {
    let current_index = {
        let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        player
            .play_song_at(index, true, true)
            .map_err(|e| e.to_string())?;
        player.current_index()
    };

    persist_started_track(&state.zmp.setting, current_index).await?;
    emit_track_changed(&app, current_index)?;
    Ok(())
}

#[tauri::command]
pub async fn search_songs(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<usize, String> {
    let trimmed = query.trim();

    state
        .zmp
        .setting
        .set_saved_search_blob(&state.zmp.pool, trimmed)
        .await
        .map_err(|e| e.to_string())?;
    let song_list_limit = state.zmp.setting.get_song_list_limit(&state.zmp.pool).await;

    let songs = query_song_list(&state, &state.zmp.pool, trimmed, song_list_limit).await?;

    let count = songs.len();
    let sync = sync_song_list(&state, songs)?;

    persist_queue_sync(&state.zmp.setting, sync).await?;
    emit_track_changed(&app, sync.current_index)?;
    Ok(count)
}

#[tauri::command]
pub async fn preview_search_songs(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<Vec<SongWithFilters>, String> {
    let trimmed = query.trim();
    let song_list_limit = state.zmp.setting.get_song_list_limit(&state.zmp.pool).await;

    query_song_list(&state, &state.zmp.pool, trimmed, song_list_limit).await
}

#[tauri::command]
pub async fn next_song(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let current_index = {
        let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        player.next_song().map_err(|e| e.to_string())?;
        player.current_index()
    };

    persist_started_track(&state.zmp.setting, current_index).await?;
    emit_track_changed(&app, current_index)?;
    Ok(())
}

#[tauri::command]
pub async fn previous_song(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let current_index = {
        let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        player.previous().map_err(|e| e.to_string())?;
        player.current_index()
    };

    persist_started_track(&state.zmp.setting, current_index).await?;
    emit_track_changed(&app, current_index)?;
    Ok(())
}

keybind_commands!(
    (
        get_focus_search_keybind,
        set_focus_search_keybind,
        get_focus_search_keybind,
        set_focus_search_keybind
    ),
    (
        get_settings_keybind,
        set_settings_keybind,
        get_settings_keybind,
        set_settings_keybind
    ),
    (
        get_mute_keybind,
        set_mute_keybind,
        get_mute_keybind,
        set_mute_keybind
    ),
    (
        get_shuffle_keybind,
        set_shuffle_keybind,
        get_shuffle_keybind,
        set_shuffle_keybind
    ),
    (
        get_repeat_keybind,
        set_repeat_keybind,
        get_repeat_keybind,
        set_repeat_keybind
    ),
    (
        get_next_keybind,
        set_next_keybind,
        get_next_keybind,
        set_next_keybind
    ),
    (
        get_previous_keybind,
        set_previous_keybind,
        get_previous_keybind,
        set_previous_keybind
    ),
    (
        get_play_pause_keybind,
        set_play_pause_keybind,
        get_play_pause_keybind,
        set_play_pause_keybind
    ),
);

#[tauri::command]
pub async fn get_current_song_seek(state: tauri::State<'_, AppState>) -> Result<usize, String> {
    let value = state
        .zmp
        .setting
        .get_current_song_seek(&state.zmp.pool)
        .await;

    Ok(value)
}

#[tauri::command]
pub async fn save_current_song_seek(
    state: tauri::State<'_, AppState>,
    seek_value: usize,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_current_song_seek(&state.zmp.pool, seek_value)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn set_current_song_seek(
    state: tauri::State<'_, AppState>,
    seek_value: usize,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_current_song_seek(&state.zmp.pool, seek_value)
        .await
        .map_err(|e| e.to_string())?;

    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player
        .seek_to_seconds(seek_value as u64)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_play_pause(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let is_playing = state.zmp.setting.is_playing(&state.zmp.pool).await;

    Ok(is_playing)
}

#[tauri::command]
pub async fn set_play_pause(
    state: tauri::State<'_, AppState>,
    is_playing: bool,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_play_pause_flag(&state.zmp.pool, is_playing)
        .await
        .map_err(|e| e.to_string())?;

    let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.play_pause(is_playing);

    Ok(())
}

#[tauri::command]
pub async fn get_volume(state: tauri::State<'_, AppState>) -> Result<rodio::Float, String> {
    let volume = state
        .zmp
        .setting
        .get_saved_volume_value(&state.zmp.pool)
        .await;

    Ok(volume)
}

#[tauri::command]
pub async fn set_volume(
    state: tauri::State<'_, AppState>,
    volume: rodio::Float,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_saved_volume_value(&state.zmp.pool, volume)
        .await
        .map_err(|e| e.to_string())?;

    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.set_volume(volume);

    Ok(())
}

#[tauri::command]
pub async fn get_repeat(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let value = state.zmp.setting.is_repeat_flag(&state.zmp.pool).await;
    Ok(value)
}

#[tauri::command]
pub async fn set_repeat(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let is_repeat = !state.zmp.setting.is_repeat_flag(&state.zmp.pool).await;

    state
        .zmp
        .setting
        .set_repeat_flag(&state.zmp.pool, is_repeat)
        .await
        .map_err(|e| e.to_string())?;

    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.set_repeat(is_repeat);

    Ok(())
}

#[tauri::command]
pub async fn get_random(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    let value = state.zmp.setting.is_random_play(&state.zmp.pool).await;
    Ok(value)
}

#[tauri::command]
pub async fn set_random(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let is_random = !state.zmp.setting.is_random_play(&state.zmp.pool).await;

    state
        .zmp
        .setting
        .set_random_play(&state.zmp.pool, is_random)
        .await
        .map_err(|e| e.to_string())?;

    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.set_shuffle(is_random);

    Ok(())
}

#[tauri::command]
pub async fn create_filter(
    state: tauri::State<'_, AppState>,
    filter_name: String,
) -> Result<bool, String> {
    let result = state.zmp.filter.add(&state.zmp.pool, &filter_name).await;

    match result {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn remove_filter(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    filter_id: i32,
) -> Result<bool, String> {
    let mut tx = state.zmp.pool.begin().await.map_err(|e| e.to_string())?;

    let removed = state
        .zmp
        .filter
        .remove(&mut tx, filter_id)
        .await
        .map_err(|e| e.to_string())?;

    if removed {
        state
            .zmp
            .song_mutation
            .refresh_all_song_search_blobs(&mut tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    let songs = if removed {
        Some(load_song_list_in_tx(&state, &mut tx).await?)
    } else {
        None
    };

    tx.commit().await.map_err(|e| e.to_string())?;

    if let Some(songs) = songs {
        let sync = sync_song_list(&state, songs)?;
        persist_queue_sync(&state.zmp.setting, sync).await?;
        emit_track_changed(&app, sync.current_index)?;
    }

    Ok(removed)
}

#[tauri::command]
pub async fn get_filters(state: tauri::State<'_, AppState>) -> Result<Vec<Filter>, String> {
    state
        .zmp
        .filter
        .get_all(&state.zmp.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_filters_for_song(
    state: tauri::State<'_, AppState>,
    song_id: i32,
) -> Result<Vec<SongFilter>, String> {
    state
        .zmp
        .song_filter
        .get_by_song(&state.zmp.pool, song_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_filter_to_song(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    song_id: i32,
    filter_id: i32,
) -> Result<bool, String> {
    let mut tx = state.zmp.pool.begin().await.map_err(|e| e.to_string())?;

    let saved = state
        .zmp
        .song_mutation
        .add_filter_to_song_and_reindex(&mut tx, song_id, filter_id)
        .await
        .map_err(|e| e.to_string())?;

    let songs = if saved {
        Some(load_song_list_in_tx(&state, &mut tx).await?)
    } else {
        None
    };

    tx.commit().await.map_err(|e| e.to_string())?;

    if let Some(songs) = songs {
        let sync = sync_song_list(&state, songs)?;
        persist_queue_sync(&state.zmp.setting, sync).await?;
        emit_track_changed(&app, sync.current_index)?;
    }

    Ok(saved)
}

#[tauri::command]
pub async fn remove_filter_from_song(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    song_filter_id: i32,
) -> Result<bool, String> {
    let mut tx = state.zmp.pool.begin().await.map_err(|e| e.to_string())?;

    let removed = state
        .zmp
        .song_mutation
        .remove_filter_from_song_and_reindex(&mut tx, song_filter_id)
        .await
        .map_err(|e| e.to_string())?;

    let songs = if removed {
        Some(load_song_list_in_tx(&state, &mut tx).await?)
    } else {
        None
    };

    tx.commit().await.map_err(|e| e.to_string())?;

    if let Some(songs) = songs {
        let sync = sync_song_list(&state, songs)?;
        persist_queue_sync(&state.zmp.setting, sync).await?;
        emit_track_changed(&app, sync.current_index)?;
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use sqlx::{Sqlite, SqlitePool};

    use crate::{player::QueueSyncResult, setting::SettingService, sqlite::SqliteDb};

    use super::{persist_loaded_state, persist_queue_sync, persist_started_track};

    async fn setup_setting_service() -> SettingService<SqliteDb, Sqlite> {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        SettingService::new(SqliteDb { pool })
    }

    #[tokio::test]
    async fn persist_started_track_resets_seek_and_marks_playing() {
        let setting = setup_setting_service().await;

        setting
            .set_current_song_seek(&setting.pool, 27)
            .await
            .unwrap();
        setting
            .set_play_pause_flag(&setting.pool, false)
            .await
            .unwrap();

        persist_started_track(&setting, Some(3)).await.unwrap();

        assert_eq!(setting.get_saved_index(&setting.pool).await, 3);
        assert_eq!(setting.get_current_song_seek(&setting.pool).await, 0);
        assert!(setting.is_playing(&setting.pool).await);
    }

    #[tokio::test]
    async fn persist_queue_sync_clears_playback_state_when_current_song_disappears() {
        let setting = setup_setting_service().await;

        setting
            .set_current_song_seek(&setting.pool, 91)
            .await
            .unwrap();
        setting
            .set_play_pause_flag(&setting.pool, true)
            .await
            .unwrap();

        persist_queue_sync(
            &setting,
            QueueSyncResult {
                current_index: None,
                cleared_current_song: true,
            },
        )
        .await
        .unwrap();

        assert_eq!(setting.get_saved_index(&setting.pool).await, 0);
        assert_eq!(setting.get_current_song_seek(&setting.pool).await, 0);
        assert!(!setting.is_playing(&setting.pool).await);
    }

    #[tokio::test]
    async fn persist_loaded_state_preserves_seek_and_play_flag_when_song_is_still_selected() {
        let setting = setup_setting_service().await;

        setting
            .set_current_song_seek(&setting.pool, 42)
            .await
            .unwrap();
        setting
            .set_play_pause_flag(&setting.pool, true)
            .await
            .unwrap();

        persist_loaded_state(&setting, Some(1)).await.unwrap();

        assert_eq!(setting.get_saved_index(&setting.pool).await, 1);
        assert_eq!(setting.get_current_song_seek(&setting.pool).await, 42);
        assert!(setting.is_playing(&setting.pool).await);
    }
}
