use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{
    errors::AppResult, filter::Filter, song_filter::SongFilter, song_query::SongWithFilters,
    AppState,
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TrackChangedPayload {
    current_index: Option<usize>,
}

fn emit_track_changed(app: &AppHandle, current_index: Option<usize>) -> anyhow::Result<()> {
    app.emit("track-changed", TrackChangedPayload { current_index })?;
    Ok(())
}

macro_rules! keybind_commands {
    ($(($getter:ident, $setter:ident)),+ $(,)?) => {
        $(
            #[tauri::command]
            pub async fn $getter(
                state: tauri::State<'_, AppState>,
            ) -> AppResult<Option<String>> {
                Ok(state.zmp.$getter().await)
            }

            #[tauri::command]
            pub async fn $setter(
                state: tauri::State<'_, AppState>,
                keybind: String,
            ) -> AppResult<()> {
                state.zmp.$setter(&keybind).await?;
                Ok(())
            }
        )+
    };
}

//macro generation for #[tauri::command] keybinds
keybind_commands!(
    (get_focus_search_keybind, set_focus_search_keybind),
    (get_settings_keybind, set_settings_keybind),
    (get_mute_keybind, set_mute_keybind),
    (get_shuffle_keybind, set_shuffle_keybind),
    (get_repeat_keybind, set_repeat_keybind),
    (get_next_keybind, set_next_keybind),
    (get_previous_keybind, set_previous_keybind),
    (get_play_pause_keybind, set_play_pause_keybind),
    (get_increase_volume_keybind, set_increase_volume_keybind),
    (get_decrease_volume_keybind, set_decrease_volume_keybind),
    (get_seek_forward_keybind, set_seek_forward_keybind),
    (get_seek_backward_keybind, set_seek_backward_keybind),
    (get_filter_menu_keybind, set_filter_menu_keybind),
    (get_song_filter_menu_keybind, set_song_filter_menu_keybind),
    (get_keybind_settings_keybind, set_keybind_settings_keybind),
    (
        get_switch_song_filter_pane_keybind,
        set_switch_song_filter_pane_keybind
    ),
    (
        get_apply_selected_filter_keybind,
        set_apply_selected_filter_keybind
    ),
);

#[tauri::command]
pub async fn process_music_folder(state: tauri::State<'_, AppState>) -> AppResult<()> {
    state.zmp.process_music_folder().await?;
    Ok(())
}

#[tauri::command]
pub async fn load(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> AppResult<usize> {
    let result = state.zmp.load().await?;
    emit_track_changed(&app, result.current_index)?;
    Ok(result.count)
}

#[tauri::command]
pub async fn get_music_folder_path(state: tauri::State<'_, AppState>) -> AppResult<String> {
    Ok(state.zmp.get_music_folder_path().await)
}

#[tauri::command]
pub async fn set_music_folder_path(
    state: tauri::State<'_, AppState>,
    path: String,
) -> AppResult<()> {
    state.zmp.set_music_folder_path(&path).await?;
    Ok(())
}

#[tauri::command]
pub async fn has_processed_music_folder(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    Ok(state.zmp.has_processed_music_folder().await)
}

#[tauri::command]
pub async fn set_processed_music_folder(
    state: tauri::State<'_, AppState>,
    flag: bool,
) -> AppResult<()> {
    state.zmp.set_processed_music_folder(flag).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_saved_search_blob(state: tauri::State<'_, AppState>) -> AppResult<String> {
    Ok(state.zmp.get_saved_search_blob().await)
}

#[tauri::command]
pub async fn get_song_list_limit(state: tauri::State<'_, AppState>) -> AppResult<i32> {
    Ok(state.zmp.get_song_list_limit().await)
}

#[tauri::command]
pub async fn set_song_list_limit(state: tauri::State<'_, AppState>, limit: i32) -> AppResult<()> {
    state.zmp.set_song_list_limit(limit).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_current_index(state: tauri::State<'_, AppState>) -> AppResult<Option<usize>> {
    Ok(state.zmp.get_current_index()?)
}

#[tauri::command]
pub async fn get_current_song(
    state: tauri::State<'_, AppState>,
) -> AppResult<Option<SongWithFilters>> {
    Ok(state.zmp.get_current_song()?)
}

#[tauri::command]
pub async fn get_loaded_songs(
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<SongWithFilters>> {
    Ok(state.zmp.get_loaded_songs()?)
}

#[tauri::command]
pub async fn get_is_player_paused(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    Ok(state.zmp.get_is_player_paused()?)
}

#[tauri::command]
pub async fn play_song_at(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    index: usize,
) -> AppResult<()> {
    let result = state.zmp.play_song_at(index).await?;
    emit_track_changed(&app, result.current_index)?;
    Ok(())
}

#[tauri::command]
pub async fn commit_preview_search(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    query: String,
) -> AppResult<usize> {
    let result = state.zmp.commit_preview_search(&query).await?;
    emit_track_changed(&app, result.current_index)?;
    Ok(result.count)
}

#[tauri::command]
pub async fn preview_search_songs(
    state: tauri::State<'_, AppState>,
    query: String,
) -> AppResult<Vec<SongWithFilters>> {
    Ok(state.zmp.preview_search_songs(&query).await?)
}

#[tauri::command]
pub async fn next_song(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> AppResult<()> {
    let result = state.zmp.next_song().await?;
    emit_track_changed(&app, result.current_index)?;
    Ok(())
}

#[tauri::command]
pub async fn previous_song(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    let result = state.zmp.previous_song().await?;
    emit_track_changed(&app, result.current_index)?;
    Ok(())
}

#[tauri::command]
pub async fn get_current_song_seek(state: tauri::State<'_, AppState>) -> AppResult<usize> {
    Ok(state.zmp.get_current_song_seek().await)
}

#[tauri::command]
pub async fn save_current_song_seek(
    state: tauri::State<'_, AppState>,
    seek_value: usize,
) -> AppResult<()> {
    state.zmp.save_current_song_seek(seek_value).await?;
    Ok(())
}

#[tauri::command]
pub async fn set_current_song_seek(
    state: tauri::State<'_, AppState>,
    seek_value: usize,
) -> AppResult<()> {
    state.zmp.set_current_song_seek(seek_value).await?;
    Ok(())
}

#[tauri::command]
pub async fn increase_current_song_seek_by_seconds(
    state: tauri::State<'_, AppState>,
    seek_value: u64,
) -> AppResult<()> {
    state
        .zmp
        .increase_current_song_seek_by_seconds(seek_value)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn decrease_current_song_seek_by_seconds(
    state: tauri::State<'_, AppState>,
    seek_value: u64,
) -> AppResult<()> {
    state
        .zmp
        .decrease_current_song_seek_by_seconds(seek_value)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn get_play_pause(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    Ok(state.zmp.get_play_pause().await)
}

#[tauri::command]
pub async fn set_play_pause(state: tauri::State<'_, AppState>, is_playing: bool) -> AppResult<()> {
    state.zmp.set_play_pause(is_playing).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_always_start_paused(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    Ok(state.zmp.get_always_start_paused().await)
}

#[tauri::command]
pub async fn set_always_start_paused(
    state: tauri::State<'_, AppState>,
    flag: bool,
) -> AppResult<()> {
    state.zmp.set_always_start_paused(flag).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_volume(state: tauri::State<'_, AppState>) -> AppResult<rodio::Float> {
    Ok(state.zmp.get_volume().await)
}

#[tauri::command]
pub async fn set_volume(state: tauri::State<'_, AppState>, volume: rodio::Float) -> AppResult<()> {
    state.zmp.set_volume(volume).await?;
    Ok(())
}

#[tauri::command]
pub async fn increase_volume_by(
    state: tauri::State<'_, AppState>,
    volume: rodio::Float,
) -> AppResult<()> {
    state.zmp.increase_volume_by(volume).await?;
    Ok(())
}

#[tauri::command]
pub async fn decrease_volume_by(
    state: tauri::State<'_, AppState>,
    volume: rodio::Float,
) -> AppResult<()> {
    state.zmp.decrease_volume_by(volume).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_repeat(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    Ok(state.zmp.get_repeat().await)
}

#[tauri::command]
pub async fn set_repeat(state: tauri::State<'_, AppState>) -> AppResult<()> {
    state.zmp.set_repeat().await?;
    Ok(())
}

#[tauri::command]
pub async fn get_random(state: tauri::State<'_, AppState>) -> AppResult<bool> {
    Ok(state.zmp.get_random().await)
}

#[tauri::command]
pub async fn set_random(state: tauri::State<'_, AppState>) -> AppResult<()> {
    state.zmp.set_random().await?;
    Ok(())
}

#[tauri::command]
pub async fn create_filter(
    state: tauri::State<'_, AppState>,
    filter_name: String,
) -> AppResult<bool> {
    Ok(state.zmp.create_filter(&filter_name).await)
}

#[tauri::command]
pub async fn remove_filter(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    filter_id: i32,
) -> AppResult<bool> {
    let result = state.zmp.remove_filter(filter_id).await?;

    if result.changed {
        emit_track_changed(&app, result.current_index)?;
    }

    Ok(result.changed)
}

#[tauri::command]
pub async fn get_filters(state: tauri::State<'_, AppState>) -> AppResult<Vec<Filter>> {
    Ok(state.zmp.get_filters().await?)
}

#[tauri::command]
pub async fn get_filters_for_song(
    state: tauri::State<'_, AppState>,
    song_id: i32,
) -> AppResult<Vec<SongFilter>> {
    Ok(state.zmp.get_filters_for_song(song_id).await?)
}

#[tauri::command]
pub async fn add_filter_to_song(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    song_id: i32,
    filter_id: i32,
) -> AppResult<bool> {
    let result = state.zmp.add_filter_to_song(song_id, filter_id).await?;

    if result.changed {
        emit_track_changed(&app, result.current_index)?;
    }

    Ok(result.changed)
}

#[tauri::command]
pub async fn remove_filter_from_song(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    song_filter_id: i32,
) -> AppResult<bool> {
    let result = state.zmp.remove_filter_from_song(song_filter_id).await?;

    if result.changed {
        emit_track_changed(&app, result.current_index)?;
    }

    Ok(result.changed)
}
