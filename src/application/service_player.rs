use crate::{
    application::{FilterService, SettingService, SongFilterService, SongService},
    filter::FilterRepository,
    metadata::MetadataParser,
    setting::SettingRepository,
    song::SongRepository,
    song_filter::SongFilterRepository,
};

pub struct PlayerService<Se, S, F, SF>
where
    Se: SettingRepository,
    S: SongRepository,
    F: FilterRepository,
    SF: SongFilterRepository,
{
    pub setting: SettingService<Se>,
    pub song: SongService<S>,
    pub filter: FilterService<F>,
    pub song_filter: SongFilterService<SF>,
    pub metadata_parser: MetadataParser,
}

impl<Se, S, F, SF> PlayerService<Se, S, F, SF>
where
    Se: SettingRepository,
    S: SongRepository,
    F: FilterRepository,
    SF: SongFilterRepository,
{
    pub fn new(setting_repo: Se, song_repo: S, filter_repo: F, song_filter_repo: SF) -> Self {
        Self {
            setting: SettingService::new(setting_repo),
            song: SongService::new(song_repo),
            filter: FilterService::new(filter_repo),
            song_filter: SongFilterService::new(song_filter_repo),
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
