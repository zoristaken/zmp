use std::{fs::File, io::BufReader, time::Duration};

use anyhow::Context;
use rand::seq::IndexedRandom;
use rodio::Decoder;

use crate::song::Song;

pub struct Player {
    _stream_handle: rodio::MixerDeviceSink,
    player: rodio::Player,
    queue: Vec<Song>,
    current_index: Option<usize>,
    repeat: bool,
    shuffle: bool,
    volume: rodio::Float,
}

impl Player {
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
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
            _stream_handle: stream_handle,
            player,
            queue: Vec::new(),
            current_index,
            repeat,
            shuffle,
            volume,
        }
    }

    pub fn seek_to_seconds(&mut self, seconds: u64) -> anyhow::Result<()> {
        let target = Duration::from_secs(seconds);

        self.player
            .try_seek(target)
            .map_err(|e| anyhow::anyhow!("seek failed: {e}"))?;

        Ok(())
    }

    fn source_from_song(song: &Song) -> anyhow::Result<Decoder<BufReader<File>>> {
        let file = File::open(&song.file_path)
            .with_context(|| format!("failed to open file: {}", song.file_path))?;
        let source = Decoder::try_from(file)
            .with_context(|| format!("failed to decode file: {}", song.file_path))?;
        Ok(source)
    }

    fn append_song(&self, song: &Song) -> anyhow::Result<()> {
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

    pub fn set_queue(&mut self, songs: Vec<Song>) -> anyhow::Result<()> {
        self.queue = songs;

        if self.queue.is_empty() {
            self.current_index = None;
            self.player.clear();
            return Ok(());
        }

        self.current_index = Some(0);
        self.load_current_track(true)
    }

    pub fn play_song_at(&mut self, index: usize, start_playing: bool) -> anyhow::Result<()> {
        if index >= self.queue.len() {
            anyhow::bail!("index out of bounds");
        }

        self.current_index = Some(index);
        self.load_current_track(start_playing)
    }

    pub fn stop(&self) {
        self.player.stop();
    }

    pub fn play_pause(&self) {
        if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
    }

    pub fn play(&self) {
        self.player.play();
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn next_song(&mut self) -> anyhow::Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }

        if self.repeat {
            return self.load_current_track(true);
        }

        let next_index = match self.current_index {
            None => 0,
            Some(current) => {
                if self.shuffle {
                    let candidates: Vec<usize> =
                        (0..self.queue.len()).filter(|&i| i != current).collect();

                    if candidates.is_empty() {
                        current
                    } else {
                        let mut rng = rand::rng();
                        *candidates.choose(&mut rng).unwrap()
                    }
                } else {
                    let next = current + 1;
                    if next >= self.queue.len() {
                        0
                    } else {
                        next
                    }
                }
            }
        };

        self.current_index = Some(next_index);
        self.load_current_track(true)
    }

    pub fn previous(&mut self) -> anyhow::Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }

        if self.repeat {
            return self.load_current_track(true);
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

    pub fn set_repeat(&mut self, enabled: bool) {
        self.repeat = enabled;
    }

    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle = enabled;
    }

    pub fn current_song(&self) -> Option<&Song> {
        self.current_index.map(|i| &self.queue[i])
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }
}
