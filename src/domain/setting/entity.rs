#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Setting {
    pub id: i64,
    pub key: String,
    pub value: String,
}
