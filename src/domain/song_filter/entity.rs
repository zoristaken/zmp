#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct SongFilter {
    pub id: i32,
    pub song_id: i32,
    pub filter_id: i32,
}
