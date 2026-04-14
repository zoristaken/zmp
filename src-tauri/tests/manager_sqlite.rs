pub mod common;

use tempfile::tempdir;
use zmp_lib::{manager::PlayerManager, song::Song, sqlite::SqliteImpl};

use crate::common::{setup_db, write_id3_tag, write_test_wav};

fn song(title: &str, artist: &str, path: &str) -> Song {
    Song {
        id: 0,
        title: title.to_string(),
        artist: artist.to_string(),
        release_year: 1998,
        album: "Album".to_string(),
        remix: String::new(),
        search_blob: format!(
            "{} {} album 1998",
            title.to_lowercase(),
            artist.to_lowercase()
        ),
        file_path: path.to_string(),
        duration: 180,
        extension: "flac".to_string(),
        file_size: 4_096,
        file_modified_millis: 1_700_000_000_000,
    }
}

#[tokio::test]
async fn replace_library_and_reset_state_replaces_songs_and_resets_playback_settings() {
    let pool = setup_db().await;
    let manager = PlayerManager::new(pool.clone(), SqliteImpl {}).await;

    manager
        .song
        .add_songs(
            &pool,
            vec![song("Teardrop", "Massive Attack", "/music/teardrop.flac")],
        )
        .await
        .unwrap();

    manager
        .setting
        .set_saved_search_blob(&pool, "massive attack")
        .await
        .unwrap();

    manager.setting.set_saved_index(&pool, 7).await.unwrap();

    manager
        .setting
        .set_current_song_seek(&pool, 123)
        .await
        .unwrap();
    manager
        .setting
        .set_play_pause_flag(&pool, true)
        .await
        .unwrap();
    manager
        .setting
        .set_processed_music_folder(&pool, false)
        .await
        .unwrap();

    manager
        .replace_library_and_reset_state(vec![song("Archangel", "Burial", "/music/archangel.flac")])
        .await
        .unwrap();

    let songs = manager.song.list_songs(&pool).await.unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].title, "Archangel");
    assert_eq!(songs[0].artist, "Burial");
    assert_eq!(
        manager.setting.get_saved_search_blob(&pool).await.unwrap(),
        ""
    );
    assert_eq!(manager.setting.get_saved_index(&pool).await, 0);
    assert_eq!(manager.setting.get_current_song_seek(&pool).await, 0);
    assert!(!manager.setting.is_playing(&pool).await);
    assert!(manager.setting.has_processed_music_folder(&pool).await);
}

#[tokio::test]
async fn replace_library_and_reset_state_clears_song_filter_links_but_keeps_filters() {
    let pool = setup_db().await;
    let manager = PlayerManager::new(pool.clone(), SqliteImpl {}).await;

    manager
        .song
        .add_songs(
            &pool,
            vec![song("Teardrop", "Massive Attack", "/music/teardrop.flac")],
        )
        .await
        .unwrap();

    manager.filter.add(&pool, "ambient").await.unwrap();
    manager.song_filter.add(&pool, 1, 1).await.unwrap();

    manager
        .replace_library_and_reset_state(vec![song("Archangel", "Burial", "/music/archangel.flac")])
        .await
        .unwrap();

    let filters = manager.filter.get_all(&pool).await.unwrap();
    let song_filters = manager.song_filter.get_all(&pool).await.unwrap();

    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0].name, "ambient");
    assert!(song_filters.is_empty());
}

#[tokio::test]
async fn load_restores_saved_volume_into_the_player() {
    let pool = setup_db().await;
    let manager = PlayerManager::new(pool.clone(), SqliteImpl {}).await;

    manager
        .setting
        .set_saved_volume_value(&pool, 0.42)
        .await
        .unwrap();

    let state = manager.load().await.unwrap();
    let player = manager.player.lock().unwrap();

    assert_eq!(state.count, 0);
    assert_eq!(state.current_index, None);
    assert!((state.volume - 0.42).abs() < f32::EPSILON);
    assert!((player.get_volume() - 0.42).abs() < f32::EPSILON);
}

#[tokio::test]
async fn process_music_folder_imports_song_filters_from_zmp_filters_metadata() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("Massive Attack - Angel.wav");

    write_test_wav(&file, 1);
    write_id3_tag(
        &file,
        Some("Angel"),
        Some("Massive Attack"),
        Some("Mezzanine"),
        Some(1998),
        None,
    );

    let parser = zmp_lib::metadata::MetadataParser::new();
    parser
        .add_song_filters_metadata(
            &file,
            vec![
                zmp_lib::filter::Filter {
                    id: 0,
                    name: "liked".to_string(),
                },
                zmp_lib::filter::Filter {
                    id: 0,
                    name: "chill".to_string(),
                },
            ],
        )
        .unwrap();

    let pool = setup_db().await;
    let manager = PlayerManager::new(pool.clone(), SqliteImpl {}).await;

    manager
        .setting
        .set_music_folder_path(&pool, dir.path().to_string_lossy().as_ref())
        .await
        .unwrap();

    manager.process_music_folder().await.unwrap();

    let songs = manager.song.list_songs(&pool).await.unwrap();
    let filters = manager.filter.get_all(&pool).await.unwrap();
    let song_filters = manager.get_filters_for_song(songs[0].id).await.unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].title, "Angel");
    assert_eq!(filters.len(), 2);
    assert_eq!(
        filters
            .iter()
            .map(|filter| filter.name.as_str())
            .collect::<Vec<_>>(),
        vec!["liked", "chill"]
    );
    assert_eq!(song_filters.len(), 2);
}

#[tokio::test]
async fn process_music_folder_prefers_pending_db_filters_over_stale_file_metadata() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("Massive Attack - Angel.wav");
    let file_path = file.to_string_lossy().to_string();

    write_test_wav(&file, 1);
    write_id3_tag(
        &file,
        Some("Angel"),
        Some("Massive Attack"),
        Some("Mezzanine"),
        Some(1998),
        None,
    );

    let parser = zmp_lib::metadata::MetadataParser::new();
    parser
        .add_song_filters_metadata(
            &file,
            vec![zmp_lib::filter::Filter {
                id: 0,
                name: "file-filter".to_string(),
            }],
        )
        .unwrap();

    let pool = setup_db().await;
    let manager = PlayerManager::new(pool.clone(), SqliteImpl {}).await;

    manager
        .song
        .add_songs(&pool, vec![song("Angel", "Massive Attack", &file_path)])
        .await
        .unwrap();
    manager.filter.add(&pool, "db-filter").await.unwrap();
    manager.song_filter.add(&pool, 1, 1).await.unwrap();
    manager
        .setting
        .set_music_folder_path(&pool, dir.path().to_string_lossy().as_ref())
        .await
        .unwrap();
    manager
        .setting
        .set_pending_song_metadata_sync_paths(&pool, std::slice::from_ref(&file_path))
        .await
        .unwrap();

    manager.process_music_folder().await.unwrap();

    let songs = manager.song.list_songs(&pool).await.unwrap();
    let filters = manager.filter.get_all(&pool).await.unwrap();
    let song_filters = manager.get_filters_for_song(songs[0].id).await.unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(
        filters
            .iter()
            .map(|filter| filter.name.as_str())
            .collect::<Vec<_>>(),
        vec!["db-filter"]
    );
    assert_eq!(song_filters.len(), 1);
    assert_eq!(song_filters[0].filter_id, filters[0].id);
}
