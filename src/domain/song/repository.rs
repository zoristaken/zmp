use super::Song;
use crate::database::SqliteDb;
use std::sync::Arc;

pub trait SongRepository {
    fn new(pool: Arc<SqliteDb>) -> Self;
    async fn add_all(&self, songs: Vec<Song>);
    async fn get_all(&self) -> Vec<Song>;
    async fn search_by(
        &self,
        songs: &Vec<Song>,
        search: Vec<&str>,
        max_results: usize,
    ) -> Vec<Song>;
}
