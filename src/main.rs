mod application;
mod config;
mod database;
mod domain;
mod infrastructure;

use application::SongService;
use infrastructure::SqliteSongRepository;
use std::sync::Arc;
use std::time::Instant;

use crate::domain::SongRepository;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::new()?;
    //println!("{:#?}", config.database_url);
    let sqlite = Arc::new(database::SqliteDb::new(&config.database_url).await?);
    let repo = SqliteSongRepository::new(sqlite.clone());
    //let repo2 = SqliteSongRepository::new(sqlite.clone());
    let service = SongService::new(repo);

    let songs = service.list_songs().await;

    let searchable = vec!["song amazing s", "radiohead", "1997"];
    let start = Instant::now(); // start timer
    let r = service.search_by(&songs, searchable, 1).await;
    let duration = start.elapsed(); // elapsed time
    for s in r {
        println!("Songs List: {:?}", s);
    }

    print!("it took: {:#?}", duration);

    // if songs.len() != 0 {
    //     return Ok(());
    // }

    // let songs: Vec<domain::Song> = vec![
    //     domain::Song {
    //         id: 0,
    //         title: "Enter Sandman".to_string(),
    //         artist: "Metallica".to_string(),
    //         album: "Metallica".to_string(),
    //         remix: "".to_string(),
    //         release_year: 1991,
    //         //search blob gathers all the searchable keywords
    //         //this includes: title(song name), artist, album, year and all tags
    //         //ordered by(?):
    //         //title, artist, tags, year, album
    //         //need to find the perfect order to minimize the number of comparisons needed
    //         //to return a match found (which search fields are more common)
    //         search_blob: "enter sandman metallica metal heavy metal old 1991 metallica".to_string(),
    //         // tags: vec![
    //         //     "metal".to_string(),
    //         //     "heavy metal".to_string(),
    //         //     "old".to_string(),
    //         // ],
    //         file_path: "/home/Music/Toranja - Laços.mp3".to_string(),
    //     },
    //     domain::Song {
    //         id: 0,
    //         title: "Tuvan".to_string(),
    //         artist: "Gaia".to_string(),
    //         album: "".to_string(),
    //         remix: "Andy Blueman".to_string(),
    //         release_year: 1991,
    //         search_blob: "tuvan gaia trance favorite old 1991".to_string(),
    //         file_path: "/home/Music/Toranja - Laços.mp3".to_string(),
    //         // tags: vec![
    //         //     "trance".to_string(),
    //         //     "favorite".to_string(),
    //         //     "old".to_string(),
    //         // ],
    //     },
    // ];

    // service.add_songs(songs).await;

    // let ss = service.list_songs().await;
    // for s in ss {
    //     println!("Songs List: {:?}", s);
    // }

    // id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    // name                    TEXT                NOT NULL,
    // artist                  TEXT,
    // release_year            INT2,
    // album                   TEXT,
    // remix                   TEXT
    // Now insert the row:
    //service.add_songs(songs).await;

    // let songs_db: Vec<domain::Song> = sqlx::query_as::<sqlx::Sqlite, domain::Song>(
    //     "SELECT id, title, artist, release_year, album, remix, search_blob, file_path FROM song",
    // )
    // .fetch_all(&sqlite.pool)
    // .await?;

    //println!("size: {}", songs.len());

    // let songs_db = sqlx::query("SELECT * FROM song")
    //     .fetch_all(&sqlite.pool)
    //     .await?;
    //println!("{:#?}", songs_db);
    Ok(())

    //TODO:
    // read modules and fix the abomination
    // abstract database access layer (repositories/services)
    // implement domain
    // remove all tests from main
    // high volume tests to compare sqlite vs vector
    // add settings
    // try local web?
    // add frontend
    // make tauri alternative
}
