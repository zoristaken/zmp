use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use sqlx::{Database, Pool};
use tauri::{AppHandle, Emitter};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::Instant,
};

use crate::{
    metadata::{DiscoveredMusicFile, MetadataParser},
    setting::{MusicFolderSyncSettings, SettingRepository, SettingService},
    song::{Song, SongRepository, SongService},
    song_mutation::{SongMutationRepository, SongMutationService},
};

#[derive(Debug, Clone, Copy)]
enum WatcherSignal {
    RefreshConfig,
    SyncLibrary,
}

pub struct MusicFolderWatcher<R, DB>
where
    DB: Database,
    R: SettingRepository<DB> + SongRepository<DB> + SongMutationRepository<DB>,
{
    app: AppHandle,
    setting: SettingService<R, DB>,
    song: SongService<R, DB>,
    song_mutation: SongMutationService<R, DB>,
    metadata_parser: MetadataParser,
    pool: Pool<DB>,
    has_loaded_saved_state: AtomicBool,
    signal_tx: UnboundedSender<WatcherSignal>,
    signal_rx: tokio::sync::Mutex<Option<UnboundedReceiver<WatcherSignal>>>,
}

impl<R, DB> MusicFolderWatcher<R, DB>
where
    DB: Database,
    R: SettingRepository<DB> + SongRepository<DB> + SongMutationRepository<DB> + Clone,
{
    pub fn new(app: AppHandle, pool: Pool<DB>, repos: R) -> Self {
        let (signal_tx, signal_rx) = mpsc::unbounded_channel();

        Self {
            app,
            setting: SettingService::new(repos.clone()),
            song: SongService::new(repos.clone()),
            song_mutation: SongMutationService::new(repos),
            metadata_parser: MetadataParser::new(),
            pool,
            has_loaded_saved_state: AtomicBool::new(false),
            signal_tx,
            signal_rx: tokio::sync::Mutex::new(Some(signal_rx)),
        }
    }

    pub fn mark_loaded_saved_state(&self) {
        if !self.has_loaded_saved_state.swap(true, Ordering::Relaxed) {
            let _ = self.signal_tx.send(WatcherSignal::SyncLibrary);
        }
    }

    pub fn mark_loaded_saved_state_without_sync(&self) {
        self.has_loaded_saved_state.store(true, Ordering::Relaxed);
    }

    fn has_loaded_saved_state(&self) -> bool {
        self.has_loaded_saved_state.load(Ordering::Relaxed)
    }

    pub fn refresh_watch_config(&self) {
        let _ = self.signal_tx.send(WatcherSignal::RefreshConfig);
    }

    fn emit_library_changed(&self) -> anyhow::Result<()> {
        self.app.emit("library-changed", ())?;
        Ok(())
    }

    fn should_sync_for_event(event: &Event, watched_path: &Path) -> bool {
        if matches!(event.kind, EventKind::Access(_)) {
            return false;
        }

        event.paths.iter().any(|path| {
            if path.starts_with(watched_path) {
                return true;
            }

            path.parent()
                .map(|parent| parent.starts_with(watched_path))
                .unwrap_or(false)
        })
    }

    fn songs_match_metadata(&self, existing: &Song, scanned: &Song) -> bool {
        existing.title == scanned.title
            && existing.artist == scanned.artist
            && existing.release_year == scanned.release_year
            && existing.album == scanned.album
            && existing.remix == scanned.remix
            && existing.search_blob == scanned.search_blob
            && existing.file_path == scanned.file_path
            && existing.duration == scanned.duration
            && existing.extension == scanned.extension
            && existing.file_size == scanned.file_size
            && existing.file_modified_millis == scanned.file_modified_millis
    }

    fn song_matches_discovered_file(
        &self,
        existing: &Song,
        discovered: &DiscoveredMusicFile,
    ) -> bool {
        existing.file_size == discovered.file_size
            && existing.file_modified_millis == discovered.file_modified_millis
    }

    async fn get_music_folder_sync_settings(&self) -> anyhow::Result<MusicFolderSyncSettings> {
        self.setting
            .get_music_folder_sync_settings(&self.pool)
            .await
    }

    fn resolve_music_folder_watch_path(settings: &MusicFolderSyncSettings) -> Option<PathBuf> {
        if !settings.has_processed_music_folder {
            return None;
        }

        let folder_path = settings.music_folder_path.trim();

        if folder_path.is_empty() {
            return None;
        }

        std::fs::canonicalize(Path::new(folder_path)).ok()
    }

    async fn music_folder_watch_path(&self) -> Option<PathBuf> {
        match self.get_music_folder_sync_settings().await {
            Ok(settings) => Self::resolve_music_folder_watch_path(&settings),
            Err(err) => {
                log::error!("Failed to load music folder watcher settings: {err}");
                None
            }
        }
    }

    async fn sync_music_folder_library(&self) -> anyhow::Result<bool> {
        let settings = self.get_music_folder_sync_settings().await?;
        let folder_path = settings.music_folder_path.trim().to_string();

        if !settings.has_processed_music_folder || folder_path.is_empty() {
            return Ok(false);
        }

        let discovered_files = match self
            .metadata_parser
            .discover_music_files(Path::new(&folder_path))
        {
            Ok(files) => files,
            Err(err) => {
                log::error!("Failed to auto-sync music folder: {err}");
                return Ok(false);
            }
        };

        let mut tx = self.pool.begin().await?;
        let existing_songs = self.song.list_songs(&mut tx).await?;
        let mut existing_songs_by_path = existing_songs
            .into_iter()
            .map(|song| (song.file_path.clone(), song))
            .collect::<HashMap<_, _>>();
        let mut changed = false;

        for discovered_file in discovered_files {
            match existing_songs_by_path.remove(&discovered_file.file_path) {
                Some(existing_song) => {
                    if self.song_matches_discovered_file(&existing_song, &discovered_file) {
                        continue;
                    }

                    match self
                        .metadata_parser
                        .parse_discovered_music_file(&discovered_file)
                    {
                        Ok(scanned_song) => {
                            if !self.songs_match_metadata(&existing_song, &scanned_song) {
                                let mut updated_song = scanned_song;
                                updated_song.id = existing_song.id;
                                self.song.update_song(&mut tx, updated_song).await?;
                                changed = true;
                            }
                        }
                        Err(err) => {
                            log::warn!(
                                "Failed to parse changed music file {}: {err}",
                                discovered_file.file_path
                            );
                        }
                    }
                }
                None => match self
                    .metadata_parser
                    .parse_discovered_music_file(&discovered_file)
                {
                    Ok(scanned_song) => {
                        self.song.add_song(&mut tx, scanned_song).await?;
                        changed = true;
                    }
                    Err(err) => {
                        log::warn!(
                            "Failed to parse new music file {}: {err}",
                            discovered_file.file_path
                        );
                    }
                },
            }
        }

        for removed_song in existing_songs_by_path.into_values() {
            self.song.remove_song(&mut tx, removed_song.id).await?;
            changed = true;
        }

        if !changed {
            tx.rollback().await?;
            return Ok(false);
        }

        self.song_mutation
            .refresh_all_song_search_blobs(&mut tx)
            .await?;
        tx.commit().await?;

        Ok(true)
    }

    async fn refresh_music_folder_watcher(
        &self,
        watcher: &mut Option<RecommendedWatcher>,
        watched_path: &mut Option<PathBuf>,
    ) {
        let next_path = self.music_folder_watch_path().await;

        if next_path == *watched_path {
            return;
        }

        *watcher = None;
        *watched_path = None;

        let Some(path) = next_path else {
            return;
        };

        let event_signal_tx = self.signal_tx.clone();
        let watched_root = path.clone();
        let mut next_watcher =
            match notify::recommended_watcher(move |result: notify::Result<Event>| match result {
                Ok(event) => {
                    if Self::should_sync_for_event(&event, &watched_root) {
                        let _ = event_signal_tx.send(WatcherSignal::SyncLibrary);
                    }
                }
                Err(err) => log::warn!("Music folder watcher event failed: {err}"),
            }) {
                Ok(watcher) => watcher,
                Err(err) => {
                    log::error!("Failed to create music folder watcher: {err}");
                    return;
                }
            };

        if let Err(err) = next_watcher.watch(&path, RecursiveMode::Recursive) {
            log::error!("Failed to watch music folder {}: {err}", path.display());
            return;
        }

        *watched_path = Some(path);
        *watcher = Some(next_watcher);
    }

    pub async fn run(self: Arc<Self>) {
        let mut signal_rx = self
            .signal_rx
            .lock()
            .await
            .take()
            .expect("music folder watcher can only be started once");
        let mut watcher = None;
        let mut watched_path = None;
        let debounce_window = Duration::from_millis(750);
        let debounce = tokio::time::sleep(Duration::from_secs(3600));
        tokio::pin!(debounce);
        let mut debounce_armed = false;

        self.refresh_music_folder_watcher(&mut watcher, &mut watched_path)
            .await;

        loop {
            tokio::select! {
                signal = signal_rx.recv() => {
                    let Some(signal) = signal else {
                        break;
                    };

                    let mut refresh_requested = matches!(signal, WatcherSignal::RefreshConfig);
                    let mut sync_requested = matches!(signal, WatcherSignal::SyncLibrary);

                    while let Ok(next_signal) = signal_rx.try_recv() {
                        match next_signal {
                            WatcherSignal::RefreshConfig => refresh_requested = true,
                            WatcherSignal::SyncLibrary => sync_requested = true,
                        }
                    }

                    if refresh_requested {
                        self.refresh_music_folder_watcher(&mut watcher, &mut watched_path)
                            .await;
                    }

                    if sync_requested {
                        debounce.as_mut().reset(Instant::now() + debounce_window);
                        debounce_armed = true;
                    }
                }
                _ = &mut debounce, if debounce_armed => {
                    debounce_armed = false;

                    match self.sync_music_folder_library().await {
                        Ok(true) => {
                            if self.has_loaded_saved_state() {
                                if let Err(err) = self.emit_library_changed() {
                                    log::error!("Failed to emit library-changed event: {err}");
                                }
                            }
                        }
                        Ok(false) => {}
                        Err(err) => log::error!("Failed to live-sync music folder: {err}"),
                    }
                }
            }
        }
    }
}
