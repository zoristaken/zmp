#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Song {
    pub id: i32,
    pub title: String,
    pub artist: String,
    pub release_year: i16,
    pub album: String,
    pub remix: String,
    pub search_blob: String,
    pub file_path: String,
    pub duration: i64,
}
pub trait SongRepository: Send + Sync {
    async fn add_all(&self, songs: Vec<Song>);
    async fn get_all(&self) -> Vec<Song>;
    async fn get_by_id(&self, id: i32) -> Song;
    async fn search_by(
        &self,
        songs: &Vec<Song>,
        search: Vec<&str>,
        max_results: usize,
    ) -> Vec<Song>;
    async fn search_by_db(&self, words: Vec<&str>, max_results: i32) -> Vec<Song>;
}

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

    pub async fn get_by_id(&self, id: i32) -> Song {
        self.repo.get_by_id(id).await
    }

    pub async fn search_by(
        &self,
        songs: &Vec<Song>,
        search: Vec<&str>,
        max_results: usize,
    ) -> Vec<Song> {
        self.repo.search_by(songs, search, max_results).await
    }

    pub async fn search_by_db(&self, words: Vec<&str>, max_results: i32) -> Vec<Song> {
        if words.len() == 0 {
            return vec![];
        }

        self.repo.search_by_db(words, max_results).await
    }
}
