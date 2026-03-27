#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct SongFilter {
    pub id: i64,
    pub song_id: i64,
    pub filter_id: i64,
}
