pub mod common;
use crate::common::{sample_song_filters, setup_db_with_song_and_filters};
use zmp_lib::{song_filter::SongFilterService, sqlite::SqliteDb};

#[tokio::test]
async fn integration_add_inserts_song_filter() {
    let pool = setup_db_with_song_and_filters().await;
    let sqlite = SqliteDb { pool };
    let service = SongFilterService::new(sqlite.clone());

    service.add(&service.pool, 3, 1).await.unwrap();

    let items = service.get_all(&service.pool).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, 1);
    assert_eq!(items[0].song_id, 3);
    assert_eq!(items[0].filter_id, 1);
}

#[tokio::test]
async fn integration_add_multiple_inserts_all_song_filters() {
    let pool = setup_db_with_song_and_filters().await;
    let sqlite = SqliteDb { pool };
    let service = SongFilterService::new(sqlite);

    service
        .add_multiple(&service.pool, sample_song_filters())
        .await
        .unwrap();

    let items = service.get_all(&service.pool).await.unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0].id, 1);
    assert_eq!(items[3].id, 4);
}

#[tokio::test]
async fn integration_get_all_returns_all_rows() {
    let pool = setup_db_with_song_and_filters().await;
    let sqlite = SqliteDb { pool };
    let service = SongFilterService::new(sqlite);

    service
        .add_multiple(&service.pool, sample_song_filters())
        .await
        .unwrap();

    let items = service.get_all(&service.pool).await.unwrap();
    assert_eq!(items.len(), 4);
}

#[tokio::test]
async fn integration_get_by_id_returns_matching_row() {
    let pool = setup_db_with_song_and_filters().await;
    let sqlite = SqliteDb { pool };
    let service = SongFilterService::new(sqlite);

    service
        .add_multiple(&service.pool, sample_song_filters())
        .await
        .unwrap();

    let item = service.get_by_id(&service.pool, 3).await.unwrap();
    assert_eq!(item.id, 3);
    assert_eq!(item.song_id, 2);
    assert_eq!(item.filter_id, 1);
}

#[tokio::test]
async fn integration_get_by_id_returns_error_when_missing() {
    let pool = setup_db_with_song_and_filters().await;
    let sqlite = SqliteDb { pool };
    let service = SongFilterService::new(sqlite);

    let err = service.get_by_id(&service.pool, 999).await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "no rows returned by a query that expected to return at least one row"
    );
}

#[tokio::test]
async fn integration_get_by_filter_returns_matching_rows() {
    let pool = setup_db_with_song_and_filters().await;
    let sqlite = SqliteDb { pool };
    let service = SongFilterService::new(sqlite.clone());

    service
        .add_multiple(&service.pool, sample_song_filters())
        .await
        .unwrap();

    let items = service.get_by_filter(&service.pool, 1).await.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, 1);
    assert_eq!(items[1].id, 3);
}

#[tokio::test]
async fn integration_get_by_filter_returns_empty_when_missing() {
    let pool = setup_db_with_song_and_filters().await;
    let sqlite = SqliteDb { pool };
    let service = SongFilterService::new(sqlite);

    service
        .add_multiple(&service.pool, sample_song_filters())
        .await
        .unwrap();

    let items = service.get_by_filter(&service.pool, 999).await.unwrap();
    assert!(items.is_empty());
}

#[tokio::test]
async fn integration_get_by_song_returns_matching_rows() {
    let pool = setup_db_with_song_and_filters().await;
    let sqlite = SqliteDb { pool };
    let service = SongFilterService::new(sqlite);

    service
        .add_multiple(&service.pool, sample_song_filters())
        .await
        .unwrap();

    let items = service.get_by_song(&service.pool, 1).await.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, 1);
    assert_eq!(items[1].id, 2);
}

#[tokio::test]
async fn integration_get_by_song_returns_empty_when_missing() {
    let pool = setup_db_with_song_and_filters().await;
    let sqlite = SqliteDb { pool };
    let service = SongFilterService::new(sqlite);

    service
        .add_multiple(&service.pool, sample_song_filters())
        .await
        .unwrap();

    let items = service.get_by_song(&service.pool, 999).await.unwrap();
    assert!(items.is_empty());
}
