use std::{io::BufReader, time::Duration};

use rodio::decoder;

use crate::song::Song;

pub struct Player {
    _stream_handle: rodio::MixerDeviceSink,
    player: rodio::Player,
}

impl Player {
    pub fn new() -> Self {
        let stream_handle = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
        let player = rodio::Player::connect_new(&stream_handle.mixer());

        Self {
            _stream_handle: stream_handle,
            player: player,
        }
    }

    pub fn fresh_queue(&self, songs: Vec<Song>) -> anyhow::Result<()> {
        self.player.clear();
        for song in songs {
            let file = std::fs::File::open(song.file_path)?;
            let reader = BufReader::new(file);
            let source = decoder::Decoder::new(reader)?;
            self.player.append(source);
        }
        self.player.play();
        self.player.sleep_until_end();
        Ok(())
    }

    pub fn play_pause(&self) {
        if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
    }

    pub fn next(&self) {
        self.player.skip_one();
    }

    //TODO: need to implement logic to keep previously played songs
    pub fn previous(&self) {}

    pub fn set_volume(&self, volume: rodio::Float) {
        self.player.set_volume(volume.clamp(0.0, 1.0));
    }

    pub fn seek(&self, abs_pos: Duration) -> anyhow::Result<()> {
        Ok(self.player.try_seek(abs_pos)?)
    }
}
