use super::Setting;

pub trait SettingRepository: Send + Sync {
    async fn set(&self, key: &str, value: &str);
    async fn get(&self, key: &str) -> anyhow::Result<Setting>;
}
