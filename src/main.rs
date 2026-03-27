mod application;
mod config;
mod database;
mod domain;
mod infrastructure;

use application::SongService;
use infrastructure::SqliteSongRepository;
use std::sync::Arc;

use crate::domain::{Song, SongRepository};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::new()?;
    let sqlite = Arc::new(database::SqliteDb::new(&config.database_url).await?);
    let repo = SqliteSongRepository::new(sqlite.clone());
    let service = SongService::new(repo);

    //testing for now
    let songs = service.list_songs().await;
    service
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
    let r = service.search_by(&songs, searchable, 1).await;
    for s in r {
        println!("Songs List: {:?}", s);
    }

    let searchable = vec!["2026", "man", "insertion"];
    let r = service.search_by_db(searchable, 1).await;

    for s in r {
        println!("Songs List: {:?}", s);
    }

    Ok(())
}

//TODO:
// read modules and fix the smaller abomination
// implement domain (filters + settings + random)
// remove all tests from main
// try local web?
// add frontend
// make tauri alternative
