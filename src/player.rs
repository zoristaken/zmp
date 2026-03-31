use std::io::BufReader;

pub struct Player {}

impl Player {
    //testing functionality for now
    pub fn play(file_path: &str) -> anyhow::Result<()> {
        let stream_handle = rodio::DeviceSinkBuilder::open_default_sink()?;
        let mixer = stream_handle.mixer();

        let file = std::fs::File::open(file_path)?;
        let player = rodio::play(mixer, BufReader::new(file))?;
        player.set_volume(0.5);

        player.play();
        player.sleep_until_end();
        Ok(())
    }
}
