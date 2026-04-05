use std::{path::Path, sync::atomic::AtomicBool};

use sqlx::{Database, Pool, Sqlite};

use crate::{
    filter::{FilterRepository, FilterService},
    metadata::MetadataParser,
    player::Player,
    setting::{SettingRepository, SettingService},
    song::{SongRepository, SongService},
    song_filter::{SongFilterRepository, SongFilterService},
    sqlite::SqliteDb,
};

//pub type AppPlayerManager = PlayerManager<SqliteDb, SqliteDb, SqliteDb, SqliteDb, Sqlite>;
pub struct AppState {
    pub first_time: AtomicBool,
    pub zmp: PlayerManager<SqliteDb, Sqlite>,
}

pub trait HasPool<DB: Database> {
    fn pool(&self) -> &Pool<DB>;
}

pub struct PlayerManager<R, DB>
where
    DB: Database,
    R: SettingRepository<DB>
        + SongRepository<DB>
        + FilterRepository<DB>
        + SongFilterRepository<DB>
        + HasPool<DB>,
{
    pub setting: SettingService<R, DB>,
    pub song: SongService<R, DB>,
    pub filter: FilterService<R, DB>,
    pub song_filter: SongFilterService<R, DB>,
    pub metadata_parser: MetadataParser,
    pub player: Player,
    pub pool: sqlx::Pool<DB>,
}

impl<R, DB> PlayerManager<R, DB>
where
    DB: Database,
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
}
