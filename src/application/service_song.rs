use crate::domain::{Song, SongRepository};

pub struct SongService<R: SongRepository> {
    repo: R,
}

impl<R: SongRepository> SongService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn add_songs(&self, songs: Vec<Song>) {
        self.repo.add_all(songs).await
    }

    pub async fn list_songs(&self) -> Vec<Song> {
        self.repo.get_all().await
    }

    pub async fn search_by(
        &self,
        songs: &Vec<Song>,
        search: Vec<&str>,
        max_results: usize,
    ) -> Vec<Song> {
        self.repo.search_by(songs, search, max_results).await
    }

    pub async fn search_by_db(&self, words: Vec<&str>, max_results: i64) -> Vec<Song> {
        if words.len() == 0 {
            return vec![];
        }

        self.repo.search_by_db(words, max_results).await
    }
}
