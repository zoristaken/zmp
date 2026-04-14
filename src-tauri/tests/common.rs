use std::path::Path;

use id3::{Tag, TagLike, Version};
use sqlx::SqlitePool;
use zmp_lib::{
    filter::{Filter, FilterService},
    song::{Song, SongService},
    song_filter::{SongFilter, SongFilterService},
    sqlite::SqliteImpl,
};

pub async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    let _ = sqlx::migrate!("./migrations").run(&pool).await;

    pool
}

pub fn write_test_wav(path: &Path, duration_secs: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 8_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec).unwrap();

    let total_samples = 8_000 * duration_secs;
    for _ in 0..total_samples {
        writer.write_sample(0i16).unwrap();
    }

    writer.finalize().unwrap();
}

pub fn write_id3_tag(
    path: &Path,
    title: Option<&str>,
    artist: Option<&str>,
    album: Option<&str>,
    year: Option<i32>,
    remixer: Option<&str>,
) {
    let mut tag = Tag::new();

    if let Some(title) = title {
        tag.set_title(title);
    }

    if let Some(artist) = artist {
        tag.set_artist(artist);
    }

    if let Some(album) = album {
        tag.set_album(album);
    }

    if let Some(year) = year {
        tag.set_year(year);
    }

    if let Some(remixer) = remixer {
        tag.add_frame(id3::Frame::with_content(
            "TPE4",
            id3::Content::Text(remixer.to_string()),
        ));
    }

    tag.write_to_path(path, Version::Id3v24).unwrap();
}

pub async fn setup_db_with_song_and_filters() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    let _ = sqlx::migrate!("./migrations").run(&pool).await;

    let sqlite = SqliteImpl {};
    let song_service = SongService::new(sqlite.clone());
    let filter_service = FilterService::new(sqlite.clone());

    let _ = song_service.add_songs(&pool, sample_songs()).await;

    let filters = sample_filters();

    for filter in filters {
        let _ = filter_service.add(&pool, &filter.name).await;
    }

    pool
}

pub async fn setup_db_with_query_fixture() -> SqlitePool {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let song_service = SongService::new(sqlite.clone());
    let filter_service = FilterService::new(sqlite.clone());
    let song_filter_service = SongFilterService::new(sqlite);

    song_service.add_songs(&pool, sample_songs()).await.unwrap();

    filter_service.add(&pool, "favorites").await.unwrap();
    filter_service.add(&pool, "ambient").await.unwrap();

    song_filter_service.add(&pool, 1, 1).await.unwrap();
    song_filter_service.add(&pool, 1, 2).await.unwrap();
    song_filter_service.add(&pool, 3, 2).await.unwrap();

    pool
}

pub async fn setup_db_with_single_song_and_filters() -> SqlitePool {
    let pool = setup_db().await;
    let sqlite = SqliteImpl {};
    let song_service = SongService::new(sqlite.clone());
    let filter_service = FilterService::new(sqlite);

    song_service
        .add_songs(
            &pool,
            vec![song(
                0,
                "Teardrop",
                "Massive Attack",
                1998,
                "Mezzanine",
                "",
                "teardrop massive attack mezzanine 1998",
                "/music/Teardrop.mp3",
                180,
                "mp3",
            )],
        )
        .await
        .unwrap();

    filter_service.add(&pool, "ambient").await.unwrap();
    filter_service.add(&pool, "favorites").await.unwrap();

    pool
}

#[allow(clippy::too_many_arguments)]
pub fn song(
    id: i32,
    title: &str,
    artist: &str,
    release_year: i32,
    album: &str,
    remix: &str,
    search_blob: &str,
    file_path: &str,
    duration: i64,
    extension: &str,
) -> Song {
    Song {
        id,
        title: title.to_string(),
        artist: artist.to_string(),
        release_year,
        album: album.to_string(),
        remix: remix.to_string(),
        search_blob: search_blob.to_string(),
        file_path: file_path.to_string(),
        duration,
        extension: extension.to_string(),
        file_size: 4_096,
        file_modified_millis: 1_700_000_000_000,
    }
}

pub fn sample_songs() -> Vec<Song> {
    vec![
        song(
            1,
            "Teardrop",
            "Massive Attack",
            1998,
            "Mezzanine",
            "",
            "teardrop massive attack mezzanine 1998",
            "/music/teardrop.mp3",
            330,
            "mp3",
        ),
        song(
            2,
            "Windowlicker",
            "Aphex Twin",
            1999,
            "Windowlicker",
            "",
            "windowlicker aphex twin windowlicker 1999",
            "/music/windowlicker.mp4",
            360,
            "mp4",
        ),
        song(
            3,
            "Roygbiv",
            "Boards of Canada",
            1998,
            "Music Has the Right to Children",
            "",
            "roygbiv boards of canada music has the right to children 1998",
            "/music/roygbiv.flac",
            170,
            "flac",
        ),
    ]
}

pub fn filter(id: i32, name: &str) -> Filter {
    Filter {
        id,
        name: name.to_string(),
    }
}

pub fn sample_filters() -> Vec<Filter> {
    vec![
        filter(1, "ambient"),
        filter(2, "electronic"),
        filter(3, "favorites"),
    ]
}

pub fn song_filter(id: i32, song_id: i32, filter_id: i32) -> SongFilter {
    SongFilter {
        id,
        song_id,
        filter_id,
    }
}

pub fn sample_song_filters() -> Vec<SongFilter> {
    vec![
        song_filter(1, 1, 1),
        song_filter(2, 1, 2),
        song_filter(3, 2, 1),
        song_filter(3, 3, 3),
    ]
}
