#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct SongFilter {
    pub id: i32,
    pub song_id: i32,
    pub filter_id: i32,
}

pub trait SongFilterRepository: Send + Sync {
    async fn add(&self, song_filter: SongFilter);
    async fn add_multiple(&self, song_filters: Vec<SongFilter>);
    async fn get_all(&self) -> Vec<SongFilter>;
    async fn get_by_id(&self, id: i32) -> SongFilter;
    async fn get_by_filter(&self, filter_id: i32) -> Vec<SongFilter>;
    async fn get_by_song(&self, song_id: i32) -> Vec<SongFilter>;
}
