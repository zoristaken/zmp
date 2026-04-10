pub mod common;

use zmp_lib::{
    song_query::{group_rows, SongQueryService, SongWithFilterRow},
    sqlite::SqliteDb,
};

use crate::common::setup_db_with_query_fixture;

fn row(
    id: i32,
    title: &str,
    filter_id: Option<i32>,
    filter_name: Option<&str>,
) -> SongWithFilterRow {
    SongWithFilterRow {
        id,
        title: title.to_string(),
        artist: "artist".to_string(),
        release_year: 1999,
        album: "album".to_string(),
        remix: String::new(),
        search_blob: title.to_lowercase(),
        file_path: format!("/music/{id}.mp3"),
        duration: 180,
        extension: "mp3".to_string(),
        filter_id,
        filter_name: filter_name.map(str::to_string),
    }
}

#[test]
fn integration_group_rows_preserves_input_song_order() {
    let rows = vec![
        row(2, "Alpha", Some(1), Some("ambient")),
        row(1, "Bravo", Some(2), Some("favorites")),
        row(3, "Charlie", None, None),
    ];

    let grouped = group_rows(rows);

    assert_eq!(grouped.len(), 3);
    assert_eq!(grouped[0].song.id, 2);
    assert_eq!(grouped[1].song.id, 1);
    assert_eq!(grouped[2].song.id, 3);
}

#[test]
fn integration_group_rows_keeps_multiple_filters_on_the_same_song() {
    let rows = vec![
        row(7, "Teardrop", Some(1), Some("ambient")),
        row(7, "Teardrop", Some(2), Some("favorites")),
    ];

    let grouped = group_rows(rows);

    assert_eq!(grouped.len(), 1);
    assert_eq!(grouped[0].filters.len(), 2);
    assert_eq!(grouped[0].filters[0].name, "ambient");
    assert_eq!(grouped[0].filters[1].name, "favorites");
}

#[tokio::test]
async fn integration_list_songs_with_filters_preserves_title_order_and_includes_filters() {
    let pool = setup_db_with_query_fixture().await;
    let sqlite = SqliteDb { pool };
    let service = SongQueryService::new(sqlite.clone());

    let songs = service.list_songs_with_filters(&sqlite.pool).await.unwrap();

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
async fn integration_search_by_db_with_filters_respects_limit_and_returns_attached_filters() {
    let pool = setup_db_with_query_fixture().await;
    let sqlite = SqliteDb { pool };
    let service = SongQueryService::new(sqlite.clone());

    let songs = service
        .search_by_db_with_filters(&sqlite.pool, &["1998"], 1)
        .await
        .unwrap();

    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].song.title, "Roygbiv");
    assert_eq!(songs[0].filters.len(), 1);
    assert_eq!(songs[0].filters[0].name, "ambient");
}
