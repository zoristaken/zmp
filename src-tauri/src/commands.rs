use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{manager::AppState, song::Song};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackChangedPayload {
    current_index: Option<usize>,
}

fn emit_track_changed(app: &AppHandle, current_index: Option<usize>) -> Result<(), String> {
    app.emit("track-changed", TrackChangedPayload { current_index })
        .map_err(|e| e.to_string())
}

//TODO: temporary mock of what a setup would look like
#[tauri::command]
pub async fn init(state: tauri::State<'_, AppState>) -> Result<(), String> {
    //select music folder path
    state
        .zmp
        .setting
        .set_music_folder_path(&state.zmp.pool, "/home/z/Music")
        .await
        .map_err(|e| e.to_string())?;

    //...?
    //process music folder
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
    let saved_search = state
        .zmp
        .setting
        .get_saved_search_blob(&state.zmp.pool)
        .await
        .unwrap_or_default();

    let songs = if saved_search.trim().is_empty() {
        state
            .zmp
            .song
            .list_songs(&state.zmp.pool)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let words: Vec<&str> = saved_search.split_whitespace().collect();
        state
            .zmp
            .song
            .search_by_db(&state.zmp.pool, &words, 100)
            .await
            .map_err(|e| e.to_string())?
    };

    let is_random = state.zmp.setting.is_random_play(&state.zmp.pool).await;
    let is_repeat = state.zmp.setting.is_repeat_flag(&state.zmp.pool).await;
    let saved_index = state.zmp.setting.get_saved_index(&state.zmp.pool).await;
    let saved_seek = state
        .zmp
        .setting
        .get_current_song_seek(&state.zmp.pool)
        .await;

    let count = songs.len();

    {
        let mut stored = state
            .loaded_songs
            .lock()
            .map_err(|_| "failed to lock state".to_string())?;
        *stored = songs.clone();
    }

    let current_index = {
        let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        player.set_shuffle(is_random);
        player.set_repeat(is_repeat);
        player.set_queue(songs.clone()).map_err(|e| e.to_string())?;

        if !songs.is_empty() {
            let index = saved_index.min(songs.len() - 1);
            player.play_song_at(index).map_err(|e| e.to_string())?;

            if saved_seek > 0 {
                player
                    .seek_to_seconds(saved_seek as u64)
                    .map_err(|e| e.to_string())?;
            }
        }

        player.current_index()
    };

    emit_track_changed(&app, current_index)?;
    Ok(count)
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
pub async fn get_current_song(state: tauri::State<'_, AppState>) -> Result<Option<Song>, String> {
    let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    Ok(player.current_song().cloned())
}

#[tauri::command]
pub async fn get_loaded_songs(state: tauri::State<'_, AppState>) -> Result<Vec<Song>, String> {
    let songs = state
        .loaded_songs
        .lock()
        .map_err(|_| "failed to lock loaded songs".to_string())?
        .clone();

    Ok(songs)
}

#[tauri::command]
pub async fn get_is_paused(state: tauri::State<'_, AppState>) -> Result<bool, String> {
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
        player.play_song_at(index).map_err(|e| e.to_string())?;
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

    let songs = if trimmed.is_empty() {
        state
            .zmp
            .song
            .list_songs(&state.zmp.pool)
            .await
            .map_err(|e| e.to_string())?
    } else {
        let words: Vec<&str> = trimmed.split_whitespace().collect();

        state
            .zmp
            .song
            .search_by_db(&state.zmp.pool, &words, 100)
            .await
            .map_err(|e| e.to_string())?
    };

    let count = songs.len();

    {
        let mut stored = state
            .loaded_songs
            .lock()
            .map_err(|_| "failed to lock loaded songs".to_string())?;
        *stored = songs.clone();
    }

    let current_index = {
        let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        player.set_queue(songs).map_err(|e| e.to_string())?;
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
    Ok(count)
}

#[tauri::command]
pub async fn next_song(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let current_index = {
        let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
        player.next().map_err(|e| e.to_string())?;
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
pub async fn play_pause(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.play_pause();
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
    if is_playing {
        player.play();
    } else {
        player.pause();
    }

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
