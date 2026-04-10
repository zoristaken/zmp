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

//TODO: PROPER IMPLEMENTATION WITH CUSTOM ERRORS
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

//TODO: PROPER IMPLEMENTATION WITH CUSTOM ERRORS
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
        .search_by(&songs, &["boards", "ambient"], 10)
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
    let results = service.search_by(&songs, &["electronic"], 1).await.unwrap();

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
        .search_by_db(&service.pool, &["electronic"], 1)
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
