use crate::domain::{Setting, SettingRepository};

pub struct SettingService<R: SettingRepository> {
    repo: R,
}

impl<R: SettingRepository> SettingService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn set(&self, key: &str, value: &str) {
        self.repo.set(key, value).await
    }

    pub async fn get(&self, key: &str) -> Setting {
        self.repo.get(key).await
    }
}
