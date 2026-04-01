#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Filter {
    pub id: i32,
    pub name: String,
}

pub trait FilterRepository: Send + Sync {
    async fn add(&self, name: &str);
    async fn get_all(&self) -> Vec<Filter>;
    async fn get_by_name(&self, name: &str) -> Filter;
    async fn get_by_id(&self, filter_id: i32) -> Filter;
}

pub struct FilterService<R: FilterRepository> {
    repo: R,
}

impl<R: FilterRepository> FilterService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn add(&self, name: &str) {
        self.repo.add(name).await;
    }

    pub async fn get_all(&self) -> Vec<Filter> {
        self.repo.get_all().await
    }

    pub async fn get_by_name(&self, name: &str) -> Filter {
        self.repo.get_by_name(name).await
    }

    pub async fn get_by_id(&self, filter_id: i32) -> Filter {
        self.repo.get_by_id(filter_id).await
    }
}
