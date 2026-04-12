pub mod common;
use crate::common::{filter, sample_filters, setup_db};
use zmp_lib::{filter::FilterService, sqlite::SqliteImpl};

#[tokio::test]
async fn integration_add_inserts_expected_filter() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = FilterService::new(sqlite);

    service.add(&pool, "trip-hop").await.unwrap();

    let all = service.get_all(&pool).await.unwrap();
    assert_eq!(all, vec![filter(1, "trip-hop")]);
}

#[tokio::test]
async fn integration_get_all_returns_exact_filters_in_order() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = FilterService::new(sqlite);

    service.add(&pool, "ambient").await.unwrap();
    service.add(&pool, "electronic").await.unwrap();
    service.add(&pool, "favorites").await.unwrap();

    let actual = service.get_all(&pool).await.unwrap();
    assert_eq!(actual, sample_filters());
}

#[tokio::test]
async fn integration_get_by_name_returns_exact_filter() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = FilterService::new(sqlite);

    service.add(&pool, "ambient").await.unwrap();
    service.add(&pool, "electronic").await.unwrap();

    let actual = service.get_by_name(&pool, "electronic").await.unwrap();
    assert_eq!(actual, filter(2, "electronic"));
}

#[tokio::test]
async fn integration_get_by_name_returns_error_when_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = FilterService::new(sqlite);

    let err = service.get_by_name(&pool, "missing").await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "no rows returned by a query that expected to return at least one row"
    );
}

#[tokio::test]
async fn integration_get_by_id_returns_exact_filter() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = FilterService::new(sqlite);

    service.add(&pool, "ambient").await.unwrap();
    service.add(&pool, "electronic").await.unwrap();
    service.add(&pool, "favorites").await.unwrap();

    let actual = service.get_by_id(&pool, 3).await.unwrap();
    assert_eq!(actual, filter(3, "favorites"));
}

#[tokio::test]
async fn integration_get_by_id_returns_error_when_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = FilterService::new(sqlite);

    let err = service.get_by_id(&pool, 999).await.unwrap_err();
    assert_eq!(
        err.to_string(),
        "no rows returned by a query that expected to return at least one row"
    );
}

#[tokio::test]
async fn integration_add_returns_error_for_duplicate_name() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = FilterService::new(sqlite);

    service.add(&pool, "ambient").await.unwrap();

    let err = service.add(&pool, "ambient").await.unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("failed to insert filter") || msg.contains("UNIQUE"));
}

#[tokio::test]
async fn integration_remove_returns_false_when_filter_is_missing() {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let service = FilterService::new(sqlite);

    let removed = service.remove(&pool, 999).await.unwrap();

    assert!(!removed);
}
