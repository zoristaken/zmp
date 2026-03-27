use crate::domain::{Filter, FilterRepository};

pub struct FilterService<R: FilterRepository> {
    repo: R,
}

impl<R: FilterRepository> FilterService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn set(&self, name: &str) {
        self.repo.set(name).await;
    }

    pub async fn get_all(&self) -> Vec<Filter> {
        self.repo.get_all().await
    }

    pub async fn get_by_name(&self, name: &str) -> Filter {
        self.repo.get_by_name(name).await
    }

    pub async fn get_by_id(&self, filter_id: i64) -> Filter {
        self.repo.get_by_id(filter_id).await
    }
}
