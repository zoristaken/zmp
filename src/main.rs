mod application;
mod config;
mod database;
mod filter;
mod metadata;
mod player;
mod setting;
mod song;
mod song_filter;

use application::FilterService;
use application::SettingService;
use application::SongFilterService;
use application::SongService;
use config::Config;
use filter::SqliteFilterRepository;
use metadata::MetadataParser;
use setting::SqliteSettingRepository;
use song::SqliteSongRepository;
use song_filter::SqliteSongFilterRepository;

use std::sync::Arc;

use song_filter::SongFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::new()?;
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
    setting_service.set_music_folder_path("/home/z/Music").await;

    setting_service
        .set_random_play(!setting_service.is_random_play().await)
        .await;

    println!(
        "set random play to:{}",
        setting_service.is_random_play().await
    );

    setting_service.set_next_keybind("f3").await;
    let next_kb = setting_service.get_next_keybind().await;
    setting_service.set_previous_keybind("f1").await;
    let previous_kb = setting_service.get_previous_keybind().await;
    setting_service.set_play_stop_keybind("f2").await;
    let play_kb = setting_service.get_play_stop_keybind().await;
    setting_service.set_settings_keybind("f5").await;
    let settings_kb = setting_service.get_settings_keybind().await;
    setting_service.set_random_keybind("f4").await;
    let random_kb = setting_service.get_random_keybind().await;

    println!(
        "KEYBINDS\nnext:{:?}\nprevious:{:?}\nplay/stop:{:?}\nsettings:{:?}\nrandom:{:?}",
        next_kb, previous_kb, play_kb, settings_kb, random_kb
    );

    let r = setting_service.get_music_folder_path().await;
    println!("Found music path setting: {:?}", r);

    let metadata_parser = MetadataParser::new();

    //TODO: generalize the db and executor so we can pass the pool into different services
    //to allow for transaction commits across services/database tables queries
    if !setting_service.has_processed_music_folder().await {
        let songs = metadata_parser.parse_song_metadata(r.as_str()).await;
        setting_service.set_processed_music_folder(true).await;
        song_service.add_songs(songs).await;
    } else {
        println!("already processed music folder")
    }

    let songs = song_service.list_songs().await;
    for s in &songs {
        println!("All songs list: {:#?}", s);
    }
    // let searchable = vec!["tuvan"];

    // let r = song_service.search_by(&songs, searchable, 5).await;
    // for s in r {
    //     println!("1 Songs List: {:#?}", s);
    // }

    let searchable = vec!["tuvan"];
    let r = song_service.search_by_db(searchable, 1).await;
    for s in r {
        println!("2 Songs List: {:#?}", s);
        println!(
            "playing {:?} by {:?} for {:?} seconds",
            s.title, s.artist, s.duration
        );
        let _ = player::Player::play(&s.file_path);
        println!("finished song")
    }

    let r = song_service.get_by_id(1).await;
    let _ = player::Player::play(&r.file_path);

    filter_service.add("trance").await;
    filter_service.add("metal").await;
    filter_service.add("oldschool").await;
    filter_service.add("favorite").await;

    let r = filter_service.get_all().await;
    for s in r {
        println!("Found all filter <id:{:?}, name:{:?}>", s.id, s.name);
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
    let r = song_filter_service.get_by_song(54).await;
    for s in r {
        println!(
            "Found song filter by song <id:{:?}, filter_id:{:?}, song_id:{:?}>",
            s.id, s.filter_id, s.song_id
        );
    }
    Ok(())
}

//TODO:
// read modules
// move tests into actual tests (perhaps db mocks + actual dev db test)
// add actual error logic + remove .unwrap()
// check transaction TODO
// try local web?
// add frontend
// make tauri alternative
