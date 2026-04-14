pub mod common;

use tempfile::tempdir;
use zmp_lib::{
    filter::{Filter, FilterService},
    metadata::MetadataParser,
    song::SongService,
    song_filter::SongFilterService,
    song_metadata_filter_sync::SongMetadataFilterSyncService,
    sqlite::SqliteImpl,
};

use crate::common::{setup_db, song, write_id3_tag, write_test_wav};

#[tokio::test]
async fn sync_song_filters_from_metadata_replaces_existing_links() {
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

    let parser = MetadataParser::new();
    parser
        .add_song_filters_metadata(
            &file,
            vec![
                Filter {
                    id: 0,
                    name: "liked".to_string(),
                },
                Filter {
                    id: 0,
                    name: "chill".to_string(),
                },
            ],
        )
        .unwrap();

    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let song_service = SongService::new(sqlite.clone());
    let filter_service = FilterService::new(sqlite.clone());
    let song_filter_service = SongFilterService::new(sqlite.clone());
    let sync_service = SongMetadataFilterSyncService::new(sqlite);

    let inserted_song = song_service
        .add_song(
            &pool,
            song(
                0,
                "Angel",
                "Massive Attack",
                1998,
                "Mezzanine",
                "",
                "angel massive attack mezzanine 1998",
                &file.to_string_lossy(),
                180,
                "wav",
            ),
        )
        .await
        .unwrap();

    filter_service.add(&pool, "stale").await.unwrap();
    song_filter_service
        .add(&pool, inserted_song.id, 1)
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    let changed = sync_service
        .sync_song_filters_from_metadata(&mut tx, inserted_song.id, &file)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let filters = filter_service.get_all(&pool).await.unwrap();
    let song_filters = song_filter_service
        .get_by_song(&pool, inserted_song.id)
        .await
        .unwrap();

    assert!(changed);
    assert_eq!(
        filters
            .iter()
            .map(|filter| filter.name.as_str())
            .collect::<Vec<_>>(),
        vec!["stale", "liked", "chill"]
    );
    assert_eq!(song_filters.len(), 2);
}
