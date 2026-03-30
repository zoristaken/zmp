#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Filter {
    pub id: i32,
    pub name: String,
}
