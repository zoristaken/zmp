use crate::domain::song;

mod config;
mod database;
mod domain;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let songs: Vec<domain::song::song::Song> = vec![
        domain::song::song::Song {
            id: 0,
            title: "Enter Sandman".to_string(),
            artist: "Metallica".to_string(),
            album: "Metallica".to_string(),
            remix: "".to_string(),
            release_year: 1991,
            //search blob gathers all the searchable keywords
            //this includes: title(song name), artist, album, year and all tags
            //ordered by(?):
            //title, artist, tags, year, album
            //need to find the perfect order to minimize the number of comparisons needed
            //to return a match found (which search fields are more common)
            search_blob: "enter sandman metallica metal heavy metal old 1991 metallica".to_string(),
            // tags: vec![
            //     "metal".to_string(),
            //     "heavy metal".to_string(),
            //     "old".to_string(),
            // ],
        },
        domain::song::song::Song {
            id: 0,
            title: "Tuvan".to_string(),
            artist: "Gaia".to_string(),
            album: "".to_string(),
            remix: "Andy Blueman".to_string(),
            release_year: 1991,
            search_blob: "tuvan gaia trance favorite old 1991".to_string(),
            // tags: vec![
            //     "trance".to_string(),
            //     "favorite".to_string(),
            //     "old".to_string(),
            // ],
        },
    ];
    // let searchable = vec!["91"];

    // let results: Vec<&Song> = songs
    //     .iter()
    //     .filter(|s| {
    //         searchable
    //             .iter()
    //             .any(|x| !x.is_empty() && s.search_blob.contains(x))
    //     })
    //     .take(10)
    //     .collect();

    //let path = get_or_create_config_dir();
    let config = config::config::Config::new()?;
    println!("{:#?}", config.database_url);
    let sqlite = database::sqlite::SqliteDb::new(&config.database_url).await?;

    // id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    // name                    TEXT                NOT NULL,
    // artist                  TEXT,
    // release_year            INT2,
    // album                   TEXT,
    // remix                   TEXT
    // Now insert the row:
    for val in songs {
        let result = sqlx::query(
            "INSERT INTO song (title, artist, release_year, album, remix, search_blob)
                    VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING;",
        )
        .bind(val.title)
        .bind(val.artist)
        .bind(val.release_year)
        .bind(val.album)
        .bind(val.remix)
        .bind(val.search_blob)
        .execute(&sqlite.pool)
        .await;

        println!("{:#?}", result);
    }
    let songs_db: Vec<song::song::Song> = sqlx::query_as::<sqlx::Sqlite, song::song::Song>(
        "SELECT id, title, artist, release_year, album, remix, search_blob FROM song",
    )
    .fetch_all(&sqlite.pool)
    .await?;

    println!("size: {}", songs_db.len());

    for s in songs_db {
        println!("Found song: {:?}", s);
    }
    // let songs_db = sqlx::query("SELECT * FROM song")
    //     .fetch_all(&sqlite.pool)
    //     .await?;
    //println!("{:#?}", songs_db);
    Ok(())
}
