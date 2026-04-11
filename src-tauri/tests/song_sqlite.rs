pub mod common;
use crate::common::{sample_songs, setup_db};
use zmp_lib::{song::SongService, sqlite::SqliteDb};

#[tokio::test]
async fn integration_add_songs_and_list_songs() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    service
        .add_songs(&service.pool, sample_songs())
        .await
        .unwrap();

    let songs = service.list_songs(&service.pool).await.unwrap();
    assert_eq!(songs.len(), 3);
    assert_eq!(songs[0].title, "Teardrop");
    assert_eq!(songs[0].extension, "mp3");
    assert_eq!(songs[1].title, "Windowlicker");
    assert_eq!(songs[2].title, "Roygbiv");
}

#[tokio::test]
async fn integration_get_by_id_returns_matching_song() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    service
        .add_songs(&service.pool, sample_songs())
        .await
        .unwrap();

    let song = service.get_song_by_id(&service.pool, 2).await.unwrap();
    assert_eq!(song.title, "Windowlicker");
    assert_eq!(song.artist, "Aphex Twin");
    assert_eq!(song.extension, "mp4")
}

#[tokio::test]
async fn integration_get_by_id_returns_error_when_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    let err = service
        .get_song_by_id(&service.pool, 999)
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "no rows returned by a query that expected to return at least one row"
    );
}

#[tokio::test]
async fn integration_get_by_title_artist_returns_matching_song() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    service
        .add_songs(&service.pool, sample_songs())
        .await
        .unwrap();

    let song = service
        .get_by_title_artist(&service.pool, "Roygbiv", "Boards of Canada")
        .await
        .unwrap();

    assert_eq!(song.id, 3);
    assert_eq!(song.release_year, 1998);
}

#[tokio::test]
async fn integration_get_by_title_artist_returns_error_when_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    let err = service
        .get_by_title_artist(&service.pool, "Missing", "Nobody")
        .await
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        "no rows returned by a query that expected to return at least one row"
    );
}

#[tokio::test]
async fn integration_search_by_filters_in_memory_input() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
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
async fn integration_search_by_honors_max_results() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    let songs = sample_songs();
    let results = service.search_by(&songs, &["1998"], 1).await.unwrap();

    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn integration_search_by_db_returns_empty_when_words_are_empty() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    service
        .add_songs(&service.pool, sample_songs())
        .await
        .unwrap();

    let results = service.search_by_db(&service.pool, &[], 10).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn integration_search_by_db_finds_matching_rows() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    service
        .add_songs(&service.pool, sample_songs())
        .await
        .unwrap();

    let results = service
        .search_by_db(&service.pool, &["massive", "attack"], 10)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Teardrop");
}

#[tokio::test]
async fn integration_search_by_db_honors_max_results() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    service
        .add_songs(&service.pool, sample_songs())
        .await
        .unwrap();

    let results = service
        .search_by_db(&service.pool, &["1998"], 1)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn integration_search_by_db_alternative_finds_matching_rows() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    service
        .add_songs(&service.pool, sample_songs())
        .await
        .unwrap();

    let results = service
        .search_by_db_alternative(&service.pool, &["aphex", "twin"], 10)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Windowlicker");
}

#[tokio::test]
async fn integration_replace_songs_replaces_existing_library_contents() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    service
        .add_songs(&service.pool, sample_songs())
        .await
        .unwrap();

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

    service
        .replace_songs(&service.pool, replacement)
        .await
        .unwrap();

    let songs = service.list_songs(&service.pool).await.unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].title, "Archangel");
    assert_eq!(songs[0].artist, "Burial");
}

#[tokio::test]
async fn integration_replace_songs_allows_duplicate_title_artist_with_different_paths() {
    let pool = setup_db().await;
    let sqlite = SqliteDb { pool };
    let service = SongService::new(sqlite);

    service
        .replace_songs(
            &service.pool,
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

    let songs = service.list_songs(&service.pool).await.unwrap();

    assert_eq!(songs.len(), 2);
    assert_eq!(songs[0].title, "Freak On a Leash");
    assert_eq!(songs[1].title, "Freak On a Leash");
    assert_ne!(songs[0].file_path, songs[1].file_path);
}
