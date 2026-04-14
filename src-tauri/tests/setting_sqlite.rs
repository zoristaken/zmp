pub mod common;
use crate::common::setup_db;
use zmp_lib::setting::{SettingService, DEFAULT_SONG_LIST_LIMIT};
use zmp_lib::sqlite::SqliteImpl;

#[tokio::test]
async fn integration_music_folder_path_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service
        .set_music_folder_path(&pool, "/var/lib/music")
        .await
        .unwrap();

    let actual = service.get_music_folder_path(&pool).await.unwrap();
    assert_eq!(actual, "/var/lib/music");
}

#[tokio::test]
async fn integration_repeat_flag_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    assert!(!service.is_repeat_flag(&pool).await);

    service.set_repeat_flag(&pool, true).await.unwrap();
    assert!(service.is_repeat_flag(&pool).await);

    service.set_repeat_flag(&pool, false).await.unwrap();
    assert!(!service.is_repeat_flag(&pool).await);
}

#[tokio::test]
async fn integration_random_play_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    assert!(!service.is_random_play(&pool).await);

    service.set_random_play(&pool, true).await.unwrap();
    assert!(service.is_random_play(&pool).await);

    service.set_random_play(&pool, false).await.unwrap();
    assert!(!service.is_random_play(&pool).await);
}

#[tokio::test]
async fn integration_always_start_paused_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    assert!(!service.should_always_start_paused(&pool).await);

    service.set_always_start_paused(&pool, true).await.unwrap();
    assert!(service.should_always_start_paused(&pool).await);

    service.set_always_start_paused(&pool, false).await.unwrap();
    assert!(!service.should_always_start_paused(&pool).await);
}

#[tokio::test]
async fn integration_processed_music_folder_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    assert!(!service.has_processed_music_folder(&pool).await);

    service
        .set_processed_music_folder(&pool, true)
        .await
        .unwrap();
    assert!(service.has_processed_music_folder(&pool).await);

    service
        .set_processed_music_folder(&pool, false)
        .await
        .unwrap();
    assert!(!service.has_processed_music_folder(&pool).await);
}

#[tokio::test]
async fn integration_saved_volume_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service.set_saved_volume_value(&pool, 0.42).await.unwrap();
    let actual = service.get_saved_volume_value(&pool).await;

    assert!((actual - 0.42).abs() < f32::EPSILON);
}

#[tokio::test]
async fn integration_saved_volume_returns_default_for_invalid_db_value() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    sqlx::query(
        r#"
        INSERT INTO setting (key, value)
        VALUES ('volume_value', 'definitely-not-a-float')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let actual = service.get_saved_volume_value(&pool).await;
    assert_eq!(actual, 0.5);
}

#[tokio::test]
async fn integration_saved_search_blob_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service
        .set_saved_search_blob(&pool, "genre:ambient")
        .await
        .unwrap();

    let actual = service.get_saved_search_blob(&pool).await.unwrap();
    assert_eq!(actual, "genre:ambient");
}

#[tokio::test]
async fn integration_song_list_limit_returns_default_when_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    let actual = service.get_song_list_limit(&pool).await;

    assert_eq!(actual, DEFAULT_SONG_LIST_LIMIT);
}

#[tokio::test]
async fn integration_song_list_limit_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service.set_song_list_limit(&pool, 2500).await.unwrap();

    let actual = service.get_song_list_limit(&pool).await;

    assert_eq!(actual, 2500);
}

#[tokio::test]
async fn integration_song_list_limit_rejects_non_positive_values() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    let zero_error = service.set_song_list_limit(&pool, 0).await.unwrap_err();
    let negative_error = service.set_song_list_limit(&pool, -25).await.unwrap_err();

    assert!(zero_error.to_string().contains("greater than 0"));
    assert!(negative_error.to_string().contains("greater than 0"));
}

#[tokio::test]
async fn integration_keybind_roundtrips() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service.set_settings_keybind(&pool, "Ctrl+,").await.unwrap();
    service
        .set_play_pause_keybind(&pool, "Space")
        .await
        .unwrap();
    service
        .set_previous_keybind(&pool, "Ctrl+Left")
        .await
        .unwrap();
    service.set_next_keybind(&pool, "Ctrl+Right").await.unwrap();
    service.set_shuffle_keybind(&pool, "Ctrl+R").await.unwrap();
    service
        .set_keybind_settings_keybind(&pool, "Ctrl+Shift+K")
        .await
        .unwrap();
    service
        .set_saved_search_blob(&pool, "aphex twin")
        .await
        .unwrap();
    service
        .set_switch_song_filter_pane_keybind(&pool, "Tab")
        .await
        .unwrap();
    service
        .set_apply_selected_filter_keybind(&pool, "Enter")
        .await
        .unwrap();

    assert_eq!(service.get_settings_keybind(&pool).await.unwrap(), "Ctrl+,");
    assert_eq!(
        service.get_play_pause_keybind(&pool).await.unwrap(),
        "Space"
    );
    assert_eq!(
        service.get_previous_keybind(&pool).await.unwrap(),
        "Ctrl+Left"
    );
    assert_eq!(service.get_next_keybind(&pool).await.unwrap(), "Ctrl+Right");
    assert_eq!(service.get_shuffle_keybind(&pool).await.unwrap(), "Ctrl+R");
    assert_eq!(
        service.get_keybind_settings_keybind(&pool).await.unwrap(),
        "Ctrl+Shift+K"
    );
    assert_eq!(
        service
            .get_switch_song_filter_pane_keybind(&pool)
            .await
            .unwrap(),
        "Tab"
    );
    assert_eq!(
        service
            .get_apply_selected_filter_keybind(&pool)
            .await
            .unwrap(),
        "Enter"
    );
    assert_eq!(
        service.get_saved_search_blob(&pool).await.unwrap(),
        "aphex twin"
    );
}

#[tokio::test]
async fn integration_app_settings_snapshot_batches_saved_values() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service
        .set_music_folder_path(&pool, "/music/library")
        .await
        .unwrap();
    service
        .set_processed_music_folder(&pool, true)
        .await
        .unwrap();
    service
        .set_saved_search_blob(&pool, "burial")
        .await
        .unwrap();
    service.set_song_list_limit(&pool, 2500).await.unwrap();
    service.set_always_start_paused(&pool, true).await.unwrap();
    service
        .set_play_pause_keybind(&pool, "Space")
        .await
        .unwrap();
    service.set_filter_menu_keybind(&pool, "F").await.unwrap();

    let snapshot = service.get_app_settings_snapshot(&pool).await.unwrap();

    assert_eq!(snapshot.music_folder_path, "/music/library");
    assert!(snapshot.has_processed_music_folder);
    assert_eq!(snapshot.saved_search_blob, "burial");
    assert_eq!(snapshot.song_list_limit, 2500);
    assert!(snapshot.always_start_paused);
    assert_eq!(snapshot.keybinds.play_pause, "Space");
    assert_eq!(snapshot.keybinds.toggle_filter_menu, "F");
}

#[tokio::test]
async fn integration_music_folder_sync_settings_batch_defaults_when_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    let snapshot = service.get_music_folder_sync_settings(&pool).await.unwrap();

    assert_eq!(snapshot.music_folder_path, "");
    assert!(!snapshot.has_processed_music_folder);
}

#[tokio::test]
async fn integration_updates_existing_setting_instead_of_duplicate_insert() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service
        .set_music_folder_path(&pool, "/music/a")
        .await
        .unwrap();
    service
        .set_music_folder_path(&pool, "/music/b")
        .await
        .unwrap();

    let actual = service.get_music_folder_path(&pool).await.unwrap();
    assert_eq!(actual, "/music/b");

    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM setting
        WHERE key = 'music_folder_path'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1);
}

#[tokio::test]
async fn integration_persist_started_track_resets_seek_and_marks_playing() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service.set_current_song_seek(&pool, 27).await.unwrap();
    service.set_play_pause_flag(&pool, false).await.unwrap();

    service.persist_started_track(&pool, Some(3)).await.unwrap();

    assert_eq!(service.get_saved_index(&pool).await, 3);
    assert_eq!(service.get_current_song_seek(&pool).await, 0);
    assert!(service.is_playing(&pool).await);
}

#[tokio::test]
async fn integration_persist_queue_sync_clears_playback_state_when_current_song_disappears() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service.set_current_song_seek(&pool, 91).await.unwrap();
    service.set_play_pause_flag(&pool, true).await.unwrap();

    service.persist_queue_sync(&pool, None, true).await.unwrap();

    assert_eq!(service.get_saved_index(&pool).await, 0);
    assert_eq!(service.get_current_song_seek(&pool).await, 0);
    assert!(!service.is_playing(&pool).await);
}

#[tokio::test]
async fn integration_persist_queue_sync_preserves_seek_and_play_flag_when_song_is_still_selected() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SettingService::new(sqlite);

    service.set_current_song_seek(&pool, 42).await.unwrap();
    service.set_play_pause_flag(&pool, true).await.unwrap();

    service
        .persist_queue_sync(&pool, Some(1), false)
        .await
        .unwrap();

    assert_eq!(service.get_saved_index(&pool).await, 1);
    assert_eq!(service.get_current_song_seek(&pool).await, 42);
    assert!(service.is_playing(&pool).await);
}
