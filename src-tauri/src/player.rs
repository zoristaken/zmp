use std::{fs::File, io::BufReader, time::Duration};

use anyhow::Context;
use rand::seq::IndexedRandom;
use rodio::{Decoder, Source};

use crate::{setting, song::Song};

pub struct Player {
    _stream_handle: rodio::MixerDeviceSink,
    player: rodio::Player,
    queue: Vec<Song>,
    current_index: Option<usize>,
    history: Vec<usize>,

    repeat_one: bool,
    shuffle: bool,
    volume: rodio::Float,
}

impl Player {
    pub fn new() -> Self {
        let stream_handle = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
        let player = rodio::Player::connect_new(&stream_handle.mixer());

        Self {
            _stream_handle: stream_handle,
            player,
            queue: Vec::new(),
            current_index: None,
            repeat_one: false,
            shuffle: false,
            volume: setting::DEFAULT_VOLUME,
            history: Vec::new(),
        }
    }

    fn source_from_song(song: &Song) -> anyhow::Result<Decoder<BufReader<File>>> {
        let file = File::open(&song.file_path)
            .with_context(|| format!("failed to open file: {}", song.file_path))?;
        let reader = BufReader::new(file);
        let source = Decoder::new(reader)
            .with_context(|| format!("failed to decode file: {}", song.file_path))?;
        Ok(source)
    }

    fn append_song(&self, song: &Song) -> anyhow::Result<()> {
        let source = Self::source_from_song(song)?;
        if self.repeat_one {
            self.player.append(source.repeat_infinite());
        } else {
            self.player.append(source);
        }
        self.player.set_volume(self.volume);
        Ok(())
    }

    fn load_current_track(&self) -> anyhow::Result<()> {
        self.player.clear();

        if let Some(index) = self.current_index {
            let song = &self.queue[index];
            self.append_song(song)?;
            self.player.play();
        }

        Ok(())
    }

    pub fn set_queue(&mut self, songs: Vec<Song>) -> anyhow::Result<()> {
        self.queue = songs;
        self.history.clear();

        if self.queue.is_empty() {
            self.current_index = None;
            self.player.clear();
            return Ok(());
        }

        self.current_index = Some(0);
        self.load_current_track()
    }

    pub fn play_song_at(&mut self, index: usize) -> anyhow::Result<()> {
        if index >= self.queue.len() {
            anyhow::bail!("index out of bounds");
        }

        if let Some(current) = self.current_index {
            self.history.push(current);
        }

        self.current_index = Some(index);
        self.load_current_track()
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

    pub fn next(&mut self) -> anyhow::Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }

        let next_index = match self.current_index {
            None => 0,
            Some(current) => {
                self.history.push(current);

                if self.shuffle {
                    let candidates: Vec<usize> =
                        (0..self.queue.len()).filter(|&i| i != current).collect();

                    if candidates.is_empty() {
                        current
                    } else {
                        let mut rng = rand::rng();
                        *candidates
                            .choose(&mut rng)
                            .expect("candidates is not empty")
                    }
                } else {
                    (current + 1) % self.queue.len()
                }
            }
        };

        self.current_index = Some(next_index);
        self.load_current_track()
    }

    pub fn previous(&mut self) -> anyhow::Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }

        // common UX: if current song has played a bit, restart it
        if self.player.get_pos() > Duration::from_secs(3) {
            return self.seek(Duration::ZERO);
        }

        let prev_index = if let Some(prev) = self.history.pop() {
            prev
        } else {
            match self.current_index {
                None => 0,
                Some(0) => self.queue.len() - 1,
                Some(current) => current - 1,
            }
        };

        self.current_index = Some(prev_index);
        self.load_current_track()
    }

    pub fn set_volume(&mut self, volume: rodio::Float) {
        self.volume = volume.clamp(0.0, 1.0);
        self.player.set_volume(self.volume);
    }

    pub fn seek(&self, abs_pos: Duration) -> anyhow::Result<()> {
        self.player.try_seek(abs_pos)?;
        Ok(())
    }

    pub fn set_repeat_one(&mut self, enabled: bool) -> anyhow::Result<()> {
        self.repeat_one = enabled;

        // reload current song so repeat mode takes effect immediately
        if self.current_index.is_some() {
            self.load_current_track()?;
        }

        Ok(())
    }

    pub fn toggle_repeat_one(&mut self) -> anyhow::Result<bool> {
        let enabled = !self.repeat_one;
        self.set_repeat_one(enabled)?;
        Ok(enabled)
    }

    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle = enabled;
    }

    pub fn toggle_shuffle(&mut self) -> bool {
        self.shuffle = !self.shuffle;
        self.shuffle
    }

    pub fn current_song(&self) -> Option<&Song> {
        self.current_index.map(|i| &self.queue[i])
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }
}
