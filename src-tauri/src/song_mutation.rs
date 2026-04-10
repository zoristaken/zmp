use std::marker::PhantomData;

use async_trait::async_trait;
use sqlx::{Acquire, Database};

#[async_trait]
pub trait SongMutationRepository<DB>
where
    DB: Database,
{
    async fn refresh_song_search_blob<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn add_filter_to_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
        filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn remove_filter_from_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send;

    async fn refresh_all_song_search_blobs<'a, A>(&self, acquiree: A) -> anyhow::Result<usize>
    where
        A: Acquire<'a, Database = DB> + Send;
}

pub struct SongMutationService<R, DB>
where
    R: SongMutationRepository<DB>,
    DB: Database,
{
    _db: PhantomData<DB>,
    repo: R,
}

impl<R, DB> SongMutationService<R, DB>
where
    R: SongMutationRepository<DB>,
    DB: Database,
{
    pub fn new(repo: R) -> Self {
        Self {
            _db: PhantomData::default(),
            repo,
        }
    }

    pub async fn add_filter_to_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_id: i32,
        filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo
            .add_filter_to_song_and_reindex(acquiree, song_id, filter_id)
            .await
    }

    pub async fn remove_filter_from_song_and_reindex<'a, A>(
        &self,
        acquiree: A,
        song_filter_id: i32,
    ) -> anyhow::Result<bool>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo
            .remove_filter_from_song_and_reindex(acquiree, song_filter_id)
            .await
    }

    pub async fn refresh_all_song_search_blobs<'a, A>(&self, acquiree: A) -> anyhow::Result<usize>
    where
        A: Acquire<'a, Database = DB> + Send,
    {
        self.repo.refresh_all_song_search_blobs(acquiree).await
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use crate::{
        filter::FilterService,
        song::{Song, SongService},
        song_filter::SongFilterService,
        sqlite::SqliteDb,
    };

    use super::SongMutationService;

    fn song(id: i32, title: &str, artist: &str, year: i32, album: &str, search_blob: &str) -> Song {
        Song {
            id,
            title: title.to_string(),
            artist: artist.to_string(),
            release_year: year,
            album: album.to_string(),
            remix: String::new(),
            search_blob: search_blob.to_string(),
            file_path: format!("/music/{title}.mp3"),
            duration: 180,
            extension: "mp3".to_string(),
        }
    }

    async fn setup_db() -> SqliteDb {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        SqliteDb { pool }
    }

    async fn seed_song_and_filters(sqlite: &SqliteDb) {
        let song_service = SongService::new(sqlite.clone());
        let filter_service = FilterService::new(sqlite.clone());

        song_service
            .add_songs(
                &song_service.pool,
                vec![song(
                    0,
                    "Teardrop",
                    "Massive Attack",
                    1998,
                    "Mezzanine",
                    "teardrop massive attack mezzanine 1998",
                )],
            )
            .await
            .unwrap();

        filter_service
            .add(&filter_service.pool, "ambient")
            .await
            .unwrap();
        filter_service
            .add(&filter_service.pool, "favorites")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn add_filter_to_song_and_reindex_updates_search_results() {
        let sqlite = setup_db().await;
        seed_song_and_filters(&sqlite).await;
        let service = SongMutationService::new(sqlite.clone());
        let song_service = SongService::new(sqlite.clone());

        let before = song_service
            .search_by_db(&song_service.pool, &["ambient"], 10)
            .await
            .unwrap();
        assert!(before.is_empty());

        let updated = service
            .add_filter_to_song_and_reindex(&sqlite.pool, 1, 1)
            .await
            .unwrap();

        assert!(updated);

        let after = song_service
            .search_by_db(&song_service.pool, &["ambient"], 10)
            .await
            .unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].title, "Teardrop");
        assert!(after[0].search_blob.contains("ambient"));
    }

    #[tokio::test]
    async fn remove_filter_from_song_and_reindex_removes_filter_terms() {
        let sqlite = setup_db().await;
        seed_song_and_filters(&sqlite).await;
        let service = SongMutationService::new(sqlite.clone());
        let song_service = SongService::new(sqlite.clone());
        let song_filter_service = SongFilterService::new(sqlite.clone());

        service
            .add_filter_to_song_and_reindex(&sqlite.pool, 1, 1)
            .await
            .unwrap();

        let link = song_filter_service
            .get_by_song(&song_filter_service.pool, 1)
            .await
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        let removed = service
            .remove_filter_from_song_and_reindex(&sqlite.pool, link.id)
            .await
            .unwrap();

        assert!(removed);

        let after = song_service
            .search_by_db(&song_service.pool, &["ambient"], 10)
            .await
            .unwrap();
        assert!(after.is_empty());

        let song = song_service
            .get_song_by_id(&song_service.pool, 1)
            .await
            .unwrap();
        assert_eq!(song.search_blob, "teardrop massive attack mezzanine 1998");
    }

    #[tokio::test]
    async fn refresh_all_song_search_blobs_reindexes_existing_song_filter_links() {
        let sqlite = setup_db().await;
        seed_song_and_filters(&sqlite).await;
        let service = SongMutationService::new(sqlite.clone());
        let song_service = SongService::new(sqlite.clone());
        let song_filter_service = SongFilterService::new(sqlite.clone());

        song_filter_service
            .add(&song_filter_service.pool, 1, 2)
            .await
            .unwrap();

        let refreshed = service
            .refresh_all_song_search_blobs(&sqlite.pool)
            .await
            .unwrap();

        assert_eq!(refreshed, 1);

        let results = song_service
            .search_by_db(&song_service.pool, &["favorites"], 10)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Teardrop");
        assert!(results[0].search_blob.contains("favorites"));
    }
}
