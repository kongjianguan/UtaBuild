use crate::models::{LyricElement, SearchResponse};
use crate::output::LyricsOutput;
use crate::platform::{ensure_dir_exists, get_cache_dir, get_data_dir};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub data: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub title: Option<String>,
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub title: String,
    pub artist: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCache {
    pub timestamp: DateTime<Utc>,
    pub query: SearchQuery,
    pub results: Vec<SearchResultItem>,
}

impl SearchCache {
    pub fn new(query: SearchQuery, results: Vec<SearchResultItem>) -> Self {
        Self {
            timestamp: Utc::now(),
            query,
            results,
        }
    }

    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        let duration = now - self.timestamp;
        duration < Duration::hours(24)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponseCache {
    pub timestamp: DateTime<Utc>,
    pub query: SearchQuery,
    pub response: SearchResponse,
}

impl SearchResponseCache {
    pub fn new(query: SearchQuery, response: SearchResponse) -> Self {
        Self {
            timestamp: Utc::now(),
            query,
            response,
        }
    }

    pub fn is_valid(&self) -> bool {
        let now = Utc::now();
        let duration = now - self.timestamp;
        duration < Duration::hours(24)
    }
}

pub struct Cache {
    cache_dir: PathBuf,
}

impl Cache {
    pub fn new() -> anyhow::Result<Self> {
        let cache_dir = get_cache_dir().join("cache");
        ensure_dir_exists(&cache_dir)?;
        debug!("缓存目录: {:?}", cache_dir);

        Ok(Cache { cache_dir })
    }

    pub fn get(&self, key: &str) -> Option<CacheEntry> {
        let file_path = self.cache_dir.join(format!("{}.json", key));
        if file_path.exists() {
            match fs::read_to_string(&file_path) {
                Ok(content) => match serde_json::from_str::<CacheEntry>(&content) {
                    Ok(entry) => return Some(entry),
                    Err(e) => warn!("解析缓存失败: {}", e),
                },
                Err(e) => warn!("读取缓存失败: {}", e),
            }
        }
        None
    }

    pub fn set(&self, key: &str, data: serde_json::Value) -> anyhow::Result<()> {
        let entry = CacheEntry {
            key: key.to_string(),
            data,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let file_path = self.cache_dir.join(format!("{}.json", key));
        let content = serde_json::to_string_pretty(&entry)?;
        fs::write(&file_path, content)?;
        debug!("缓存已保存: {}", key);

        Ok(())
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.cache_dir)?;
            debug!("缓存已清除");
        }
        Ok(())
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new().expect("无法创建缓存目录")
    }
}

fn get_search_cache_path(cache_dir: Option<&PathBuf>) -> PathBuf {
    if let Some(dir) = cache_dir {
        dir.join("search_cache.json")
    } else {
        get_data_dir().join("search_cache.json")
    }
}

fn get_search_response_cache_dir(cache_dir: Option<&PathBuf>) -> PathBuf {
    if let Some(dir) = cache_dir {
        dir.join("search_responses")
    } else {
        get_cache_dir().join("search_responses")
    }
}

pub fn save_search_cache(
    query: SearchQuery,
    results: Vec<SearchResultItem>,
    cache_dir: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let cache = SearchCache::new(query, results);
    let path = get_search_cache_path(cache_dir);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(&cache)?;
    fs::write(&path, content)?;
    debug!("搜索缓存已保存: {:?}", path);

    Ok(())
}

pub fn load_search_cache(cache_dir: Option<&PathBuf>) -> Option<SearchCache> {
    let path = get_search_cache_path(cache_dir);

    if !path.exists() {
        debug!("搜索缓存文件不存在: {:?}", path);
        return None;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<SearchCache>(&content) {
            Ok(cache) => {
                if cache.is_valid() {
                    debug!("搜索缓存有效: {:?}", path);
                    Some(cache)
                } else {
                    debug!("搜索缓存已过期: {:?}", path);
                    None
                }
            }
            Err(e) => {
                warn!("解析搜索缓存失败: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("读取搜索缓存失败: {}", e);
            None
        }
    }
}

fn normalize_search_query(
    title: &str,
    artist: Option<&str>,
    search_type: &str,
    page: u32,
) -> SearchQuery {
    SearchQuery {
        title: {
            let trimmed = title.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        },
        artist: artist.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }),
        search_type: Some(search_type.trim().to_string()),
        page: Some(page.max(1)),
    }
}

fn search_query_to_cache_filename(query: &SearchQuery) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let encoded = serde_json::to_string(query).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    encoded.hash(&mut hasher);
    format!("{:x}.json", hasher.finish())
}

pub fn save_search_response_cache(
    title: &str,
    artist: Option<&str>,
    search_type: &str,
    page: u32,
    response: SearchResponse,
    cache_dir: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let query = normalize_search_query(title, artist, search_type, page);
    let cache_dir = get_search_response_cache_dir(cache_dir);
    fs::create_dir_all(&cache_dir)?;

    let path = cache_dir.join(search_query_to_cache_filename(&query));
    let cache = SearchResponseCache::new(query, response);
    let content = serde_json::to_string_pretty(&cache)?;
    fs::write(&path, content)?;
    debug!("搜索响应缓存已保存: {:?}", path);

    Ok(())
}

pub fn get_search_response_cache(
    title: &str,
    artist: Option<&str>,
    search_type: &str,
    page: u32,
    cache_dir: Option<&PathBuf>,
) -> Option<SearchResponse> {
    let query = normalize_search_query(title, artist, search_type, page);
    let cache_dir = get_search_response_cache_dir(cache_dir);
    let path = cache_dir.join(search_query_to_cache_filename(&query));

    if !path.exists() {
        debug!("搜索响应缓存文件不存在: {:?}", path);
        return None;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<SearchResponseCache>(&content) {
            Ok(cache) => {
                if !cache.is_valid() {
                    debug!("搜索响应缓存已过期: {:?}", path);
                    let _ = fs::remove_file(&path);
                    return None;
                }

                if cache.query.title != query.title
                    || cache.query.artist != query.artist
                    || cache.query.search_type != query.search_type
                    || cache.query.page != query.page
                {
                    warn!("搜索响应缓存键不匹配，忽略: {:?}", path);
                    return None;
                }

                debug!("搜索响应缓存有效: {:?}", path);
                Some(cache.response)
            }
            Err(e) => {
                warn!("解析搜索响应缓存失败: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("读取搜索响应缓存失败: {}", e);
            None
        }
    }
}

pub fn clear_search_response_cache(cache_dir: Option<&PathBuf>) -> anyhow::Result<()> {
    let cache_dir = get_search_response_cache_dir(cache_dir);
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)?;
        debug!("搜索响应缓存已清除");
    }
    Ok(())
}

pub fn get_cached_result(index: usize, cache_dir: Option<&PathBuf>) -> Option<SearchResultItem> {
    let cache = load_search_cache(cache_dir)?;

    if index >= cache.results.len() {
        debug!("索引超出范围: index={}, len={}", index, cache.results.len());
        return None;
    }

    Some(cache.results[index].clone())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsCache {
    pub timestamp: DateTime<Utc>,
    pub url: String,
    pub data: LyricsOutput,
}

impl LyricsCache {
    pub fn new(url: String, data: LyricsOutput) -> Self {
        Self {
            timestamp: Utc::now(),
            url,
            data,
        }
    }

    pub fn is_valid(&self) -> bool {
        true
    }
}

fn get_lyrics_cache_dir() -> PathBuf {
    get_cache_dir().join("lyrics")
}

fn get_lyrics_annotations_cache_dir(cache_dir: Option<&PathBuf>) -> PathBuf {
    if let Some(dir) = cache_dir {
        dir.join("lyrics_annotations")
    } else {
        get_cache_dir().join("lyrics_annotations")
    }
}

fn url_to_cache_filename(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:x}.json", hash)
}

pub fn save_lyrics_cache(url: &str, lyrics: LyricsOutput) -> anyhow::Result<()> {
    let cache_dir = get_lyrics_cache_dir();
    fs::create_dir_all(&cache_dir)?;

    let cache = LyricsCache::new(url.to_string(), lyrics);
    let filename = url_to_cache_filename(url);
    let path = cache_dir.join(&filename);

    let content = serde_json::to_string_pretty(&cache)?;
    fs::write(&path, content)?;
    debug!("歌词缓存已保存: {:?}", path);

    Ok(())
}

pub fn get_lyrics_cache(url: &str) -> Option<LyricsOutput> {
    let cache_dir = get_lyrics_cache_dir();
    let filename = url_to_cache_filename(url);
    let path = cache_dir.join(&filename);

    if !path.exists() {
        debug!("歌词缓存文件不存在: {:?}", path);
        return None;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<LyricsCache>(&content) {
            Ok(cache) => {
                if cache.is_valid() {
                    debug!("歌词缓存有效: {:?}", path);
                    Some(cache.data)
                } else {
                    debug!("歌词缓存已过期: {:?}", path);
                    if let Err(e) = fs::remove_file(&path) {
                        warn!("删除过期缓存失败: {}", e);
                    }
                    None
                }
            }
            Err(e) => {
                warn!("解析歌词缓存失败: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("读取歌词缓存失败: {}", e);
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsAnnotationsCache {
    pub timestamp: DateTime<Utc>,
    pub url: String,
    pub annotations: Vec<LyricElement>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
}

impl LyricsAnnotationsCache {
    pub fn new(url: String, annotations: Vec<LyricElement>) -> Self {
        Self::new_with_metadata(url, annotations, None, None, None, None)
    }

    pub fn new_with_metadata(
        url: String,
        annotations: Vec<LyricElement>,
        title: Option<String>,
        artist: Option<String>,
        album: Option<String>,
        cover_url: Option<String>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            url,
            annotations,
            title,
            artist,
            album,
            cover_url,
        }
    }

    pub fn is_valid(&self) -> bool {
        true
    }
}

pub fn save_lyrics_annotations_cache(
    url: &str,
    annotations: &[LyricElement],
    cache_dir: Option<&PathBuf>,
) -> anyhow::Result<()> {
    save_lyrics_annotations_cache_with_metadata(url, annotations, None, None, None, None, cache_dir)
}

pub fn save_lyrics_annotations_cache_with_metadata(
    url: &str,
    annotations: &[LyricElement],
    title: Option<&str>,
    artist: Option<&str>,
    album: Option<&str>,
    cover_url: Option<&str>,
    cache_dir: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let cache_dir = get_lyrics_annotations_cache_dir(cache_dir);
    fs::create_dir_all(&cache_dir)?;

    let filename = url_to_cache_filename(url);
    let path = cache_dir.join(&filename);
    let existing = fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str::<LyricsAnnotationsCache>(&content).ok())
        .filter(|cache| cache.url == url);

    let choose_metadata = |incoming: Option<&str>, existing: Option<String>| {
        incoming
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or(existing)
    };

    let cache = LyricsAnnotationsCache::new_with_metadata(
        url.to_string(),
        annotations.to_vec(),
        choose_metadata(
            title,
            existing.as_ref().and_then(|cache| cache.title.clone()),
        ),
        choose_metadata(
            artist,
            existing.as_ref().and_then(|cache| cache.artist.clone()),
        ),
        choose_metadata(
            album,
            existing.as_ref().and_then(|cache| cache.album.clone()),
        ),
        choose_metadata(
            cover_url,
            existing.as_ref().and_then(|cache| cache.cover_url.clone()),
        ),
    );
    let content = serde_json::to_string_pretty(&cache)?;
    fs::write(&path, content)?;
    debug!("歌词注释缓存已保存: {:?}", path);

    Ok(())
}

pub fn get_lyrics_annotations_cache(
    url: &str,
    cache_dir: Option<&PathBuf>,
) -> Option<Vec<LyricElement>> {
    get_lyrics_annotations_cache_entry(url, cache_dir).map(|cache| cache.annotations)
}

pub fn get_lyrics_annotations_cache_entry(
    url: &str,
    cache_dir: Option<&PathBuf>,
) -> Option<LyricsAnnotationsCache> {
    let cache_dir = get_lyrics_annotations_cache_dir(cache_dir);
    let filename = url_to_cache_filename(url);
    let path = cache_dir.join(&filename);

    if !path.exists() {
        debug!("歌词注释缓存文件不存在: {:?}", path);
        return None;
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<LyricsAnnotationsCache>(&content) {
            Ok(cache) => {
                if cache.url != url {
                    warn!("歌词注释缓存 URL 不匹配，忽略: {:?}", path);
                    return None;
                }

                debug!("歌词注释缓存有效: {:?}", path);
                Some(cache)
            }
            Err(e) => {
                warn!("解析歌词注释缓存失败: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("读取歌词注释缓存失败: {}", e);
            None
        }
    }
}

pub fn list_lyrics_annotations_cache(
    cache_dir: Option<&PathBuf>,
) -> anyhow::Result<Vec<LyricsAnnotationsCache>> {
    let cache_dir = get_lyrics_annotations_cache_dir(cache_dir);
    if !cache_dir.exists() {
        return Ok(Vec::new());
    }

    let mut caches = Vec::new();
    for entry in fs::read_dir(cache_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<LyricsAnnotationsCache>(&content) {
                Ok(cache) => caches.push(cache),
                Err(e) => warn!("解析歌词注释缓存失败: {}", e),
            },
            Err(e) => warn!("读取歌词注释缓存失败: {}", e),
        }
    }

    Ok(caches)
}

pub fn delete_lyrics_annotations_cache(
    url: &str,
    cache_dir: Option<&PathBuf>,
) -> anyhow::Result<bool> {
    let cache_dir = get_lyrics_annotations_cache_dir(cache_dir);
    let path = cache_dir.join(url_to_cache_filename(url));
    if path.exists() {
        fs::remove_file(&path)?;
        debug!("歌词注释缓存已删除: {:?}", path);
        return Ok(true);
    }
    Ok(false)
}

pub fn clear_lyrics_annotations_cache(cache_dir: Option<&PathBuf>) -> anyhow::Result<()> {
    let cache_dir = get_lyrics_annotations_cache_dir(cache_dir);
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)?;
        debug!("歌词注释缓存已清除");
    }
    Ok(())
}

pub fn clear_lyrics_cache() -> anyhow::Result<()> {
    let cache_dir = get_lyrics_cache_dir();
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)?;
        debug!("歌词缓存已清除");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_search_cache_is_valid() {
        let query = SearchQuery {
            title: Some("テスト曲".to_string()),
            artist: Some("テストアーティスト".to_string()),
            search_type: None,
            page: None,
        };
        let results = vec![SearchResultItem {
            title: "テスト曲".to_string(),
            artist: "テストアーティスト".to_string(),
            url: "https://example.com/test".to_string(),
        }];

        let cache = SearchCache::new(query, results);

        assert!(cache.is_valid());
        assert_eq!(cache.results.len(), 1);
    }

    #[test]
    fn test_search_cache_expired() {
        let query = SearchQuery {
            title: Some("テスト曲".to_string()),
            artist: None,
            search_type: None,
            page: None,
        };
        let results = vec![];

        let mut cache = SearchCache::new(query, results);
        cache.timestamp = Utc::now() - Duration::hours(25);

        assert!(!cache.is_valid());
    }

    #[test]
    fn test_lyrics_cache_is_valid() {
        let lyrics_output = LyricsOutput {
            status: "success".to_string(),
            title: Some("テスト曲".to_string()),
            artist: Some("テストアーティスト".to_string()),
            url: Some("https://example.com/test".to_string()),
            lyrics: None,
        };

        let cache = LyricsCache::new("https://example.com/test".to_string(), lyrics_output);

        assert!(cache.is_valid());
        assert_eq!(cache.url, "https://example.com/test");
    }

    #[test]
    fn test_lyrics_cache_is_permanent() {
        let lyrics_output = LyricsOutput {
            status: "success".to_string(),
            title: Some("テスト曲".to_string()),
            artist: None,
            url: None,
            lyrics: None,
        };

        let mut cache = LyricsCache::new("https://example.com/test".to_string(), lyrics_output);
        cache.timestamp = Utc::now() - Duration::hours(25);

        assert!(cache.is_valid());
    }

    #[test]
    fn test_url_to_cache_filename() {
        let url1 = "https://example.com/song1";
        let url2 = "https://example.com/song2";

        let filename1 = url_to_cache_filename(url1);
        let filename2 = url_to_cache_filename(url2);

        assert!(filename1.ends_with(".json"));
        assert!(filename2.ends_with(".json"));
        assert_ne!(filename1, filename2);

        let filename1_again = url_to_cache_filename(url1);
        assert_eq!(filename1, filename1_again);
    }

    #[test]
    fn test_save_and_load_search_cache() {
        let temp_dir = tempdir().unwrap();
        let cache_path = PathBuf::from(temp_dir.path());

        let query = SearchQuery {
            title: Some("テスト曲".to_string()),
            artist: Some("テストアーティスト".to_string()),
            search_type: None,
            page: None,
        };
        let results = vec![
            SearchResultItem {
                title: "テスト曲".to_string(),
                artist: "テストアーティスト".to_string(),
                url: "https://example.com/test".to_string(),
            },
            SearchResultItem {
                title: "テスト曲2".to_string(),
                artist: "テストアーティスト2".to_string(),
                url: "https://example.com/test2".to_string(),
            },
        ];

        save_search_cache(query.clone(), results.clone(), Some(&cache_path)).unwrap();

        let loaded = load_search_cache(Some(&cache_path)).unwrap();

        assert_eq!(loaded.query.title, query.title);
        assert_eq!(loaded.query.artist, query.artist);
        assert_eq!(loaded.results.len(), 2);
        assert_eq!(loaded.results[0].title, "テスト曲");
    }

    #[test]
    fn test_load_nonexistent_search_cache() {
        let temp_dir = tempdir().unwrap();
        let cache_path = PathBuf::from(temp_dir.path());

        let loaded = load_search_cache(Some(&cache_path));

        assert!(loaded.is_none());
    }

    #[test]
    fn test_get_cached_result() {
        let temp_dir = tempdir().unwrap();
        let cache_path = PathBuf::from(temp_dir.path());

        let query = SearchQuery {
            title: Some("テスト曲".to_string()),
            artist: None,
            search_type: None,
            page: None,
        };
        let results = vec![
            SearchResultItem {
                title: "曲1".to_string(),
                artist: "アーティスト1".to_string(),
                url: "https://example.com/1".to_string(),
            },
            SearchResultItem {
                title: "曲2".to_string(),
                artist: "アーティスト2".to_string(),
                url: "https://example.com/2".to_string(),
            },
        ];

        save_search_cache(query, results, Some(&cache_path)).unwrap();

        let result0 = get_cached_result(0, Some(&cache_path)).unwrap();
        assert_eq!(result0.title, "曲1");

        let result1 = get_cached_result(1, Some(&cache_path)).unwrap();
        assert_eq!(result1.title, "曲2");

        let result_out_of_range = get_cached_result(5, Some(&cache_path));
        assert!(result_out_of_range.is_none());
    }

    #[test]
    fn test_cache_entry_serialization() {
        let entry = CacheEntry {
            key: "test_key".to_string(),
            data: serde_json::json!({"title": "テスト曲"}),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("test_key"));
        assert!(json.contains("1234567890"));

        let deserialized: CacheEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.key, "test_key");
        assert_eq!(deserialized.timestamp, 1234567890);
    }

    #[test]
    fn test_search_query_serialization() {
        let query = SearchQuery {
            title: Some("曲名".to_string()),
            artist: Some("アーティスト".to_string()),
            search_type: None,
            page: None,
        };

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("曲名"));
        assert!(json.contains("アーティスト"));

        let deserialized: SearchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, Some("曲名".to_string()));
        assert_eq!(deserialized.artist, Some("アーティスト".to_string()));
    }

    #[test]
    fn test_search_result_item_serialization() {
        let item = SearchResultItem {
            title: "曲名".to_string(),
            artist: "アーティスト".to_string(),
            url: "https://example.com/test".to_string(),
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("曲名"));
        assert!(json.contains("アーティスト"));
        assert!(json.contains("https://example.com/test"));
    }

    #[test]
    fn test_save_and_load_search_response_cache() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = PathBuf::from(temp_dir.path());
        let response = SearchResponse {
            status: "select".to_string(),
            query_title: Some("R".to_string()),
            query_artist: Some("Roselia".to_string()),
            search_type: "title".to_string(),
            page: 2,
            pagination: Some(crate::models::SearchPagination {
                current_page: 2,
                total_pages: 10,
                has_next: true,
            }),
            results: vec![crate::models::SearchResult::with_artist_info(
                "R".to_string(),
                "Roselia".to_string(),
                "/lyric/yb18072521/".to_string(),
                None,
                None,
            )],
            error: None,
        };

        save_search_response_cache(
            "R",
            Some("Roselia"),
            "title",
            2,
            response.clone(),
            Some(&cache_dir),
        )
        .unwrap();

        let restored =
            get_search_response_cache("R", Some("Roselia"), "title", 2, Some(&cache_dir)).unwrap();

        assert_eq!(restored.status, response.status);
        assert_eq!(restored.page, response.page);
        assert_eq!(restored.pagination, response.pagination);
        assert_eq!(restored.results.len(), response.results.len());
        assert_eq!(restored.results[0].url, response.results[0].url);
    }

    #[test]
    fn test_search_response_cache_miss_on_query_mismatch() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = PathBuf::from(temp_dir.path());
        let response = SearchResponse::new();

        save_search_response_cache("R", Some("Roselia"), "title", 1, response, Some(&cache_dir))
            .unwrap();

        assert!(
            get_search_response_cache("R", Some("Roselia"), "title", 2, Some(&cache_dir)).is_none()
        );
        assert!(
            get_search_response_cache("R", Some("Other"), "title", 1, Some(&cache_dir)).is_none()
        );
    }

    #[test]
    fn test_save_and_load_lyrics_annotations_cache() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = PathBuf::from(temp_dir.path());
        let annotations = vec![
            LyricElement::new_text("hello".to_string()),
            LyricElement::new_ruby("世".to_string(), "せ".to_string()),
        ];

        save_lyrics_annotations_cache("/lyric/yb18072521/", &annotations, Some(&cache_dir))
            .unwrap();

        let restored =
            get_lyrics_annotations_cache("/lyric/yb18072521/", Some(&cache_dir)).unwrap();
        assert_eq!(restored.len(), annotations.len());
        assert_eq!(restored[0].base, annotations[0].base);
        assert_eq!(restored[1].ruby, annotations[1].ruby);
    }

    #[test]
    fn test_save_lyrics_annotations_cache_preserves_existing_cover_when_metadata_missing() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = PathBuf::from(temp_dir.path());
        let url = "/lyric/cover-preserve/";
        let annotations = vec![LyricElement::new_text("hello".to_string())];

        save_lyrics_annotations_cache_with_metadata(
            url,
            &annotations,
            Some("Title"),
            Some("Artist"),
            Some("Album"),
            Some("https://example.test/cover.jpg"),
            Some(&cache_dir),
        )
        .unwrap();
        save_lyrics_annotations_cache_with_metadata(
            url,
            &annotations,
            Some("Title"),
            Some("Artist"),
            None,
            None,
            Some(&cache_dir),
        )
        .unwrap();

        let restored = get_lyrics_annotations_cache_entry(url, Some(&cache_dir)).unwrap();
        assert_eq!(restored.album.as_deref(), Some("Album"));
        assert_eq!(
            restored.cover_url.as_deref(),
            Some("https://example.test/cover.jpg")
        );
    }

    #[test]
    fn test_delete_lyrics_annotations_cache() {
        let temp_dir = tempdir().unwrap();
        let cache_dir = PathBuf::from(temp_dir.path());
        let url = "/lyric/delete-test/";
        let annotations = vec![LyricElement::new_text("削除テスト".to_string())];

        save_lyrics_annotations_cache(url, &annotations, Some(&cache_dir)).unwrap();
        assert!(get_lyrics_annotations_cache(url, Some(&cache_dir)).is_some());

        assert!(delete_lyrics_annotations_cache(url, Some(&cache_dir)).unwrap());
        assert!(get_lyrics_annotations_cache(url, Some(&cache_dir)).is_none());
        assert!(!delete_lyrics_annotations_cache(url, Some(&cache_dir)).unwrap());
    }
}
