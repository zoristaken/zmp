#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Filter {
    pub id: i64,
    pub name: String,
}
