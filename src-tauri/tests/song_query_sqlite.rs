pub mod common;

use zmp_lib::{song_query::SongQueryService, sqlite::SqliteImpl};

use crate::common::setup_db_with_query_fixture;

#[tokio::test]
async fn integration_list_songs_with_filters_preserves_title_order_and_includes_filters() {
    let pool = setup_db_with_query_fixture().await;
    let sqlite = SqliteImpl {};
    let service = SongQueryService::new(sqlite.clone());

    let songs = service.query_song_list(&pool, "", 10, None).await.unwrap();

    assert_eq!(songs.len(), 3);
    assert_eq!(songs[0].song.title, "Roygbiv");
    assert_eq!(songs[1].song.title, "Teardrop");
    assert_eq!(songs[2].song.title, "Windowlicker");
    assert_eq!(songs[0].filters.len(), 1);
    assert_eq!(songs[0].filters[0].name, "ambient");
    assert_eq!(songs[1].filters.len(), 2);
    assert_eq!(songs[1].filters[0].name, "ambient");
    assert_eq!(songs[1].filters[1].name, "favorites");
}

#[tokio::test]
async fn integration_list_songs_with_filters_limit_keeps_all_filters_for_selected_songs() {
    let pool = setup_db_with_query_fixture().await;
    let sqlite = SqliteImpl {};
    let service = SongQueryService::new(sqlite.clone());

    let songs = service.query_song_list(&pool, "", 2, None).await.unwrap();

    assert_eq!(songs.len(), 2);
    assert_eq!(songs[0].song.title, "Roygbiv");
    assert_eq!(songs[1].song.title, "Teardrop");
    assert_eq!(songs[1].filters.len(), 2);
    assert_eq!(songs[1].filters[0].name, "ambient");
    assert_eq!(songs[1].filters[1].name, "favorites");
}

#[tokio::test]
async fn integration_list_songs_with_filters_includes_pinned_song_beyond_limit() {
    let pool = setup_db_with_query_fixture().await;
    let sqlite = SqliteImpl {};
    let service = SongQueryService::new(sqlite.clone());

    let songs = service
        .query_song_list(&pool, "", 1, Some(2))
        .await
        .unwrap();

    assert_eq!(songs.len(), 2);
    assert_eq!(songs[0].song.title, "Roygbiv");
    assert_eq!(songs[1].song.title, "Windowlicker");
}

#[tokio::test]
async fn integration_search_by_db_with_filters_respects_limit_and_returns_attached_filters() {
    let pool = setup_db_with_query_fixture().await;
    let sqlite = SqliteImpl {};
    let service = SongQueryService::new(sqlite.clone());

    let songs = service
        .query_song_list(&pool, "1998", 1, None)
        .await
        .unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].song.title, "Roygbiv");
    assert_eq!(songs[0].filters.len(), 1);
    assert_eq!(songs[0].filters[0].name, "ambient");
}

#[tokio::test]
async fn integration_search_by_db_with_filters_limit_does_not_drop_extra_filter_rows() {
    let pool = setup_db_with_query_fixture().await;
    let sqlite = SqliteImpl {};
    let service = SongQueryService::new(sqlite.clone());

    let songs = service
        .query_song_list(&pool, "teardrop", 1, None)
        .await
        .unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].song.title, "Teardrop");
    assert_eq!(songs[0].filters.len(), 2);
    assert_eq!(songs[0].filters[0].name, "ambient");
    assert_eq!(songs[0].filters[1].name, "favorites");
}

#[tokio::test]
async fn integration_query_song_list_uses_pinned_song_for_empty_query() {
    let pool = setup_db_with_query_fixture().await;
    let sqlite = SqliteImpl {};
    let service = SongQueryService::new(sqlite.clone());

    let songs = service
        .query_song_list(&pool, "   ", 1, Some(2))
        .await
        .unwrap();

    assert_eq!(songs.len(), 2);
    assert_eq!(songs[0].song.title, "Roygbiv");
    assert_eq!(songs[1].song.title, "Windowlicker");
}

#[tokio::test]
async fn integration_query_song_list_searches_trimmed_query() {
    let pool = setup_db_with_query_fixture().await;
    let sqlite = SqliteImpl {};
    let service = SongQueryService::new(sqlite.clone());

    let songs = service
        .query_song_list(&pool, "  teardrop  ", 10, Some(3))
        .await
        .unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].song.title, "Teardrop");
    assert_eq!(songs[0].filters.len(), 2);
}
