use crate::cache_manager::CacheManager;
use crate::output::{ErrorOutput, HistoryItem, HistoryOutput, LyricsOutput};
use crate::platform::{ensure_dir_exists, get_data_dir};
use crate::searcher::UtaTenSearcher;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

const MAX_HISTORY_SIZE: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyricist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct History {
    pub items: Vec<HistoryRecord>,
}

impl History {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
}

fn get_history_file_path(cache_dir: Option<&PathBuf>) -> PathBuf {
    if let Some(dir) = cache_dir {
        dir.join("history.json")
    } else {
        get_data_dir().join("history.json")
    }
}

fn ensure_history_dir(cache_dir: Option<&PathBuf>) -> anyhow::Result<()> {
    let path = get_history_file_path(cache_dir);
    if let Some(parent) = path.parent() {
        ensure_dir_exists(&parent.to_path_buf())?;
    }
    Ok(())
}

fn load_history(cache_dir: Option<&PathBuf>) -> History {
    let path = get_history_file_path(cache_dir);

    if !path.exists() {
        return History::new();
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<History>(&content) {
            Ok(history) => history,
            Err(_) => History::new(),
        },
        Err(_) => History::new(),
    }
}

fn save_history(history: &History, cache_dir: Option<&PathBuf>) -> anyhow::Result<()> {
    ensure_history_dir(cache_dir)?;
    let path = get_history_file_path(cache_dir);
    let json = serde_json::to_string_pretty(history)?;
    fs::write(&path, json)?;
    Ok(())
}

fn get_current_timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn list(cache_dir: Option<&PathBuf>) -> anyhow::Result<()> {
    info!("列出历史记录");

    let history = load_history(cache_dir);

    let items: Vec<HistoryItem> = history
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| HistoryItem {
            index,
            title: item.title.clone(),
            artist: if item.artist.is_empty() {
                None
            } else {
                Some(item.artist.clone())
            },
            url: if item.url.is_empty() {
                None
            } else {
                Some(item.url.clone())
            },
            timestamp: item.timestamp.clone(),
            lyricist: item.lyricist.clone(),
            composer: item.composer.clone(),
        })
        .collect();

    let output = if items.is_empty() {
        HistoryOutput::empty()
    } else {
        HistoryOutput::new(items)
    };

    println!("{}", output.to_json()?);
    Ok(())
}

pub async fn use_record(index: u32, cache_dir: Option<&PathBuf>) -> anyhow::Result<()> {
    info!("使用历史记录: {}", index);

    let history = load_history(cache_dir);

    let idx = index as usize;

    if idx >= history.items.len() {
        let output = ErrorOutput::error(&format!(
            "历史记录索引 {} 超出范围，当前共有 {} 条记录",
            index,
            history.items.len()
        ));
        println!("{}", output.to_json()?);
        return Ok(());
    }

    let item = &history.items[idx];

    let cache = CacheManager::new();
    let searcher = Arc::new(UtaTenSearcher::new(cache));

    let process_result = searcher.process_song(&item.title, Some(&item.artist)).await;

    if process_result.search_results.is_empty() {
        let output = ErrorOutput::no_results("未找到匹配的歌词");
        println!("{}", output.to_json()?);
        return Ok(());
    }

    let target_index = process_result
        .search_results
        .iter()
        .position(|r| r.url == item.url)
        .unwrap_or(0);

    let lyricist = process_result
        .search_results
        .get(target_index)
        .and_then(|r| r.lyricist.clone());
    let composer = process_result
        .search_results
        .get(target_index)
        .and_then(|r| r.composer.clone());

    let selected_result = searcher
        .select_result(process_result.clone(), target_index)
        .await;

    if selected_result.status == "success" {
        add_to_history(
            &selected_result.found_title,
            &selected_result.found_artist,
            &selected_result.lyrics_url,
            lyricist,
            composer,
            cache_dir,
        )?;

        let lyrics_output = LyricsOutput::success(
            selected_result.found_title.clone(),
            selected_result.found_artist.clone(),
            selected_result.lyrics_url,
            &selected_result.ruby_annotations,
        );
        println!("{}", lyrics_output.to_json()?);
    } else {
        let output = ErrorOutput::error(selected_result.error.as_deref().unwrap_or("获取歌词失败"));
        println!("{}", output.to_json()?);
    }

    Ok(())
}

pub fn clear(cache_dir: Option<&PathBuf>) -> anyhow::Result<()> {
    info!("清除历史记录");

    let history = History::new();
    save_history(&history, cache_dir)?;

    let output = HistoryOutput::empty();

    println!("{}", output.to_json()?);
    Ok(())
}

pub fn add_to_history(
    title: &str,
    artist: &str,
    url: &str,
    lyricist: Option<String>,
    composer: Option<String>,
    cache_dir: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let mut history = load_history(cache_dir);

    history
        .items
        .retain(|item| !(item.title == title && item.artist == artist));

    let new_item = HistoryRecord {
        title: title.to_string(),
        artist: artist.to_string(),
        url: url.to_string(),
        timestamp: get_current_timestamp(),
        lyricist,
        composer,
    };

    history.items.insert(0, new_item);

    if history.items.len() > MAX_HISTORY_SIZE {
        history.items.truncate(MAX_HISTORY_SIZE);
    }

    save_history(&history, cache_dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_history_new() {
        let history = History::new();
        assert!(history.items.is_empty());
    }

    #[test]
    fn test_history_default() {
        let history = History::default();
        assert!(history.items.is_empty());
    }

    #[test]
    fn test_history_record_serialization() {
        let record = HistoryRecord {
            title: "テスト曲".to_string(),
            artist: "テストアーティスト".to_string(),
            url: "https://example.com/test".to_string(),
            timestamp: "2024-01-01T12:00:00".to_string(),
            lyricist: Some("作詞者".to_string()),
            composer: Some("作曲者".to_string()),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("テスト曲"));
        assert!(json.contains("テストアーティスト"));
        assert!(json.contains("https://example.com/test"));
        assert!(json.contains("2024-01-01T12:00:00"));
        assert!(json.contains("作詞者"));
        assert!(json.contains("作曲者"));

        let deserialized: HistoryRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "テスト曲");
        assert_eq!(deserialized.artist, "テストアーティスト");
    }

    #[test]
    fn test_history_record_without_optional_fields() {
        let record = HistoryRecord {
            title: "曲名".to_string(),
            artist: "アーティスト".to_string(),
            url: "https://example.com/test".to_string(),
            timestamp: "2024-01-01T12:00:00".to_string(),
            lyricist: None,
            composer: None,
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("曲名"));
        assert!(!json.contains("lyricist"));
        assert!(!json.contains("composer"));
    }

    #[test]
    fn test_history_serialization() {
        let mut history = History::new();
        history.items.push(HistoryRecord {
            title: "曲1".to_string(),
            artist: "アーティスト1".to_string(),
            url: "https://example.com/1".to_string(),
            timestamp: "2024-01-01T12:00:00".to_string(),
            lyricist: None,
            composer: None,
        });
        history.items.push(HistoryRecord {
            title: "曲2".to_string(),
            artist: "アーティスト2".to_string(),
            url: "https://example.com/2".to_string(),
            timestamp: "2024-01-02T12:00:00".to_string(),
            lyricist: None,
            composer: None,
        });

        let json = serde_json::to_string(&history).unwrap();
        assert!(json.contains("\"items\""));
        assert!(json.contains("\"曲1\""));
        assert!(json.contains("\"曲2\""));

        let deserialized: History = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.items.len(), 2);
    }

    #[test]
    fn test_get_history_file_path_with_cache_dir() {
        let cache_dir = PathBuf::from("/tmp/test_cache");
        let path = get_history_file_path(Some(&cache_dir));

        assert_eq!(path, PathBuf::from("/tmp/test_cache/history.json"));
    }

    #[test]
    fn test_get_history_file_path_without_cache_dir() {
        let path = get_history_file_path(None);

        assert!(path.to_string_lossy().contains("utabuild"));
        assert!(path.to_string_lossy().ends_with("history.json"));
    }

    #[test]
    fn test_load_history_nonexistent_file() {
        let temp_dir = tempdir().unwrap();
        let cache_path = PathBuf::from(temp_dir.path());

        let history = load_history(Some(&cache_path));

        assert!(history.items.is_empty());
    }

    #[test]
    fn test_save_and_load_history() {
        let temp_dir = tempdir().unwrap();
        let cache_path = PathBuf::from(temp_dir.path());

        let mut history = History::new();
        history.items.push(HistoryRecord {
            title: "テスト曲".to_string(),
            artist: "テストアーティスト".to_string(),
            url: "https://example.com/test".to_string(),
            timestamp: "2024-01-01T12:00:00".to_string(),
            lyricist: Some("作詞者".to_string()),
            composer: None,
        });

        save_history(&history, Some(&cache_path)).unwrap();

        let loaded = load_history(Some(&cache_path));

        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0].title, "テスト曲");
        assert_eq!(loaded.items[0].artist, "テストアーティスト");
        assert_eq!(loaded.items[0].lyricist, Some("作詞者".to_string()));
    }

    #[test]
    fn test_add_to_history() {
        let temp_dir = tempdir().unwrap();
        let cache_path = PathBuf::from(temp_dir.path());

        add_to_history(
            "曲1",
            "アーティスト1",
            "https://example.com/1",
            None,
            None,
            Some(&cache_path),
        )
        .unwrap();

        let history = load_history(Some(&cache_path));
        assert_eq!(history.items.len(), 1);
        assert_eq!(history.items[0].title, "曲1");

        add_to_history(
            "曲2",
            "アーティスト2",
            "https://example.com/2",
            Some("作詞者".to_string()),
            Some("作曲者".to_string()),
            Some(&cache_path),
        )
        .unwrap();

        let history = load_history(Some(&cache_path));
        assert_eq!(history.items.len(), 2);
        assert_eq!(history.items[0].title, "曲2");
        assert_eq!(history.items[1].title, "曲1");
    }

    #[test]
    fn test_add_to_history_updates_existing() {
        let temp_dir = tempdir().unwrap();
        let cache_path = PathBuf::from(temp_dir.path());

        add_to_history(
            "曲1",
            "アーティスト1",
            "https://example.com/1",
            None,
            None,
            Some(&cache_path),
        )
        .unwrap();

        let history = load_history(Some(&cache_path));
        assert_eq!(history.items.len(), 1);

        add_to_history(
            "曲1",
            "アーティスト1",
            "https://example.com/1_updated",
            Some("新しい作詞者".to_string()),
            None,
            Some(&cache_path),
        )
        .unwrap();

        let history = load_history(Some(&cache_path));
        assert_eq!(history.items.len(), 1);
        assert_eq!(history.items[0].url, "https://example.com/1_updated");
        assert_eq!(history.items[0].lyricist, Some("新しい作詞者".to_string()));
    }

    #[test]
    fn test_add_to_history_max_size() {
        let temp_dir = tempdir().unwrap();
        let cache_path = PathBuf::from(temp_dir.path());

        for i in 0..(MAX_HISTORY_SIZE + 10) {
            add_to_history(
                &format!("曲{}", i),
                &format!("アーティスト{}", i),
                &format!("https://example.com/{}", i),
                None,
                None,
                Some(&cache_path),
            )
            .unwrap();
        }

        let history = load_history(Some(&cache_path));
        assert_eq!(history.items.len(), MAX_HISTORY_SIZE);

        assert_eq!(
            history.items[0].title,
            format!("曲{}", MAX_HISTORY_SIZE + 9)
        );
    }

    #[test]
    fn test_get_current_timestamp() {
        let timestamp = get_current_timestamp();

        assert!(timestamp.contains("T"));
        assert!(timestamp.len() > 10);
    }
}
