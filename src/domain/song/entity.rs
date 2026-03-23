#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Song {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub release_year: i64,
    pub album: String,
    pub remix: String,
    pub search_blob: String,
    pub file_path: String,
}
