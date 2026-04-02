use std::{io::BufReader, marker::PhantomData, path::Path};

use crate::{
    filter::{FilterRepository, FilterService},
    metadata::MetadataParser,
    setting::{SettingRepository, SettingService},
    song::{SongRepository, SongService},
    song_filter::{SongFilterRepository, SongFilterService},
    sqlite::{HasPool, RepositoryDb},
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
pub struct PlayerService<S, So, F, Sf, DB>
where
    DB: RepositoryDb,
    S: SettingRepository<DB>,
    So: SongRepository<DB>,
    F: FilterRepository<DB>,
    Sf: SongFilterRepository<DB>,
{
    pub setting: SettingService<S, DB>,
    pub song: SongService<So, DB>,
    pub filter: FilterService<F, DB>,
    pub song_filter: SongFilterService<Sf, DB>,
    pub metadata_parser: MetadataParser,
    player: Player,
    pub pool: sqlx::Pool<DB>,
    _db: std::marker::PhantomData<DB>,
}

impl<R, DB> PlayerService<R, R, R, R, DB>
where
    DB: RepositoryDb,
    R: SettingRepository<DB>
        + SongRepository<DB>
        + FilterRepository<DB>
        + SongFilterRepository<DB>
        + HasPool<DB>
        + Clone,
{
    pub fn new(repos: R) -> Self {
        Self {
            setting: SettingService::new(repos.clone()),
            song: SongService::new(repos.clone()),
            filter: FilterService::new(repos.clone()),
            song_filter: SongFilterService::new(repos.clone()),
            metadata_parser: MetadataParser::new(),
            player: Player::new(),
            pool: repos.pool().clone(),
            _db: PhantomData,
        }
    }

    pub async fn process_music_folder(&self) -> anyhow::Result<()> {
        if !self.setting.has_processed_music_folder(&self.pool).await {
            let folder_path = self.setting.get_music_folder_path(&self.pool).await?;
            let songs = self
                .metadata_parser
                .parse_song_metadata(Path::new(&folder_path))?;

            let mut tx = self.pool.begin().await?;
            self.setting
                .set_processed_music_folder(&mut tx, true)
                .await?;
            self.song.add_songs(&mut tx, songs).await?;
            tx.commit().await?
        } else {
            println!("already processed music folder")
        }
        Ok(())
    }

    pub async fn play(&self, file_path: &str, volume: rodio::Float) -> anyhow::Result<()> {
        if volume == 0.0 {
            let saved_volume = self.setting.get_saved_volume_value(&self.pool).await;
            self.player.play(file_path, saved_volume)?;
            return Ok(());
        }

        self.player.play(file_path, volume)?;
        Ok(())
    }
}
