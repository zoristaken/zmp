use serde::Serialize;
use sqlx::Sqlite;
use tauri::{AppHandle, Emitter};

use crate::{
    filter::Filter, manager::AppState, song_filter::SongFilter, song_query::SongWithFilters,
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

async fn refresh_loaded_song_state_in_tx(
    state: &AppState,
    tx: &mut sqlx::Transaction<'_, Sqlite>,
) -> Result<(), String> {
    let saved_search = state
        .zmp
        .setting
        .get_saved_search_blob(&mut *tx)
        .await
        .unwrap_or_default();

    let songs: Vec<SongWithFilters> = if saved_search.trim().is_empty() {
        state
            .zmp
            .song_query
            .list_songs_with_filters(&mut *tx)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let words: Vec<&str> = saved_search.split_whitespace().collect();

        state
            .zmp
            .song_query
            .search_by_db_with_filters(&mut *tx, &words, 10000)
            .await
            .map_err(|e| e.to_string())?
    };

    {
        let mut stored = state
            .loaded_songs
            .lock()
            .map_err(|_| "failed to lock loaded songs".to_string())?;

        *stored = songs;
    }

    Ok(())
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

    let saved_search = state
        .zmp
        .setting
        .get_saved_search_blob(&mut tx)
        .await
        .unwrap_or_default();

    let songs = if saved_search.trim().is_empty() {
        state
            .zmp
            .song_query
            .list_songs_with_filters(&mut tx)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let words: Vec<&str> = saved_search.split_whitespace().collect();
        state
            .zmp
            .song_query
            .search_by_db_with_filters(&mut tx, &words, 100)
            .await
            .map_err(|e| e.to_string())?
    };

    let is_shuffle = state.zmp.setting.is_random_play(&mut tx).await;
    let is_repeat = state.zmp.setting.is_repeat_flag(&mut tx).await;
    let saved_index = state.zmp.setting.get_saved_index(&mut tx).await;
    let saved_seek = state.zmp.setting.get_current_song_seek(&mut tx).await;

    let saved_play_pause_flag = state.zmp.setting.is_playing(&mut tx).await;

    let count = songs.len();

    {
        let mut stored = state
            .loaded_songs
            .lock()
            .map_err(|_| "failed to lock state".to_string())?;
        *stored = songs.clone();
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    let current_index = player
        .load_saved_state(
            is_shuffle,
            is_repeat,
            saved_index,
            saved_seek,
            saved_play_pause_flag,
            songs,
        )
        .map_err(|e| e.to_string())?;

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
        .map_err(|e| e.to_string())?;

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
    state
        .zmp
        .setting
        .get_saved_search_blob(&state.zmp.pool)
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
    let songs = state
        .loaded_songs
        .lock()
        .map_err(|_| "failed to lock loaded songs".to_string())?
        .clone();

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
    let songs = state
        .loaded_songs
        .lock()
        .map_err(|_| "failed to lock loaded songs".to_string())?
        .clone();

    let current_index = {
        let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        player.set_queue(songs).map_err(|e| e.to_string())?;
        player
            .play_song_at(index, true, true)
            .map_err(|e| e.to_string())?;
        player.current_index()
    };

    state
        .zmp
        .setting
        .set_saved_index(&state.zmp.pool, current_index.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())?;

    state
        .zmp
        .setting
        .set_current_song_seek(&state.zmp.pool, 0)
        .await
        .map_err(|e| e.to_string())?;

    emit_track_changed(&app, current_index)?;
    Ok(())
}

#[tauri::command]
pub async fn search_songs(
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

    let songs = if trimmed.is_empty() {
        state
            .zmp
            .song_query
            .list_songs_with_filters(&state.zmp.pool)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let words: Vec<&str> = trimmed.split_whitespace().collect();

        state
            .zmp
            .song_query
            .search_by_db_with_filters(&state.zmp.pool, &words, 10000)
            .await
            .map_err(|e| e.to_string())?
    };

    let count = songs.len();

    {
        let mut stored = state
            .loaded_songs
            .lock()
            .map_err(|_| "failed to lock loaded songs".to_string())?;
        *stored = songs;
    }

    Ok(count)
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

    state
        .zmp
        .setting
        .set_saved_index(&state.zmp.pool, current_index.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())?;

    state
        .zmp
        .setting
        .set_current_song_seek(&state.zmp.pool, 0)
        .await
        .map_err(|e| e.to_string())?;

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

    state
        .zmp
        .setting
        .set_saved_index(&state.zmp.pool, current_index.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())?;

    state
        .zmp
        .setting
        .set_current_song_seek(&state.zmp.pool, 0)
        .await
        .map_err(|e| e.to_string())?;

    emit_track_changed(&app, current_index)?;
    Ok(())
}

#[tauri::command]
pub async fn get_focus_search_keybind(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let value = state
        .zmp
        .setting
        .get_focus_search_keybind(&state.zmp.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(value)
}

#[tauri::command]
pub async fn set_focus_search_keybind(
    state: tauri::State<'_, AppState>,
    keybind: String,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_focus_search_keybind(&state.zmp.pool, &keybind)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_settings_keybind(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let value = state
        .zmp
        .setting
        .get_settings_keybind(&state.zmp.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(value)
}

#[tauri::command]
pub async fn set_settings_keybind(
    state: tauri::State<'_, AppState>,
    keybind: String,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_settings_keybind(&state.zmp.pool, &keybind)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_mute_keybind(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let value = state
        .zmp
        .setting
        .get_mute_keybind(&state.zmp.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(value)
}

#[tauri::command]
pub async fn set_mute_keybind(
    state: tauri::State<'_, AppState>,
    keybind: String,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_mute_keybind(&state.zmp.pool, &keybind)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_shuffle_keybind(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let value = state
        .zmp
        .setting
        .get_shuffle_keybind(&state.zmp.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(value)
}

#[tauri::command]
pub async fn set_shuffle_keybind(
    state: tauri::State<'_, AppState>,
    keybind: String,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_shuffle_keybind(&state.zmp.pool, &keybind)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_repeat_keybind(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let value = state
        .zmp
        .setting
        .get_repeat_keybind(&state.zmp.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(value)
}

#[tauri::command]
pub async fn set_repeat_keybind(
    state: tauri::State<'_, AppState>,
    keybind: String,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_repeat_keybind(&state.zmp.pool, &keybind)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_next_keybind(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let value = state
        .zmp
        .setting
        .get_next_keybind(&state.zmp.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(value)
}

#[tauri::command]
pub async fn set_next_keybind(
    state: tauri::State<'_, AppState>,
    keybind: String,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_next_keybind(&state.zmp.pool, &keybind)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_previous_keybind(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let value = state
        .zmp
        .setting
        .get_previous_keybind(&state.zmp.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(value)
}

#[tauri::command]
pub async fn set_previous_keybind(
    state: tauri::State<'_, AppState>,
    keybind: String,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_previous_keybind(&state.zmp.pool, &keybind)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_play_pause_keybind(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let value = state
        .zmp
        .setting
        .get_play_pause_keybind(&state.zmp.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(value)
}

#[tauri::command]
pub async fn set_play_pause_keybind(
    state: tauri::State<'_, AppState>,
    keybind: String,
) -> Result<(), String> {
    state
        .zmp
        .setting
        .set_play_pause_keybind(&state.zmp.pool, &keybind)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

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

        refresh_loaded_song_state_in_tx(&state, &mut tx).await?
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    if removed {
        let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        emit_track_changed(&app, player.current_index())?;
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

    if saved {
        refresh_loaded_song_state_in_tx(&state, &mut tx).await?
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    if saved {
        let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        emit_track_changed(&app, player.current_index())?;
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

    if removed {
        refresh_loaded_song_state_in_tx(&state, &mut tx).await?
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    if removed {
        let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        emit_track_changed(&app, player.current_index())?;
    }

    Ok(removed)
}
