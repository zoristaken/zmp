use std::{path::Path, sync::Mutex};

use sqlx::{Database, Pool, Sqlite};

use crate::{
    filter::{FilterRepository, FilterService},
    metadata::MetadataParser,
    player::Player,
    setting::{SettingRepository, SettingService},
    song::{Song, SongRepository, SongService},
    song_filter::{SongFilterRepository, SongFilterService},
    song_mutation::{SongMutationRepository, SongMutationService},
    song_query::{SongQueryRepository, SongQueryService},
    sqlite::SqliteDb,
};
pub struct AppState {
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

async fn replace_library_and_reset_state<R, DB>(
    song_service: &SongService<R, DB>,
    setting_service: &SettingService<R, DB>,
    songs: Vec<Song>,
) -> anyhow::Result<()>
where
    DB: Database,
    R: SongRepository<DB> + SettingRepository<DB> + HasPool<DB>,
{
    let mut tx = song_service.pool.begin().await?;
    song_service.replace_songs(&mut tx, songs).await?;
    setting_service.set_saved_search_blob(&mut tx, "").await?;
    setting_service.set_saved_index(&mut tx, 0).await?;
    setting_service.set_current_song_seek(&mut tx, 0).await?;
    setting_service.set_play_pause_flag(&mut tx, false).await?;
    setting_service
        .set_processed_music_folder(&mut tx, true)
        .await?;
    tx.commit().await?;

    Ok(())
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
        let folder_path = self.setting.get_music_folder_path(&self.pool).await?;
        let songs = self
            .metadata_parser
            .parse_song_metadata(Path::new(&folder_path))?;

        replace_library_and_reset_state(&self.song, &self.setting, songs).await
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use crate::{
        filter::FilterService, setting::SettingService, song::SongService,
        song_filter::SongFilterService, sqlite::SqliteDb,
    };

    use super::replace_library_and_reset_state;

    fn song(title: &str, artist: &str, path: &str) -> crate::song::Song {
        crate::song::Song {
            id: 0,
            title: title.to_string(),
            artist: artist.to_string(),
            release_year: 1998,
            album: "Album".to_string(),
            remix: String::new(),
            search_blob: format!(
                "{} {} album 1998",
                title.to_lowercase(),
                artist.to_lowercase()
            ),
            file_path: path.to_string(),
            duration: 180,
            extension: "flac".to_string(),
        }
    }

    async fn setup_db() -> SqliteDb {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        SqliteDb { pool }
    }

    #[tokio::test]
    async fn replace_library_and_reset_state_replaces_songs_and_resets_playback_settings() {
        let sqlite = setup_db().await;
        let song_service = SongService::new(sqlite.clone());
        let setting_service = SettingService::new(sqlite.clone());

        song_service
            .add_songs(
                &song_service.pool,
                vec![song("Teardrop", "Massive Attack", "/music/teardrop.flac")],
            )
            .await
            .unwrap();

        setting_service
            .set_saved_search_blob(&setting_service.pool, "massive attack")
            .await
            .unwrap();
        setting_service
            .set_saved_index(&setting_service.pool, 7)
            .await
            .unwrap();
        setting_service
            .set_current_song_seek(&setting_service.pool, 123)
            .await
            .unwrap();
        setting_service
            .set_play_pause_flag(&setting_service.pool, true)
            .await
            .unwrap();
        setting_service
            .set_processed_music_folder(&setting_service.pool, false)
            .await
            .unwrap();

        replace_library_and_reset_state(
            &song_service,
            &setting_service,
            vec![song("Archangel", "Burial", "/music/archangel.flac")],
        )
        .await
        .unwrap();

        let songs = song_service.list_songs(&song_service.pool).await.unwrap();

        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].title, "Archangel");
        assert_eq!(songs[0].artist, "Burial");
        assert_eq!(
            setting_service
                .get_saved_search_blob(&setting_service.pool)
                .await
                .unwrap(),
            ""
        );
        assert_eq!(
            setting_service.get_saved_index(&setting_service.pool).await,
            0
        );
        assert_eq!(
            setting_service
                .get_current_song_seek(&setting_service.pool)
                .await,
            0
        );
        assert!(!setting_service.is_playing(&setting_service.pool).await);
        assert!(
            setting_service
                .has_processed_music_folder(&setting_service.pool)
                .await
        );
    }

    #[tokio::test]
    async fn replace_library_and_reset_state_clears_song_filter_links_but_keeps_filters() {
        let sqlite = setup_db().await;
        let song_service = SongService::new(sqlite.clone());
        let setting_service = SettingService::new(sqlite.clone());
        let filter_service = FilterService::new(sqlite.clone());
        let song_filter_service = SongFilterService::new(sqlite.clone());

        song_service
            .add_songs(
                &song_service.pool,
                vec![song("Teardrop", "Massive Attack", "/music/teardrop.flac")],
            )
            .await
            .unwrap();

        filter_service
            .add(&filter_service.pool, "ambient")
            .await
            .unwrap();
        song_filter_service
            .add(&song_filter_service.pool, 1, 1)
            .await
            .unwrap();

        replace_library_and_reset_state(
            &song_service,
            &setting_service,
            vec![song("Archangel", "Burial", "/music/archangel.flac")],
        )
        .await
        .unwrap();

        let filters = filter_service.get_all(&filter_service.pool).await.unwrap();
        let song_filters = song_filter_service
            .get_all(&song_filter_service.pool)
            .await
            .unwrap();

        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].name, "ambient");
        assert!(song_filters.is_empty());
    }
}
