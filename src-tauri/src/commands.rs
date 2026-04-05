use crate::manager::AppState;

// #[tauri::command]
// pub async fn first_config() {}

#[tauri::command]
pub async fn init(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .zmp
        .process_music_folder()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

//TODO: receive string from frontend, loading everything for now
#[tauri::command]
pub async fn load(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let songs = state
        .zmp
        .song
        .list_songs(&state.zmp.pool.clone())
        .await
        .map_err(|e| e.to_string())?;

    let mut stored = state
        .loaded_songs
        .lock()
        .map_err(|_| "failed to lock state")?;

    *stored = songs;
    drop(stored);

    let songs = state
        .loaded_songs
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.set_queue(songs).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn next_song(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;

    player.next().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn play_pause(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.play_pause();

    Ok(())
}

#[tauri::command]
pub async fn previous_song(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.previous().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn set_volume(
    state: tauri::State<'_, AppState>,
    volume: rodio::Float,
) -> Result<(), String> {
    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.set_volume(volume);

    Ok(())
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
pub async fn set_repeat(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let is_repeat = !state.zmp.setting.is_repeat_flag(&state.zmp.pool).await;

    state
        .zmp
        .setting
        .set_repeat_flag(&state.zmp.pool, is_repeat)
        .await
        .map_err(|e| e.to_string())?;

    let mut player = state.zmp.player.lock().map_err(|e| e.to_string())?;
    player.toggle_repeat_one().map_err(|e| e.to_string())?;

    Ok(())
}
