mod config;
mod filter;
mod metadata;
mod player;
mod setting;
mod song;
mod song_filter;
mod sqlite;

use config::Config;
use rand::prelude::*;
use song_filter::SongFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::new()?;
    let sqlite = sqlite::SqliteDb::new(&config.database_url).await?;

    //testing for now
    let player_service = player::PlayerService::new(sqlite);

    player_service
        .setting
        .set_music_folder_path("/home/z/Music")
        .await;

    player_service
        .setting
        .set_random_play(!player_service.setting.is_random_play().await)
        .await;

    println!(
        "set random play to:{}",
        player_service.setting.is_random_play().await
    );

    player_service
        .setting
        .set_repeat_flag(!player_service.setting.is_repeat_flag().await)
        .await;

    println!(
        "set repeat play to:{}",
        player_service.setting.is_repeat_flag().await
    );

    player_service.setting.set_next_keybind("f3").await;
    let next_kb = player_service.setting.get_next_keybind().await;
    player_service.setting.set_previous_keybind("f1").await;
    let previous_kb = player_service.setting.get_previous_keybind().await;
    player_service.setting.set_play_stop_keybind("f2").await;
    let play_kb = player_service.setting.get_play_stop_keybind().await;
    player_service.setting.set_settings_keybind("f5").await;
    let settings_kb = player_service.setting.get_settings_keybind().await;
    player_service.setting.set_random_keybind("f4").await;
    let random_kb = player_service.setting.get_random_keybind().await;

    println!(
        "KEYBINDS\nnext:{:?}\nprevious:{:?}\nplay/stop:{:?}\nsettings:{:?}\nrandom:{:?}",
        next_kb, previous_kb, play_kb, settings_kb, random_kb
    );

    player_service.process_music_folder().await;

    let songs = player_service.song.list_songs().await;
    let mut rng = rand::rng();

    let id = rng.random_range(1..songs.len());

    // for s in &songs {
    //     println!("All songs list: {:#?}", s);
    // }
    let searchable = vec!["left", "behind"];
    let r = player_service.song.search_by(&songs, searchable, 10).await;
    for s in r {
        println!("1 Songs List: {:#?}", s);
    }

    let searchable = vec!["left", "behind"];
    let r = player_service.song.search_by_db(searchable, 10).await;
    for s in r {
        println!("2 Songs List: {:#?}", s);
    }

    let _ = player_service
        .song
        .get_by_title_artist("Sanctuary (Opening)".to_string(), "Utada".to_string());

    player_service
        .setting
        .set_saved_search_blob("left behind")
        .await;

    let blob = player_service.setting.get_saved_search_blob().await?;
    println!("db saved search blob: {:?}", blob);

    player_service.setting.set_saved_volume_value(0.7).await;

    let saved_volume = player_service.setting.get_saved_volume_value().await;
    println!("saved volume variable: {:?}", saved_volume);

    let _ = player_service.song.get_by_id(id as i32).await;

    let r = player_service
        .song
        .search_by_db(blob.split_whitespace().collect(), 1)
        .await;

    let _ = player_service
        .play(r.first().unwrap().file_path.as_str(), saved_volume)
        .await;
    println!("finished song");

    player_service.filter.add("trance").await;
    player_service.filter.add("metal").await;
    player_service.filter.add("oldschool").await;
    player_service.filter.add("favorite").await;

    let r = player_service.filter.get_all().await;
    for s in r {
        println!("Found all filter <id:{:?}, name:{:?}>", s.id, s.name);
    }

    let r = player_service.filter.get_by_name("favorite").await;
    println!("Found filter by name <id:{:?}, name:{:?}>", r.id, r.name);
    let r = player_service.filter.get_by_id(1).await;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);
    let r = player_service.filter.get_by_id(2).await;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);
    let r = player_service.filter.get_by_id(3).await;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);
    let r = player_service.filter.get_by_id(4).await;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);

    player_service
        .song_filter
        .add(SongFilter {
            id: 0,
            filter_id: 1,
            song_id: 100001,
        })
        .await;

    player_service
        .song_filter
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

    let r = player_service.song_filter.get_all().await;
    for s in r {
        println!(
            "Found all song filter <id:{:?}, filter_id:{:?}, song_id{:?}>",
            s.id, s.filter_id, s.song_id
        );
    }
    let r = player_service.song_filter.get_by_id(1).await;
    println!(
        "Found song filter by id <id:{:?}, filter_id:{:?}, song_id:{:?}>",
        r.id, r.filter_id, r.song_id
    );
    let r = player_service.song_filter.get_by_filter(2).await;
    for s in r {
        println!(
            "Found song filter by filter <id:{:?}, filter_id:{:?}, song_id:{:?}>",
            s.id, s.filter_id, s.song_id
        );
    }
    let r = player_service.song_filter.get_by_song(54).await;
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
