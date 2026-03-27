mod application;
mod config;
mod database;
mod domain;
mod infrastructure;

use application::FilterService;
use application::SettingService;
use application::SongFilterService;
use application::SongService;
use infrastructure::SqliteFilterRepository;
use infrastructure::SqliteSettingRepository;
use infrastructure::SqliteSongFilterRepository;
use infrastructure::SqliteSongRepository;

use std::{sync::Arc, time::Instant};

use crate::domain::Song;
use crate::domain::SongFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::new()?;
    let sqlite = Arc::new(database::SqliteDb::new(&config.database_url).await?);
    let song_repo = SqliteSongRepository::new(sqlite.clone());
    let setting_repo = SqliteSettingRepository::new(sqlite.clone());
    let filter_repo = SqliteFilterRepository::new(sqlite.clone());
    let song_filter_repo = SqliteSongFilterRepository::new(sqlite.clone());
    let song_service = SongService::new(song_repo);
    let setting_service = SettingService::new(setting_repo);
    let filter_service = FilterService::new(filter_repo);
    let song_filter_service = SongFilterService::new(song_filter_repo);

    //testing for now
    let songs = song_service.list_songs().await;
    song_service
        .add_songs(vec![Song {
            id: 0,
            title: "insertion".to_string(),
            artist: "manual".to_string(),
            album: "lazy".to_string(),
            release_year: 2026,
            remix: "z".to_string(),
            search_blob: "insertion manual 2026 z".to_string(),
            file_path: "".to_string(),
            duration: 123,
        }])
        .await;

    let searchable = vec!["2026", "man", "insertion"];
    let start = Instant::now();
    let r = song_service.search_by(&songs, searchable, 1).await;
    let duration = start.elapsed();
    for s in r {
        println!("Songs List: {:?}", s);
    }
    println!("{:?}", duration);

    let searchable = vec!["2026", "man", "insertion"];
    let start = Instant::now();
    let r = song_service.search_by_db(searchable, 1).await;
    let duration = start.elapsed();
    for s in r {
        println!("Songs List: {:?}", s);
    }
    println!("{:?}", duration);

    setting_service.set("music_folder_path", "~/Music").await;

    let r = setting_service.get("music_folder_path").await;
    println!(
        "Found setting <id:{:?}, key:{:?}, value:{:?}>",
        r.id, r.key, r.value
    );

    filter_service.set("trance").await;
    filter_service.set("metal").await;
    filter_service.set("oldschool").await;
    filter_service.set("favorite").await;

    let r = filter_service.get_all().await;
    for s in r {
        println!("Found all filter <id:{:?}, name:{:?}", s.id, s.name);
    }

    let r = filter_service.get_by_name("favorite").await;
    println!("Found filter by name <id:{:?}, name:{:?}>", r.id, r.name);
    let r = filter_service.get_by_id(1).await;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);
    let r = filter_service.get_by_id(2).await;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);
    let r = filter_service.get_by_id(3).await;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);
    let r = filter_service.get_by_id(4).await;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);

    song_filter_service
        .add(SongFilter {
            id: 0,
            filter_id: 1,
            song_id: 100001,
        })
        .await;

    song_filter_service
        .add_multiple(vec![
            SongFilter {
                id: 0,
                filter_id: 4,
                song_id: 100001,
            },
            SongFilter {
                id: 0,
                filter_id: 1,
                song_id: 1,
            },
        ])
        .await;

    let r = song_filter_service.get_all().await;
    for s in r {
        println!(
            "Found all song filter <id:{:?}, filter_id:{:?}, song_id{:?}>",
            s.id, s.filter_id, s.song_id
        );
    }
    let r = song_filter_service.get_by_id(1).await;
    println!(
        "Found song filter by id <id:{:?}, filter_id:{:?}, song_id:{:?}>",
        r.id, r.filter_id, r.song_id
    );
    let r = song_filter_service.get_by_filter(2).await;
    for s in r {
        println!(
            "Found song filter by filter <id:{:?}, filter_id:{:?}, song_id:{:?}>",
            s.id, s.filter_id, s.song_id
        );
    }
    let r = song_filter_service.get_by_song(100001).await;
    for s in r {
        println!(
            "Found song filter by song <id:{:?}, filter_id:{:?}, song_id:{:?}>",
            s.id, s.filter_id, s.song_id
        );
    }
    Ok(())
}

//TODO:
// read modules and fix the smaller abomination
// implement domain (filters + settings + random) (just missing random, updates and deletes for basic crud)
// remove all tests from main
// try local web?
// add frontend
// make tauri alternative
