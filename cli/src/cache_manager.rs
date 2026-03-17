use crate::models::LyricElement;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const LYRICS_CACHE_TTL_SECS: u64 = 604800;
const LYRICS_CACHE_MAX_CAPACITY: u64 = 1000;
const SEARCH_CACHE_TTL_SECS: u64 = 86400;
const SEARCH_CACHE_MAX_CAPACITY: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultEntry {
    pub search_results: Vec<serde_json::Value>,
    pub found_title: String,
    pub found_artist: String,
    pub lyrics_url: String,
}

impl SearchResultEntry {
    pub fn new(
        search_results: Vec<serde_json::Value>,
        found_title: String,
        found_artist: String,
        lyrics_url: String,
    ) -> Self {
        Self {
            search_results,
            found_title,
            found_artist,
            lyrics_url,
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
            .time_to_live(Duration::from_secs(LYRICS_CACHE_TTL_SECS))
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
        CacheStats { total, valid: total }
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

    fn make_key(title: &str, artist: Option<&str>) -> String {
        let title_lower = title.to_lowercase().trim().to_string();
        let artist_lower = artist
            .map(|a| a.to_lowercase().trim().to_string())
            .unwrap_or_default();
        format!("{}|{}", title_lower, artist_lower)
    }

    pub async fn get(&self, title: &str, artist: Option<&str>) -> Option<SearchResultEntry> {
        let key = Self::make_key(title, artist);
        self.cache.get(&key).await
    }

    pub async fn insert(
        &self,
        title: &str,
        artist: Option<&str>,
        entry: SearchResultEntry,
    ) {
        let key = Self::make_key(title, artist);
        self.cache.insert(key, entry).await;
    }

    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
    }

    pub fn stats(&self) -> CacheStats {
        let total = self.cache.entry_count();
        CacheStats { total, valid: total }
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
