pub mod common;

use crate::common::setup_db_with_single_song_and_filters;
use zmp_lib::{
    song::SongService, song_filter::SongFilterService, song_mutation::SongMutationService,
    sqlite::SqliteImpl,
};

#[tokio::test]
async fn integration_add_filter_to_song_and_reindex_updates_search_results() {
    let pool = setup_db_with_single_song_and_filters().await;
    let sqlite = SqliteImpl {};
    let service = SongMutationService::new(sqlite.clone());
    let song_service = SongService::new(sqlite.clone());

    let before = song_service
        .search_by_db(&pool, &["ambient"], 10)
        .await
        .unwrap();
    assert!(before.is_empty());

    let updated = service
        .add_filter_to_song_and_reindex(&pool, 1, 1)
        .await
        .unwrap();

    assert!(updated);

    let after = song_service
        .search_by_db(&pool, &["ambient"], 10)
        .await
        .unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].title, "Teardrop");
    assert!(after[0].search_blob.contains("ambient"));
}

#[tokio::test]
async fn integration_remove_filter_from_song_and_reindex_removes_filter_terms() {
    let pool = setup_db_with_single_song_and_filters().await;
    let sqlite = SqliteImpl {};
    let service = SongMutationService::new(sqlite.clone());
    let song_service = SongService::new(sqlite.clone());
    let song_filter_service = SongFilterService::new(sqlite.clone());

    service
        .add_filter_to_song_and_reindex(&pool, 1, 1)
        .await
        .unwrap();

    let link = song_filter_service
        .get_by_song(&pool, 1)
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let removed = service
        .remove_filter_from_song_and_reindex(&pool, link.id)
        .await
        .unwrap();

    assert!(removed);

    let after = song_service
        .search_by_db(&pool, &["ambient"], 10)
        .await
        .unwrap();
    assert!(after.is_empty());

    let song = song_service.get_song_by_id(&pool, 1).await.unwrap();
    assert_eq!(song.search_blob, "teardrop massive attack mezzanine 1998");
}

#[tokio::test]
async fn integration_refresh_all_song_search_blobs_reindexes_existing_song_filter_links() {
    let pool = setup_db_with_single_song_and_filters().await;
    let sqlite = SqliteImpl {};
    let service = SongMutationService::new(sqlite.clone());
    let song_service = SongService::new(sqlite.clone());
    let song_filter_service = SongFilterService::new(sqlite.clone());

    song_filter_service.add(&pool, 1, 2).await.unwrap();

    let refreshed = service.refresh_all_song_search_blobs(&pool).await.unwrap();

    assert_eq!(refreshed, 1);

    let results = song_service
        .search_by_db(&pool, &["favorites"], 10)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Teardrop");
    assert!(results[0].search_blob.contains("favorites"));
}
