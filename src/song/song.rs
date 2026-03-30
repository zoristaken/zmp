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
    async fn search_by(
        &self,
        songs: &Vec<Song>,
        search: Vec<&str>,
        max_results: usize,
    ) -> Vec<Song>;
    async fn search_by_db(&self, words: Vec<&str>, max_results: i32) -> Vec<Song>;
}
