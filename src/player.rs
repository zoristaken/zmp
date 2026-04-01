use std::io::BufReader;

use crate::{
    filter::{FilterRepository, FilterService},
    metadata::MetadataParser,
    setting::{SettingRepository, SettingService},
    song::{SongRepository, SongService},
    song_filter::{SongFilterRepository, SongFilterService},
};

pub struct Player {}

impl Player {
    //testing functionality for now
    pub fn play(file_path: &str) -> anyhow::Result<()> {
        let stream_handle = rodio::DeviceSinkBuilder::open_default_sink()?;
        let mixer = stream_handle.mixer();

        let file = std::fs::File::open(file_path)?;
        let player = rodio::play(mixer, BufReader::new(file))?;
        player.set_volume(0.5);

        player.play();
        player.sleep_until_end();
        Ok(())
    }
}

pub trait PlayerRepository:
    SettingRepository + SongRepository + FilterRepository + SongFilterRepository
{
}

impl<T> PlayerRepository for T where
    T: SettingRepository + SongRepository + FilterRepository + SongFilterRepository
{
}

pub struct PlayerService<R>
where
    R: PlayerRepository,
{
    pub setting: SettingService<R>,
    pub song: SongService<R>,
    pub filter: FilterService<R>,
    pub song_filter: SongFilterService<R>,
    pub metadata_parser: MetadataParser,
}

impl<R> PlayerService<R>
where
    R: PlayerRepository + Clone,
{
    pub fn new(repos: R) -> Self {
        Self {
            setting: SettingService::new(repos.clone()),
            song: SongService::new(repos.clone()),
            filter: FilterService::new(repos.clone()),
            song_filter: SongFilterService::new(repos.clone()),
            metadata_parser: MetadataParser::new(),
        }
    }

    //TODO: generalize the db and executor so we can pass the pool into different services
    //to allow for transaction commits across services/database tables queries
    //transaction required for the set_processed_music_folder and add_songs
    pub async fn process_music_folder(&self) {
        if !self.setting.has_processed_music_folder().await {
            let folder_path = self.setting.get_music_folder_path().await;
            let songs = self
                .metadata_parser
                .parse_song_metadata(folder_path.as_str())
                .await;
            self.setting.set_processed_music_folder(true).await;
            self.song.add_songs(songs).await;
        } else {
            println!("already processed music folder")
        }
    }
}
