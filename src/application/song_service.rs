use crate::domain::{Song, SongRepository};

pub struct SongService<R: SongRepository> {
    repo: R,
}

impl<R: SongRepository> SongService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn add_songs(&self, songs: Vec<Song>) {
        self.repo.add_all(songs).await;
    }

    pub async fn list_songs(&self) -> Vec<Song> {
        self.repo.get_all().await
    }
}
