use super::Filter;
pub trait FilterRepository: Send + Sync {
    async fn set(&self, name: &str);
    async fn get_all(&self) -> Vec<Filter>;
    async fn get_by_name(&self, name: &str) -> Filter;
    async fn get_by_id(&self, filter_id: i64) -> Filter;
}
