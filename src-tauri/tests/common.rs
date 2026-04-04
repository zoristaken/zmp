use sqlx::SqlitePool;
use zmp_lib::{
    filter::{Filter, FilterService},
    song::{Song, SongService},
    song_filter::SongFilter,
    sqlite::SqliteDb,
};

pub async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    let _ = sqlx::migrate!("./migrations").run(&pool).await;

    pool
}

pub async fn setup_db_with_song_and_filters() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    let _ = sqlx::migrate!("./migrations").run(&pool).await;

    let sqlite = SqliteDb { pool: pool.clone() };
    let song_service = SongService::new(sqlite.clone());
    let filter_service = FilterService::new(sqlite.clone());

    let _ = song_service
        .add_songs(&song_service.pool, sample_songs())
        .await;

    let filters = sample_filters();

    for filter in filters {
        let _ = filter_service.add(&filter_service.pool, &filter.name).await;
    }

    pool
}

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
            "teardrop massive attack mezzanine trip hop",
            "/music/teardrop.mp3",
            330,
        ),
        song(
            2,
            "Windowlicker",
            "Aphex Twin",
            1999,
            "Windowlicker",
            "",
            "windowlicker aphex twin idm electronic",
            "/music/windowlicker.mp3",
            360,
        ),
        song(
            3,
            "Roygbiv",
            "Boards of Canada",
            1998,
            "Music Has the Right to Children",
            "",
            "roygbiv boards of canada ambient electronic",
            "/music/roygbiv.mp3",
            170,
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
