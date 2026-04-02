mod config;
mod filter;
mod metadata;
mod player;
mod setting;
mod song;
mod song_filter;
mod sqlite;
mod test;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![test::main_test])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
