#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Setting {
    pub id: i32,
    pub key: String,
    pub value: String,
}

pub trait SettingRepository: Send + Sync {
    async fn set(&self, key: &str, value: &str);
    async fn get(&self, key: &str) -> anyhow::Result<Setting>;
}
