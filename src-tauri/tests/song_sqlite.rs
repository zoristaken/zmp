pub mod common;
use crate::common::{sample_songs, setup_db};
use zmp_lib::{song::SongService, sqlite::SqliteImpl};

#[tokio::test]
async fn integration_add_songs_and_list_songs() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service.add_songs(&pool, sample_songs()).await.unwrap();

    let songs = service.list_songs(&pool).await.unwrap();
    assert_eq!(songs.len(), 3);
    assert_eq!(songs[0].title, "Teardrop");
    assert_eq!(songs[0].extension, "mp3");
    assert_eq!(songs[0].file_size, 4_096);
    assert_eq!(songs[0].file_modified_millis, 1_700_000_000_000);
    assert_eq!(songs[1].title, "Windowlicker");
    assert_eq!(songs[2].title, "Roygbiv");
}

#[tokio::test]
async fn integration_get_by_id_returns_matching_song() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service.add_songs(&pool, sample_songs()).await.unwrap();

    let song = service.get_song_by_id(&pool, 2).await.unwrap();
    assert_eq!(song.title, "Windowlicker");
    assert_eq!(song.artist, "Aphex Twin");
    assert_eq!(song.extension, "mp4")
}

#[tokio::test]
async fn integration_get_by_id_returns_error_when_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    let err = service.get_song_by_id(&pool, 999).await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "no rows returned by a query that expected to return at least one row"
    );
}

#[tokio::test]
async fn integration_get_by_title_artist_returns_matching_song() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service.add_songs(&pool, sample_songs()).await.unwrap();

    let song = service
        .get_by_title_artist(&pool, "Roygbiv", "Boards of Canada")
        .await
        .unwrap();

    assert_eq!(song.id, 3);
    assert_eq!(song.release_year, 1998);
}

#[tokio::test]
async fn integration_get_by_title_artist_returns_error_when_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    let err = service
        .get_by_title_artist(&pool, "Missing", "Nobody")
        .await
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        "no rows returned by a query that expected to return at least one row"
    );
}

#[tokio::test]
async fn integration_search_by_filters_in_memory_input() {
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    let songs = sample_songs();
    let results = service
        .search_by(&songs, &["boards", "1998"], 10)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Roygbiv");
    assert_eq!(results[0].extension, "flac")
}

#[tokio::test]
async fn integration_search_by_db_and_memory_results_match() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    let songs = sample_songs();
    let results_memory = service
        .search_by(&songs, &["boards", "1998"], 10)
        .await
        .unwrap();

    service.add_songs(&pool, songs).await.unwrap();

    let results_db = service
        .search_by_db(&pool, &["boards", "1998"], 10)
        .await
        .unwrap();

    assert_eq!(results_memory.len(), 1);
    assert_eq!(results_memory[0].title, "Roygbiv");
    assert_eq!(results_memory[0].extension, "flac");

    assert_eq!(results_db.len(), 1);
    assert_eq!(results_db[0].title, "Roygbiv");
    assert_eq!(results_db[0].extension, "flac");

    let results_memory = service
        .search_by(&sample_songs(), &["19"], 10)
        .await
        .unwrap();

    let results_db = service.search_by_db(&pool, &["19"], 10).await.unwrap();

    assert_eq!(results_memory.len(), 3);
    assert_eq!(results_memory[0].title, "Roygbiv");
    assert_eq!(results_memory[0].artist, "Boards of Canada");
    assert_eq!(results_memory[1].title, "Teardrop");
    assert_eq!(results_memory[1].extension, "mp3");
    assert_eq!(results_memory[2].title, "Windowlicker");
    assert_eq!(results_memory[2].extension, "mp4");

    assert_eq!(results_db.len(), 3);
    assert_eq!(results_db[0].title, "Roygbiv");
    assert_eq!(results_db[0].artist, "Boards of Canada");
    assert_eq!(results_db[1].title, "Teardrop");
    assert_eq!(results_db[1].extension, "mp3");
    assert_eq!(results_db[2].title, "Windowlicker");
    assert_eq!(results_db[2].extension, "mp4");
}

#[tokio::test]
async fn integration_search_by_honors_max_results() {
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    let songs = sample_songs();
    let results = service.search_by(&songs, &["1998"], 1).await.unwrap();

    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn integration_search_by_db_returns_empty_when_words_are_empty() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service.add_songs(&pool, sample_songs()).await.unwrap();

    let results = service.search_by_db(&pool, &[], 10).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn integration_search_by_db_finds_matching_rows() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service.add_songs(&pool, sample_songs()).await.unwrap();

    let results = service
        .search_by_db(&pool, &["massive", "attack"], 10)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Teardrop");
}

#[tokio::test]
async fn integration_search_by_db_honors_max_results() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service.add_songs(&pool, sample_songs()).await.unwrap();

    let results = service.search_by_db(&pool, &["1998"], 1).await.unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn integration_replace_songs_replaces_existing_library_contents() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service.add_songs(&pool, sample_songs()).await.unwrap();

    let replacement = vec![crate::common::song(
        0,
        "Archangel",
        "Burial",
        2007,
        "Untrue",
        "",
        "archangel burial untrue 2007",
        "/music/archangel.flac",
        230,
        "flac",
    )];

    service.replace_songs(&pool, replacement).await.unwrap();

    let songs = service.list_songs(&pool).await.unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].title, "Archangel");
    assert_eq!(songs[0].artist, "Burial");
}

#[tokio::test]
async fn integration_replace_songs_allows_duplicate_title_artist_with_different_paths() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service
        .replace_songs(
            &pool,
            vec![
                crate::common::song(
                    0,
                    "Freak On a Leash",
                    "Korn",
                    1998,
                    "Follow the Leader",
                    "",
                    "freak on a leash korn follow the leader 1998",
                    "/music/korn/follow-the-leader/11 - Korn - Freak On a Leash.flac",
                    255,
                    "flac",
                ),
                crate::common::song(
                    0,
                    "Freak On a Leash",
                    "Korn",
                    2011,
                    "The Essential Korn",
                    "",
                    "freak on a leash korn the essential korn 2011",
                    "/music/korn/the-essential-korn/11 - Korn - Freak On a Leash.flac",
                    255,
                    "flac",
                ),
            ],
        )
        .await
        .unwrap();

    let songs = service.list_songs(&pool).await.unwrap();

    assert_eq!(songs.len(), 2);
    assert_eq!(songs[0].title, "Freak On a Leash");
    assert_eq!(songs[1].title, "Freak On a Leash");
    assert_ne!(songs[0].file_path, songs[1].file_path);
}

#[tokio::test]
async fn integration_add_song_returns_saved_row() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    let saved_song = service
        .add_song(
            &pool,
            crate::common::song(
                0,
                "Archangel",
                "Burial",
                2007,
                "Untrue",
                "",
                "archangel burial untrue 2007",
                "/music/archangel.flac",
                230,
                "flac",
            ),
        )
        .await
        .unwrap();

    assert!(saved_song.id > 0);
    assert_eq!(saved_song.title, "Archangel");
    assert_eq!(saved_song.file_path, "/music/archangel.flac");
}

#[tokio::test]
async fn integration_update_song_persists_metadata_without_changing_id() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service.add_songs(&pool, sample_songs()).await.unwrap();

    let updated = crate::common::song(
        2,
        "Windowlicker (Single Edit)",
        "Aphex Twin",
        1999,
        "Windowlicker",
        "",
        "windowlicker single edit aphex twin windowlicker 1999",
        "/music/windowlicker.mp4",
        361,
        "mp4",
    );

    let changed = service.update_song(&pool, updated).await.unwrap();
    let saved = service.get_song_by_id(&pool, 2).await.unwrap();

    assert!(changed);
    assert_eq!(saved.id, 2);
    assert_eq!(saved.title, "Windowlicker (Single Edit)");
    assert_eq!(saved.duration, 361);
}

#[tokio::test]
async fn integration_remove_song_deletes_row() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = SongService::new(sqlite);

    service.add_songs(&pool, sample_songs()).await.unwrap();

    let removed = service.remove_song(&pool, 2).await.unwrap();
    let songs = service.list_songs(&pool).await.unwrap();

    assert!(removed);
    assert_eq!(songs.len(), 2);
    assert!(songs.iter().all(|song| song.id != 2));
}
