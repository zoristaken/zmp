pub mod common;
use crate::common::setup_db;
use zmp_lib::setting::{SettingService, DEFAULT_SONG_LIST_LIMIT};
use zmp_lib::sqlite::SqliteDb;

#[tokio::test]
async fn integration_music_folder_path_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    service
        .set_music_folder_path(&service.pool, "/var/lib/music")
        .await
        .unwrap();

    let actual = service.get_music_folder_path(&service.pool).await.unwrap();
    assert_eq!(actual, "/var/lib/music");
}

#[tokio::test]
async fn integration_repeat_flag_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    assert!(!service.is_repeat_flag(&service.pool).await);

    service.set_repeat_flag(&service.pool, true).await.unwrap();
    assert!(service.is_repeat_flag(&service.pool).await);

    service.set_repeat_flag(&service.pool, false).await.unwrap();
    assert!(!service.is_repeat_flag(&service.pool).await);
}

#[tokio::test]
async fn integration_random_play_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    assert!(!service.is_random_play(&service.pool).await);

    service.set_random_play(&service.pool, true).await.unwrap();
    assert!(service.is_random_play(&service.pool).await);

    service.set_random_play(&service.pool, false).await.unwrap();
    assert!(!service.is_random_play(&service.pool).await);
}

#[tokio::test]
async fn integration_processed_music_folder_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    assert!(!service.has_processed_music_folder(&service.pool).await);

    service
        .set_processed_music_folder(&service.pool, true)
        .await
        .unwrap();
    assert!(service.has_processed_music_folder(&service.pool).await);

    service
        .set_processed_music_folder(&service.pool, false)
        .await
        .unwrap();
    assert!(!service.has_processed_music_folder(&service.pool).await);
}

#[tokio::test]
async fn integration_saved_volume_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    service
        .set_saved_volume_value(&service.pool, 0.42)
        .await
        .unwrap();
    let actual = service.get_saved_volume_value(&service.pool).await;

    assert!((actual - 0.42).abs() < f32::EPSILON);
}

#[tokio::test]
async fn integration_saved_volume_returns_default_for_invalid_db_value() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    sqlx::query(
        r#"
        INSERT INTO setting (key, value)
        VALUES ('volume_value', 'definitely-not-a-float')
        "#,
    )
    .execute(&service.pool)
    .await
    .unwrap();

    let actual = service.get_saved_volume_value(&service.pool).await;
    assert_eq!(actual, 0.5);
}

#[tokio::test]
async fn integration_saved_search_blob_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    service
        .set_saved_search_blob(&service.pool, "genre:ambient")
        .await
        .unwrap();

    let actual = service.get_saved_search_blob(&service.pool).await.unwrap();
    assert_eq!(actual, "genre:ambient");
}

#[tokio::test]
async fn integration_song_list_limit_returns_default_when_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    let actual = service.get_song_list_limit(&service.pool).await;

    assert_eq!(actual, DEFAULT_SONG_LIST_LIMIT);
}

#[tokio::test]
async fn integration_song_list_limit_roundtrip() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    service
        .set_song_list_limit(&service.pool, 2500)
        .await
        .unwrap();

    let actual = service.get_song_list_limit(&service.pool).await;

    assert_eq!(actual, 2500);
}

#[tokio::test]
async fn integration_keybind_roundtrips() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    service
        .set_settings_keybind(&service.pool, "Ctrl+,")
        .await
        .unwrap();
    service
        .set_play_pause_keybind(&service.pool, "Space")
        .await
        .unwrap();
    service
        .set_previous_keybind(&service.pool, "Ctrl+Left")
        .await
        .unwrap();
    service
        .set_next_keybind(&service.pool, "Ctrl+Right")
        .await
        .unwrap();
    service
        .set_shuffle_keybind(&service.pool, "Ctrl+R")
        .await
        .unwrap();
    service
        .set_saved_search_blob(&service.pool, "aphex twin")
        .await
        .unwrap();

    assert_eq!(
        service.get_settings_keybind(&service.pool).await.unwrap(),
        "Ctrl+,"
    );
    assert_eq!(
        service.get_play_pause_keybind(&service.pool).await.unwrap(),
        "Space"
    );
    assert_eq!(
        service.get_previous_keybind(&service.pool).await.unwrap(),
        "Ctrl+Left"
    );
    assert_eq!(
        service.get_next_keybind(&service.pool).await.unwrap(),
        "Ctrl+Right"
    );
    assert_eq!(
        service.get_shuffle_keybind(&service.pool).await.unwrap(),
        "Ctrl+R"
    );
    assert_eq!(
        service.get_saved_search_blob(&service.pool).await.unwrap(),
        "aphex twin"
    );
}

#[tokio::test]
async fn integration_updates_existing_setting_instead_of_duplicate_insert() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SettingService::new(sqlite);

    service
        .set_music_folder_path(&service.pool, "/music/a")
        .await
        .unwrap();
    service
        .set_music_folder_path(&service.pool, "/music/b")
        .await
        .unwrap();

    let actual = service.get_music_folder_path(&service.pool).await.unwrap();
    assert_eq!(actual, "/music/b");

    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM setting
        WHERE key = 'music_folder_path'
        "#,
    )
    .fetch_one(&service.pool)
    .await
    .unwrap();

    assert_eq!(count, 1);
}
