//! 内存缓存管理器：基于 moka 的高性能异步缓存，管理歌词和搜索结果缓存

use crate::models::{LyricElement, SearchPagination};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 歌词缓存最大容量
const LYRICS_CACHE_MAX_CAPACITY: u64 = 1000;
/// 搜索缓存 TTL（秒），默认 24 小时
const SEARCH_CACHE_TTL_SECS: u64 = 86400;
/// 搜索缓存最大容量
const SEARCH_CACHE_MAX_CAPACITY: u64 = 500;

/// 搜索结果缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultEntry {
    /// 序列化的搜索结果列表
    pub search_results: Vec<serde_json::Value>,
    /// 找到的标题
    pub found_title: String,
    /// 找到的歌手
    pub found_artist: String,
    /// 歌词 URL
    pub lyrics_url: String,
    /// 分页信息
    pub pagination: Option<SearchPagination>,
}

impl SearchResultEntry {
    /// 创建新的搜索结果缓存条目
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

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// 总缓存条目数
    pub total: u64,
    /// 有效条目数
    pub valid: u64,
}

/// 歌词内存缓存（无 TTL，永不过期）
#[derive(Debug, Clone)]
pub struct LyricsCache {
    /// 以 URL 为键的歌词缓存
    cache: Cache<String, Vec<LyricElement>>,
}

impl LyricsCache {
    /// 创建新的歌词缓存，最大容量 1000 条
    pub fn new() -> Self {
        let cache = Cache::builder()
            .max_capacity(LYRICS_CACHE_MAX_CAPACITY)
            .build();
        Self { cache }
    }

    /// 根据 URL 获取缓存的歌词
    pub async fn get(&self, url: &str) -> Option<Vec<LyricElement>> {
        self.cache.get(url).await
    }

    /// 缓存歌词
    pub async fn insert(&self, url: String, lyrics: Vec<LyricElement>) {
        self.cache.insert(url, lyrics).await;
    }

    /// 根据 URL 失效单个缓存条目
    pub async fn invalidate(&self, url: &str) {
        self.cache.invalidate(url).await;
    }

    /// 清空所有歌词缓存
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
    }

    /// 获取歌词缓存统计
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

/// 搜索结果内存缓存（24 小时 TTL）
#[derive(Debug, Clone)]
pub struct SearchCache {
    /// 以查询键为索引的搜索缓存
    cache: Cache<String, SearchResultEntry>,
}

impl SearchCache {
    /// 创建新的搜索缓存，最大容量 500 条，24 小时过期
    pub fn new() -> Self {
        let cache = Cache::builder()
            .max_capacity(SEARCH_CACHE_MAX_CAPACITY)
            .time_to_live(Duration::from_secs(SEARCH_CACHE_TTL_SECS))
            .build();
        Self { cache }
    }

    /// 生成缓存键：`标题|歌手|搜索类型|页码`（全部小写）
    ///
    /// 分隔符 `|` 在标题/歌手中出现时会被转义为 `\|`，避免
    /// `"A|B" + "C"` 与 `"A" + "B|C"` 这类组合碰撞。
    fn make_key_with_options(
        title: &str,
        artist: Option<&str>,
        search_type: &str,
        page: u32,
    ) -> String {
        let title_lower = title.to_lowercase().trim().replace('|', "\\|");
        let artist_lower = artist
            .map(|a| a.to_lowercase().trim().replace('|', "\\|"))
            .unwrap_or_default();
        format!("{}|{}|{}|{}", title_lower, artist_lower, search_type, page)
    }

    /// 快速获取缓存（默认搜索类型 title，页码 1）
    pub async fn get(&self, title: &str, artist: Option<&str>) -> Option<SearchResultEntry> {
        self.get_with_options(title, artist, "title", 1).await
    }

    /// 带参数的缓存查询
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

    /// 快速插入缓存（默认搜索类型 title，页码 1）
    pub async fn insert(&self, title: &str, artist: Option<&str>, entry: SearchResultEntry) {
        self.insert_with_options(title, artist, "title", 1, entry)
            .await;
    }

    /// 带参数的缓存插入
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

    /// 清空所有搜索缓存
    pub async fn clear(&self) {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
    }

    /// 获取搜索缓存统计
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

/// 缓存管理器，统一管理歌词缓存和搜索缓存
#[derive(Debug, Clone)]
pub struct CacheManager {
    /// 歌词缓存
    lyrics_cache: LyricsCache,
    /// 搜索缓存
    search_cache: SearchCache,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new() -> Self {
        Self {
            lyrics_cache: LyricsCache::new(),
            search_cache: SearchCache::new(),
        }
    }

    /// 获取歌词缓存引用
    pub fn lyrics(&self) -> &LyricsCache {
        &self.lyrics_cache
    }

    /// 获取搜索缓存引用
    pub fn search(&self) -> &SearchCache {
        &self.search_cache
    }

    /// 清空所有缓存
    pub async fn clear_all(&self) {
        self.lyrics_cache.clear().await;
        self.search_cache.clear().await;
    }

    /// 获取两种缓存的统计信息
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
