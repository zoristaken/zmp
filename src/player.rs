use std::io::BufReader;

use crate::{
    filter::{FilterRepository, FilterService},
    metadata::MetadataParser,
    setting::{SettingRepository, SettingService},
    song::{SongRepository, SongService},
    song_filter::{SongFilterRepository, SongFilterService},
};

struct Player {}

impl Player {
    pub fn new() -> Self {
        Self {}
    }

    //testing functionality for now
    pub fn play(&self, file_path: &str, volume: rodio::Float) -> anyhow::Result<()> {
        let stream_handle = rodio::DeviceSinkBuilder::open_default_sink()?;
        let mixer = stream_handle.mixer();

        let file = std::fs::File::open(file_path)?;
        let player = rodio::play(mixer, BufReader::new(file))?;
        player.set_volume(volume);

        println!("playing with volume of {:?}", volume);

        player.play();
        player.sleep_until_end();
        Ok(())
    }
}
pub struct PlayerService<S, So, F, Sf>
where
    S: SettingRepository,
    So: SongRepository,
    F: FilterRepository,
    Sf: SongFilterRepository,
{
    pub setting: SettingService<S>,
    pub song: SongService<So>,
    pub filter: FilterService<F>,
    pub song_filter: SongFilterService<Sf>,
    pub metadata_parser: MetadataParser,
    player: Player,
}

impl<R> PlayerService<R, R, R, R>
where
    R: SettingRepository + SongRepository + FilterRepository + SongFilterRepository + Clone,
{
    pub fn new(repos: R) -> Self {
        Self {
            setting: SettingService::new(repos.clone()),
            song: SongService::new(repos.clone()),
            filter: FilterService::new(repos.clone()),
            song_filter: SongFilterService::new(repos.clone()),
            metadata_parser: MetadataParser::new(),
            player: Player::new(),
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

    pub async fn play(&self, file_path: &str, volume: rodio::Float) -> anyhow::Result<()> {
        if volume == 0.0 {
            let saved_volume = self.setting.get_saved_volume_value().await;
            self.player.play(file_path, saved_volume)?;
            return Ok(());
        }

        self.player.play(file_path, volume)?;

        if !self.setting.is_repeat_flag().await {
            return Ok(());
        }

        loop {
            self.player.play(file_path, volume)?;
        }
    }
}
