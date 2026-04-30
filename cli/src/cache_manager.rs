use crate::models::{LyricElement, SearchPagination};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const LYRICS_CACHE_MAX_CAPACITY: u64 = 1000;
const SEARCH_CACHE_TTL_SECS: u64 = 86400;
const SEARCH_CACHE_MAX_CAPACITY: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultEntry {
    pub search_results: Vec<serde_json::Value>,
    pub found_title: String,
    pub found_artist: String,
    pub lyrics_url: String,
    pub pagination: Option<SearchPagination>,
}

impl SearchResultEntry {
    pub fn new(
        search_results: Vec<serde_json::Value>,
        found_title: String,
        found_artist: String,
        lyrics_url: String,
        pagination: Option<SearchPagination>,
    ) -> Self {
        Self {
            search_results,
            found_title,
            found_artist,
            lyrics_url,
            pagination,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total: u64,
    pub valid: u64,
}

#[derive(Debug, Clone)]
pub struct LyricsCache {
    cache: Cache<String, Vec<LyricElement>>,
}

impl LyricsCache {
    pub fn new() -> Self {
        let cache = Cache::builder()
            .max_capacity(LYRICS_CACHE_MAX_CAPACITY)
            .build();
        Self { cache }
    }

    pub async fn get(&self, url: &str) -> Option<Vec<LyricElement>> {
        self.cache.get(url).await
    }

    pub async fn insert(&self, url: String, lyrics: Vec<LyricElement>) {
        self.cache.insert(url, lyrics).await;
    }

    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
    }

    pub fn stats(&self) -> CacheStats {
        let total = self.cache.entry_count();
        CacheStats {
            total,
            valid: total,
        }
    }
}

impl Default for LyricsCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SearchCache {
    cache: Cache<String, SearchResultEntry>,
}

impl SearchCache {
    pub fn new() -> Self {
        let cache = Cache::builder()
            .max_capacity(SEARCH_CACHE_MAX_CAPACITY)
            .time_to_live(Duration::from_secs(SEARCH_CACHE_TTL_SECS))
            .build();
        Self { cache }
    }

    fn make_key_with_options(
        title: &str,
        artist: Option<&str>,
        search_type: &str,
        page: u32,
    ) -> String {
        let title_lower = title.to_lowercase().trim().to_string();
        let artist_lower = artist
            .map(|a| a.to_lowercase().trim().to_string())
            .unwrap_or_default();
        format!("{}|{}|{}|{}", title_lower, artist_lower, search_type, page)
    }

    pub async fn get(&self, title: &str, artist: Option<&str>) -> Option<SearchResultEntry> {
        self.get_with_options(title, artist, "title", 1).await
    }

    pub async fn get_with_options(
        &self,
        title: &str,
        artist: Option<&str>,
        search_type: &str,
        page: u32,
    ) -> Option<SearchResultEntry> {
        let key = Self::make_key_with_options(title, artist, search_type, page);
        self.cache.get(&key).await
    }

    pub async fn insert(&self, title: &str, artist: Option<&str>, entry: SearchResultEntry) {
        self.insert_with_options(title, artist, "title", 1, entry)
            .await;
    }

    pub async fn insert_with_options(
        &self,
        title: &str,
        artist: Option<&str>,
        search_type: &str,
        page: u32,
        entry: SearchResultEntry,
    ) {
        let key = Self::make_key_with_options(title, artist, search_type, page);
        self.cache.insert(key, entry).await;
    }

    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
    }

    pub fn stats(&self) -> CacheStats {
        let total = self.cache.entry_count();
        CacheStats {
            total,
            valid: total,
        }
    }
}

impl Default for SearchCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CacheManager {
    lyrics_cache: LyricsCache,
    search_cache: SearchCache,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            lyrics_cache: LyricsCache::new(),
            search_cache: SearchCache::new(),
        }
    }

    pub fn lyrics(&self) -> &LyricsCache {
        &self.lyrics_cache
    }

    pub fn search(&self) -> &SearchCache {
        &self.search_cache
    }

    pub async fn clear_all(&self) {
        self.lyrics_cache.clear().await;
        self.search_cache.clear().await;
    }

    pub fn stats(&self) -> (CacheStats, CacheStats) {
        (self.lyrics_cache.stats(), self.search_cache.stats())
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SearchPagination;

    fn sample_entry(title: &str, artist: &str, url: &str, page: u32) -> SearchResultEntry {
        SearchResultEntry::new(
            vec![serde_json::json!({
                "title": title,
                "artist": artist,
                "url": url,
                "matched": false
            })],
            title.to_string(),
            artist.to_string(),
            url.to_string(),
            Some(SearchPagination {
                current_page: page,
                total_pages: 10,
                has_next: page < 10,
            }),
        )
    }

    #[tokio::test]
    async fn search_cache_separates_entries_by_page_and_search_type() {
        let cache = SearchCache::new();
        let page1_entry = sample_entry("R", "Roselia", "/lyric/yb18072521/", 1);
        let page2_entry = sample_entry("FIRE BIRD", "Roselia", "/lyric/tu19061219/", 2);
        let artist_entry = sample_entry("Roselia", "Roselia", "/artist/22798/", 1);

        cache
            .insert_with_options("R", Some("Roselia"), "title", 1, page1_entry.clone())
            .await;
        cache
            .insert_with_options("R", Some("Roselia"), "title", 2, page2_entry.clone())
            .await;
        cache
            .insert_with_options("Roselia", None, "artist", 1, artist_entry.clone())
            .await;

        assert_eq!(
            cache
                .get_with_options("R", Some("Roselia"), "title", 1)
                .await
                .unwrap()
                .lyrics_url,
            page1_entry.lyrics_url
        );
        assert_eq!(
            cache
                .get_with_options("R", Some("Roselia"), "title", 2)
                .await
                .unwrap()
                .lyrics_url,
            page2_entry.lyrics_url
        );
        assert_eq!(
            cache
                .get_with_options("Roselia", None, "artist", 1)
                .await
                .unwrap()
                .lyrics_url,
            artist_entry.lyrics_url
        );
    }

    #[tokio::test]
    async fn search_cache_preserves_pagination_and_metadata() {
        let cache = SearchCache::new();
        let entry = sample_entry("R", "Roselia", "/lyric/yb18072521/", 3);

        cache
            .insert_with_options("R", Some("Roselia"), "title", 3, entry.clone())
            .await;

        let restored = cache
            .get_with_options("R", Some("Roselia"), "title", 3)
            .await
            .unwrap();

        assert_eq!(restored.found_title, entry.found_title);
        assert_eq!(restored.found_artist, entry.found_artist);
        assert_eq!(restored.lyrics_url, entry.lyrics_url);
        assert_eq!(restored.pagination, entry.pagination);
        assert_eq!(restored.search_results, entry.search_results);
    }
}
