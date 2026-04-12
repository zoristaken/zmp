pub mod common;

use zmp_lib::{manager::PlayerManager, song::Song, sqlite::SqliteImpl};

use crate::common::setup_db;

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
