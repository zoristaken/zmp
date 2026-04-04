use crate::{
    filter,
    manager::{AppState, HasPool},
    player,
    song_filter::SongFilter,
};

#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    message: String,
}

impl From<anyhow::Error> for ErrorResponse {
    fn from(e: anyhow::Error) -> Self {
        Self {
            message: e.to_string(),
        }
    }
}

#[tauri::command]
pub async fn main_test(state: tauri::State<'_, AppState>) -> anyhow::Result<(), ErrorResponse> {
    let sqlite = state.db.clone();

    let filter_service = filter::FilterService::new(sqlite.clone());

    let r = filter_service.get_all(sqlite.pool()).await?;
    for s in &r {
        println!("Found all filter <id:{:?}, name:{:?}>", s.id, s.name);
    }

    //testing for now
    let player_service = player::PlayerService::new(sqlite.clone());

    player_service
        .setting
        .set_music_folder_path(&player_service.pool, "/home/z/Music")
        .await?;

    player_service
        .setting
        .set_random_play(&player_service.pool, true)
        .await?;

    println!(
        "set random play to:{}",
        player_service
            .setting
            .is_random_play(&player_service.pool)
            .await
    );

    player_service
        .setting
        .set_repeat_flag(&player_service.pool, true)
        .await?;

    println!(
        "set repeat play to:{}",
        player_service
            .setting
            .is_repeat_flag(&player_service.pool)
            .await
    );

    player_service
        .setting
        .set_next_keybind(&player_service.pool, "f3")
        .await?;
    let next_kb = player_service
        .setting
        .get_next_keybind(&player_service.pool)
        .await?;
    player_service
        .setting
        .set_previous_keybind(&player_service.pool, "f1")
        .await?;
    let previous_kb = player_service
        .setting
        .get_previous_keybind(&player_service.pool)
        .await?;
    player_service
        .setting
        .set_play_stop_keybind(&player_service.pool, "f2")
        .await?;
    let play_kb = player_service
        .setting
        .get_play_stop_keybind(&player_service.pool)
        .await?;
    player_service
        .setting
        .set_settings_keybind(&player_service.pool, "f5")
        .await?;
    let settings_kb = player_service
        .setting
        .get_settings_keybind(&player_service.pool)
        .await?;
    player_service
        .setting
        .set_random_keybind(&player_service.pool, "f4")
        .await?;

    let random_kb = player_service
        .setting
        .get_random_keybind(&player_service.pool)
        .await?;

    println!(
        "KEYBINDS\nnext:{:?}\nprevious:{:?}\nplay/stop:{:?}\nsettings:{:?}\nrandom:{:?}",
        next_kb, previous_kb, play_kb, settings_kb, random_kb
    );

    player_service.process_music_folder().await?;

    player_service
        .setting
        .set_last_search_str(&player_service.pool, "komm juju")
        .await?;

    let search_str = player_service
        .setting
        .get_last_search_str(&player_service.pool)
        .await?;

    let songs = player_service.song.list_songs(&player_service.pool).await?;
    // for s in &songs {
    //     println!("All songs list id: {:#?}", s.id);
    // }

    let s: Vec<&str> = search_str.split(" ").collect();
    let searchable = &s;
    let max_results: i32 = 10;
    let r = player_service
        .song
        .search_by(&songs, searchable, max_results as usize)
        .await?;

    for s in r {
        println!("1 Songs List: {:#?}", s);
    }

    let r = player_service
        .song
        .search_by_db(&player_service.pool, searchable, max_results)
        .await?;

    let _ = player_service
        .song
        .search_by_db_alternative(&player_service.pool, searchable, 10)
        .await;

    for s in r {
        println!("2 Songs List: {:#?}", s);
    }

    //let mut rng = rand::rng();

    let _ = player_service.song.get_by_title_artist(
        &player_service.pool,
        "Sanctuary (Opening)",
        "Utada",
    );

    player_service
        .setting
        .set_saved_search_blob(&player_service.pool, "apoca path")
        .await?;

    let blob = player_service
        .setting
        .get_saved_search_blob(&player_service.pool)
        .await?;
    println!("db saved search blob: {:?}", blob);

    player_service
        .setting
        .set_saved_volume_value(&player_service.pool, 0.7)
        .await?;

    let saved_volume = player_service
        .setting
        .get_saved_volume_value(&player_service.pool)
        .await;
    println!("saved volume variable: {:?}", saved_volume);

    let searchable: Vec<&str> = blob.split_whitespace().collect();
    let _ = player_service
        .song
        .search_by_db(&player_service.pool, &searchable, 1)
        .await;

    match player_service
        .filter
        .add(&player_service.pool, "trance")
        .await
    {
        Ok(_) => println!("added trance filter"),
        Err(_) => println!("failed to add trance filter "),
    }
    match player_service
        .filter
        .add(&player_service.pool, "metal")
        .await
    {
        Ok(_) => println!("added metal filter"),
        Err(_) => println!("failed to add metal filter "),
    }
    match player_service
        .filter
        .add(&player_service.pool, "oldschool")
        .await
    {
        Ok(_) => println!("added oldschool filter"),
        Err(_) => println!("failed to add oldschool filter "),
    }
    match player_service
        .filter
        .add(&player_service.pool, "favorite")
        .await
    {
        Ok(_) => println!("added favorite filter"),
        Err(_) => println!("failed to add favorite filter"),
    }

    let r = player_service.filter.get_all(&player_service.pool).await?;
    for s in r {
        println!("Found all filter <id:{:?}, name:{:?}>", s.id, s.name);
    }

    let r = player_service
        .filter
        .get_by_name(&player_service.pool, "favorite")
        .await?;
    println!("Found filter by name <id:{:?}, name:{:?}>", r.id, r.name);
    let r = player_service
        .filter
        .get_by_id(&player_service.pool, 1)
        .await?;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);
    let r = player_service
        .filter
        .get_by_id(&player_service.pool, 2)
        .await?;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);
    let r = player_service
        .filter
        .get_by_id(&player_service.pool, 3)
        .await?;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);
    let r = player_service
        .filter
        .get_by_id(&player_service.pool, 4)
        .await?;
    println!("Found filter by id <id:{:?}, name:{:?}>", r.id, r.name);

    match player_service
        .song_filter
        .add(
            &player_service.pool,
            SongFilter {
                id: 0,
                filter_id: 1,
                song_id: 33,
            },
        )
        .await
    {
        Ok(_) => println!("managed to add filter id 1 to song id 33"),
        Err(_) => println!("filter id 1 and song id 33 already exist"),
    }

    match player_service
        .song_filter
        .add_multiple(
            &player_service.pool,
            vec![
                SongFilter {
                    id: 0,
                    filter_id: 4,
                    song_id: 43,
                },
                SongFilter {
                    id: 0,
                    filter_id: 1,
                    song_id: 1,
                },
            ],
        )
        .await
    {
        Ok(_) => println!("managed to add filter id 4 to song id 43 and filter id 1 to song id 1"),
        Err(_) => println!(
            "filter id 1 and song id 43 already exist or filter id 1 and song id 1 already exist"
        ),
    }

    let r = player_service
        .song_filter
        .get_all(&player_service.pool)
        .await?;
    for s in r {
        println!(
            "Found all song filter <id:{:?}, filter_id:{:?}, song_id{:?}>",
            s.id, s.filter_id, s.song_id
        );
    }
    let r = player_service
        .song_filter
        .get_by_id(&player_service.pool, 1)
        .await?;
    println!(
        "Found song filter by id <id:{:?}, filter_id:{:?}, song_id:{:?}>",
        r.id, r.filter_id, r.song_id
    );
    let r = player_service
        .song_filter
        .get_by_filter(&player_service.pool, 2)
        .await?;
    for s in r {
        println!(
            "Found song filter by filter <id:{:?}, filter_id:{:?}, song_id:{:?}>",
            s.id, s.filter_id, s.song_id
        );
    }
    let r = player_service
        .song_filter
        .get_by_song(&player_service.pool, 54)
        .await?;
    for s in r {
        println!(
            "Found song filter by song <id:{:?}, filter_id:{:?}, song_id:{:?}>",
            s.id, s.filter_id, s.song_id
        );
    }

    while player_service
        .setting
        .is_repeat_flag(&player_service.pool)
        .await
    {
        let id = rand::random_range(1..songs.len());
        match player_service
            .song
            .get_by_id(&player_service.pool, id as i32)
            .await
        {
            Ok(r) => {
                println!("playing <id={:?}> {:?} by {:?}", r.id, r.title, r.artist);
                let _ = player_service.play(r.file_path.as_str(), 0.0).await;
                println!("finished song");
            }
            Err(_) => println!("Hit one of the skipped ids lmao"),
        }
    }
    Ok(())
}

//TODO:
// read modules
// move tests into actual tests (perhaps db mocks + actual dev db test)
// add actual error logic
// build a proper player struct
// add frontend and tauri commands
// add actual missing backend -> frontend implementations
