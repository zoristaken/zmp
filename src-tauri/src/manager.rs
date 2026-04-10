use std::{path::Path, sync::Mutex};

use sqlx::{Database, Pool, Sqlite};

use crate::{
    filter::{FilterRepository, FilterService},
    metadata::MetadataParser,
    player::Player,
    setting::{SettingRepository, SettingService},
    song::{SongRepository, SongService},
    song_filter::{SongFilterRepository, SongFilterService},
    song_mutation::{SongMutationRepository, SongMutationService},
    song_query::{SongQueryRepository, SongQueryService, SongWithFilters},
    sqlite::SqliteDb,
};
pub struct AppState {
    pub loaded_songs: Mutex<Vec<SongWithFilters>>,
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
        + SongQueryRepository<DB>
        + SongMutationRepository<DB>
        + HasPool<DB>,
{
    pub setting: SettingService<R, DB>,
    pub song: SongService<R, DB>,
    pub song_query: SongQueryService<R, DB>,
    pub song_mutation: SongMutationService<R, DB>,
    pub song_filter: SongFilterService<R, DB>,
    pub filter: FilterService<R, DB>,
    pub metadata_parser: MetadataParser,
    pub player: Mutex<Player>,
    pub pool: sqlx::Pool<DB>,
}

impl<R, DB> PlayerManager<R, DB>
where
    DB: Database,
    R: SettingRepository<DB>
        + SongRepository<DB>
        + FilterRepository<DB>
        + SongFilterRepository<DB>
        + SongQueryRepository<DB>
        + SongMutationRepository<DB>
        + HasPool<DB>
        + Clone,
{
    pub async fn new(repos: R) -> Self {
        let setting = SettingService::new(repos.clone());
        let index = setting.get_saved_index(&setting.pool).await;
        let shuffle = setting.is_random_play(&setting.pool).await;
        let repeat = setting.is_repeat_flag(&setting.pool).await;
        let volume = setting.get_saved_volume_value(&setting.pool).await;

        Self {
            setting,
            song: SongService::new(repos.clone()),
            song_query: SongQueryService::new(repos.clone()),
            song_mutation: SongMutationService::new(repos.clone()),
            song_filter: SongFilterService::new(repos.clone()),
            filter: FilterService::new(repos.clone()),
            metadata_parser: MetadataParser::new(),
            player: Mutex::new(Player::new(Some(index), shuffle, repeat, volume)),
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
        }
        Ok(())
    }
}
