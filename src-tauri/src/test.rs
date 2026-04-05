use std::sync::atomic::Ordering;

use crate::manager::AppState;

#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    message: String,
}

impl From<anyhow::Error> for ErrorResponse {
    fn from(e: anyhow::Error) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

#[tauri::command]
pub async fn main_test(state: tauri::State<'_, AppState>) -> anyhow::Result<(), ErrorResponse> {
    state.zmp.process_music_folder().await?;

    if state.first_time.load(Ordering::SeqCst) {
        state.first_time.store(false, Ordering::SeqCst);
        let songs = state.zmp.song.list_songs(&state.zmp.pool.clone()).await?;
        state.zmp.player.fresh_queue(songs)?;
    } else {
        println!("AppState addr: {:p}", &*state);
        state.zmp.player.next();
    }
    Ok(())
}

//TODO:
// read modules
// add actual error logic
// build a proper player struct
// add frontend and tauri commands
// add actual missing backend -> frontend implementations
