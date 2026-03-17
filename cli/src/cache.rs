use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use chrono::{DateTime, Utc, Duration};
use crate::output::LyricsOutput;
use crate::platform::{get_cache_dir, get_data_dir, ensure_dir_exists};

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
                Ok(content) => {
                    match serde_json::from_str::<CacheEntry>(&content) {
                        Ok(entry) => return Some(entry),
                        Err(e) => warn!("解析缓存失败: {}", e),
                    }
                }
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

pub fn save_search_cache(query: SearchQuery, results: Vec<SearchResultItem>, cache_dir: Option<&PathBuf>) -> anyhow::Result<()> {
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
        Ok(content) => {
            match serde_json::from_str::<SearchCache>(&content) {
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
            }
        }
        Err(e) => {
            warn!("读取搜索缓存失败: {}", e);
            None
        }
    }
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
        let now = Utc::now();
        let duration = now - self.timestamp;
        duration < Duration::hours(24)
    }
}

fn get_lyrics_cache_dir() -> PathBuf {
    get_cache_dir().join("lyrics")
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
        Ok(content) => {
            match serde_json::from_str::<LyricsCache>(&content) {
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
            }
        }
        Err(e) => {
            warn!("读取歌词缓存失败: {}", e);
            None
        }
    }
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
    fn test_lyrics_cache_expired() {
        let lyrics_output = LyricsOutput {
            status: "success".to_string(),
            title: Some("テスト曲".to_string()),
            artist: None,
            url: None,
            lyrics: None,
        };
        
        let mut cache = LyricsCache::new("https://example.com/test".to_string(), lyrics_output);
        cache.timestamp = Utc::now() - Duration::hours(25);
        
        assert!(!cache.is_valid());
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
}
