use std::{fs::File, io::BufReader, time::Duration};

use anyhow::Context;
use rand::RngExt;
use rodio::Decoder;

use crate::song_query::SongWithFilters;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueSyncResult {
    pub current_index: Option<usize>,
    pub cleared_current_song: bool,
}

fn resolve_queue_sync(current_song_id: Option<i32>, songs: &[SongWithFilters]) -> QueueSyncResult {
    let current_index =
        current_song_id.and_then(|song_id| songs.iter().position(|song| song.song.id == song_id));

    QueueSyncResult {
        current_index,
        cleared_current_song: current_song_id.is_some() && current_index.is_none(),
    }
}

pub struct Player {
    _stream_handle: rodio::MixerDeviceSink,
    player: rodio::Player,
    queue: Vec<SongWithFilters>,
    current_index: Option<usize>,
    repeat: bool,
    shuffle: bool,
    volume: rodio::Float,
}

impl Player {
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn queue(&self) -> &[SongWithFilters] {
        &self.queue
    }

    pub fn new(
        current_index: Option<usize>,
        shuffle: bool,
        repeat: bool,
        volume: rodio::Float,
    ) -> Self {
        let stream_handle = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
        let player = rodio::Player::connect_new(stream_handle.mixer());
        player.set_volume(volume);

        Self {
            //stream handle is kept so it doesn't get dropped, as the mixer is needed for the player
            _stream_handle: stream_handle,
            player,
            queue: Vec::new(),
            current_index,
            repeat,
            shuffle,
            volume,
        }
    }

    pub fn load_saved_state(
        &mut self,
        is_shuffle: bool,
        is_repeat: bool,
        saved_index: usize,
        saved_seek: usize,
        saved_play_pause_flag: bool,
        songs: Vec<SongWithFilters>,
    ) -> anyhow::Result<Option<usize>> {
        self.set_shuffle(is_shuffle);
        self.set_repeat(is_repeat);
        let _ = self.set_queue(songs)?;

        if !self.queue.is_empty() {
            let index = saved_index.min(self.queue.len() - 1);
            self.play_song_at(index, saved_play_pause_flag, false)?;

            if saved_seek > 0 {
                self.seek_to_seconds(saved_seek as u64)?;
            }
        }

        Ok(self.current_index())
    }
    pub fn seek_pos(&self) -> Duration {
        self.player.get_pos()
    }

    pub fn seek_to_seconds(&mut self, seconds: u64) -> anyhow::Result<()> {
        let target = Duration::from_secs(seconds);

        self.player
            .try_seek(target)
            .map_err(|e| anyhow::anyhow!("seek failed: {e}"))?;

        Ok(())
    }

    fn source_from_song(song: &SongWithFilters) -> anyhow::Result<Decoder<BufReader<File>>> {
        let file = File::open(&song.song.file_path)
            .with_context(|| format!("failed to open file: {}", song.song.file_path))?;
        let source = Decoder::try_from(file)
            .with_context(|| format!("failed to decode file: {}", song.song.file_path))?;
        Ok(source)
    }

    fn append_song(&self, song: &SongWithFilters) -> anyhow::Result<()> {
        let source = Self::source_from_song(song)?;
        self.player.append(source);
        Ok(())
    }

    fn load_current_track(&self, start_playing: bool) -> anyhow::Result<()> {
        self.player.clear();

        if let Some(index) = self.current_index {
            let song = &self.queue[index];
            self.append_song(song)?;
            if start_playing {
                self.player.play();
            }
        }

        Ok(())
    }

    pub fn set_queue(&mut self, songs: Vec<SongWithFilters>) -> anyhow::Result<QueueSyncResult> {
        let current_song_id = self
            .current_index
            .and_then(|i| self.queue.get(i))
            .map(|song| song.song.id);

        self.queue = songs;

        if self.queue.is_empty() {
            let sync_result = QueueSyncResult {
                current_index: None,
                cleared_current_song: current_song_id.is_some(),
            };
            self.current_index = None;
            self.player.clear();
            return Ok(sync_result);
        }

        // Keep the current track selected when the queue changes so playback
        // doesn't drift to a different song just because the list was rebuilt.
        let sync_result = resolve_queue_sync(current_song_id, &self.queue);
        self.current_index = sync_result.current_index;

        if sync_result.cleared_current_song {
            self.player.clear();
        }

        Ok(sync_result)
    }

    pub fn play_song_at(
        &mut self,
        index: usize,
        start_playing: bool,
        ignore_if_same: bool,
    ) -> anyhow::Result<()> {
        if index >= self.queue.len() {
            return Ok(());
        }

        if ignore_if_same && self.current_index == Some(index) {
            return Ok(());
        }

        self.current_index = Some(index);
        self.load_current_track(start_playing)
    }

    pub fn play_pause(&self, should_play: bool) {
        if should_play {
            self.player.play();
        } else {
            self.player.pause();
        }
    }

    pub fn next_song(&mut self) -> anyhow::Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }

        if self.repeat {
            return self.load_current_track(true);
        }

        let len = self.queue.len();

        let next_index = match self.current_index {
            None => 0,
            Some(current) if self.shuffle && len > 1 => {
                let mut rng = rand::rng();
                let r = rng.random_range(0..len - 1);
                if r >= current {
                    r + 1
                } else {
                    r
                }
            }
            Some(current) => (current + 1) % len,
        };

        self.current_index = Some(next_index);
        self.load_current_track(true)
    }

    pub fn previous(&mut self) -> anyhow::Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }

        //if repeat is enabled, previous just needs to "restart" the song
        //seeking to the beginning allows that functionality without
        //having to reload the same track
        if self.repeat {
            return self.seek_to_seconds(0);
        }

        let prev_index = match self.current_index {
            None => 0,
            Some(0) => self.queue.len() - 1,
            Some(current) => current - 1,
        };

        self.current_index = Some(prev_index);
        self.load_current_track(true)
    }

    pub fn set_volume(&mut self, volume: rodio::Float) {
        self.volume = volume.clamp(0.0, 1.0);
        self.player.set_volume(self.volume);
    }

    pub fn get_volume(&mut self) -> rodio::Float {
        self.player.volume()
    }

    pub fn set_repeat(&mut self, flag: bool) {
        self.repeat = flag;
    }

    pub fn set_shuffle(&mut self, flag: bool) {
        self.shuffle = flag;
    }

    pub fn current_song(&self) -> Option<&SongWithFilters> {
        if self.queue.is_empty() {
            return None;
        }

        self.current_index.map(|i| &self.queue[i])
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn current_song_id(&self) -> anyhow::Result<Option<i32>> {
        Ok(self.current_song().map(|song| song.song.id))
    }
}

#[cfg(test)]
mod tests {
    use crate::{song::Song, song_query::SongWithFilters};

    use super::{resolve_queue_sync, QueueSyncResult};

    fn song(id: i32) -> SongWithFilters {
        SongWithFilters {
            song: Song {
                id,
                title: format!("song-{id}"),
                artist: "artist".to_string(),
                release_year: 1999,
                album: "album".to_string(),
                remix: String::new(),
                search_blob: format!("song-{id}"),
                file_path: format!("/music/{id}.mp3"),
                duration: 180,
                extension: "mp3".to_string(),
            },
            filters: Vec::new(),
        }
    }

    #[test]
    fn resolve_queue_sync_retargets_the_current_song_when_it_moves() {
        let result = resolve_queue_sync(Some(2), &[song(1), song(3), song(2)]);

        assert_eq!(
            result,
            QueueSyncResult {
                current_index: Some(2),
                cleared_current_song: false,
            }
        );
    }

    #[test]
    fn resolve_queue_sync_reports_when_the_current_song_disappears() {
        let result = resolve_queue_sync(Some(7), &[song(1), song(2), song(3)]);

        assert_eq!(
            result,
            QueueSyncResult {
                current_index: None,
                cleared_current_song: true,
            }
        );
    }

    #[test]
    fn resolve_queue_sync_does_not_clear_when_nothing_was_selected() {
        let result = resolve_queue_sync(None, &[song(1), song(2)]);

        assert_eq!(
            result,
            QueueSyncResult {
                current_index: None,
                cleared_current_song: false,
            }
        );
    }
}
