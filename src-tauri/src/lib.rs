use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;
use utabuild_cli::cache::{
    clear_lyrics_annotations_cache, clear_search_response_cache, delete_lyrics_annotations_cache,
    get_lyrics_annotations_cache_entry, get_search_response_cache, list_lyrics_annotations_cache,
    save_lyrics_annotations_cache_with_metadata, save_search_response_cache,
};
use utabuild_cli::LyricElement;
use utabuild_cli::{CacheManager, UtaTenSearcher};

/// 应用状态
struct AppState {
    searcher: Mutex<UtaTenSearcher>,
    lsp_logging_enabled: Mutex<bool>,
}

/// 初始化搜索器
fn create_searcher() -> UtaTenSearcher {
    let cache_manager = CacheManager::new();
    UtaTenSearcher::new(cache_manager)
}

// ==================== Tauri Commands ====================

/// 搜索歌词
#[tauri::command]
async fn search_lyrics(
    app: AppHandle,
    title: String,
    artist: Option<String>,
    page: Option<u32>,
    use_cache: Option<bool>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let page = page.unwrap_or(1);
    let use_cache = use_cache.unwrap_or(true);
    write_app_lsp_log_if_enabled(
        &app,
        &state,
        "search",
        &format!(
            "search_lyrics title=\"{}\" artist=\"{}\" page={} use_cache={}",
            title,
            artist.as_deref().unwrap_or(""),
            page,
            use_cache
        ),
    )
    .await;

    if use_cache {
        if let Some(cached_response) =
            get_search_response_cache(&title, artist.as_deref(), "title", page, None)
        {
            write_app_lsp_log_if_enabled(&app, &state, "search", "search_lyrics cache hit").await;
            return serde_json::to_value(cached_response).map_err(|e| e.to_string());
        }
    }

    let searcher = state.searcher.lock().await;
    let result = if use_cache {
        searcher
            .search_with_options(&title, artist.as_deref(), "title", page)
            .await
    } else {
        searcher
            .search_with_options_uncached(&title, artist.as_deref(), "title", page)
            .await
    };
    drop(searcher);

    if result.error.is_none() {
        save_search_response_cache(
            &title,
            artist.as_deref(),
            "title",
            page,
            result.clone(),
            None,
        )
        .map_err(|e| e.to_string())?;
        write_app_lsp_log_if_enabled(
            &app,
            &state,
            "search",
            &format!(
                "search_lyrics success status={} results={}",
                result.status,
                result.results.len()
            ),
        )
        .await;
    } else {
        write_app_lsp_log_if_enabled(
            &app,
            &state,
            "search",
            &format!(
                "search_lyrics error={}",
                result.error.as_deref().unwrap_or("unknown")
            ),
        )
        .await;
    }

    serde_json::to_value(result).map_err(|e| e.to_string())
}

/// 选择搜索结果，获取歌词
#[tauri::command]
async fn get_lyrics(
    app: AppHandle,
    url: String,
    title: String,
    artist: Option<String>,
    use_cache: Option<bool>,
    save_salt_bridge: Option<bool>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let use_cache = use_cache.unwrap_or(true);
    let save_salt_bridge = save_salt_bridge.unwrap_or(true);
    write_app_lsp_log_if_enabled(
        &app,
        &state,
        "lyrics",
        &format!(
            "get_lyrics title=\"{}\" artist=\"{}\" url=\"{}\" use_cache={} save_salt_bridge={}",
            title,
            artist.as_deref().unwrap_or(""),
            url,
            use_cache,
            save_salt_bridge
        ),
    )
    .await;

    let searcher = state.searcher.lock().await;
    if use_cache {
        if let Some(cached_annotations) = searcher.cache().lyrics().get(&url).await {
            let existing_entry = get_lyrics_annotations_cache_entry(&url, None);
            let album = existing_entry
                .as_ref()
                .and_then(|entry| entry.album.clone());
            let cover_url = existing_entry
                .as_ref()
                .and_then(|entry| entry.cover_url.clone());
            save_lyrics_annotations_cache_with_metadata(
                &url,
                &cached_annotations,
                Some(&title),
                artist.as_deref(),
                album.as_deref(),
                cover_url.as_deref(),
                None,
            )
            .map_err(|e| e.to_string())?;
            let response =
                lyrics_success_response(title, artist, url, &cached_annotations, album, cover_url);
            if save_salt_bridge {
                save_salt_bridge_cache(&app, &response)?;
            }
            drop(searcher);
            write_app_lsp_log_if_enabled(&app, &state, "lyrics", "get_lyrics memory cache hit")
                .await;
            return Ok(response);
        }

        if let Some(cached_entry) = get_lyrics_annotations_cache_entry(&url, None) {
            let cached_annotations = cached_entry.annotations;
            let response_title = if title.trim().is_empty() {
                cached_entry
                    .title
                    .clone()
                    .unwrap_or_else(|| "未命名歌曲".to_string())
            } else {
                title.clone()
            };
            let response_artist = artist.clone().or(cached_entry.artist.clone());
            let album = cached_entry.album.clone();
            let cover_url = cached_entry.cover_url.clone();
            searcher
                .cache()
                .lyrics()
                .insert(url.clone(), cached_annotations.clone())
                .await;
            save_lyrics_annotations_cache_with_metadata(
                &url,
                &cached_annotations,
                Some(&response_title),
                response_artist.as_deref(),
                album.as_deref(),
                cover_url.as_deref(),
                None,
            )
            .map_err(|e| e.to_string())?;
            let response = lyrics_success_response(
                response_title,
                response_artist,
                url,
                &cached_annotations,
                album,
                cover_url,
            );
            if save_salt_bridge {
                save_salt_bridge_cache(&app, &response)?;
            }
            drop(searcher);
            write_app_lsp_log_if_enabled(&app, &state, "lyrics", "get_lyrics disk cache hit").await;
            return Ok(response);
        }
    }

    // 按CLI逻辑：直接用URL获取歌词，返回前端期望的格式
    match searcher.get_lyrics_with_ruby(&url).await {
        Some(html_content) => {
            let metadata = UtaTenSearcher::extract_song_page_metadata(&html_content);
            // 解析歌词和ruby
            let elements = searcher.extract_ruby_lyrics(&html_content);
            searcher
                .cache()
                .lyrics()
                .insert(url.clone(), elements.clone())
                .await;
            drop(searcher);
            save_lyrics_annotations_cache_with_metadata(
                &url,
                &elements,
                Some(&title),
                artist.as_deref(),
                metadata.album.as_deref(),
                metadata.cover_url.as_deref(),
                None,
            )
            .map_err(|e| e.to_string())?;
            let response = lyrics_success_response(
                title,
                artist,
                url,
                &elements,
                metadata.album,
                metadata.cover_url,
            );
            if save_salt_bridge {
                save_salt_bridge_cache(&app, &response)?;
            }
            write_app_lsp_log_if_enabled(
                &app,
                &state,
                "lyrics",
                &format!("get_lyrics success annotations={}", elements.len()),
            )
            .await;
            Ok(response)
        }
        None => {
            drop(searcher);
            write_app_lsp_log_if_enabled(&app, &state, "lyrics", "get_lyrics failed").await;
            serde_json::to_value(serde_json::json!({
                "status": "error",
                "error": "歌詞の取得に失敗しました"
            }))
            .map_err(|e| e.to_string())
        }
    }
}

fn save_salt_bridge_cache(app: &AppHandle, response: &serde_json::Value) -> Result<(), String> {
    let title = response
        .get("found_title")
        .and_then(|value| value.as_str())
        .unwrap_or("untitled");
    save_salt_bridge_cache_for_title(app, title, response)
}

fn save_saved_lyrics_from_response(response: &serde_json::Value) -> Result<(), String> {
    if response.get("status").and_then(|value| value.as_str()) != Some("success") {
        return Ok(());
    }

    let Some(url) = response.get("lyrics_url").and_then(|value| value.as_str()) else {
        return Ok(());
    };

    let annotations = response
        .get("ruby_annotations")
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<LyricElement>>(value).ok())
        .unwrap_or_default();
    if annotations.is_empty() {
        return Ok(());
    }

    save_lyrics_annotations_cache_with_metadata(
        url,
        &annotations,
        response.get("found_title").and_then(|value| value.as_str()),
        response
            .get("found_artist")
            .and_then(|value| value.as_str()),
        response.get("found_album").and_then(|value| value.as_str()),
        response.get("cover_url").and_then(|value| value.as_str()),
        None,
    )
    .map_err(|e| e.to_string())
}

fn save_salt_bridge_cache_for_title(
    app: &AppHandle,
    title: &str,
    response: &serde_json::Value,
) -> Result<(), String> {
    if response.get("status").and_then(|value| value.as_str()) != Some("success") {
        return Ok(());
    }
    if !response
        .get("ruby_annotations")
        .and_then(|value| value.as_array())
        .is_some_and(|annotations| {
            annotations.iter().any(|annotation| {
                annotation.get("type").and_then(|value| value.as_str()) == Some("ruby")
            })
        })
    {
        return Ok(());
    }

    let path = salt_bridge_cache_path(app, title)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string(response).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn salt_bridge_cache_path(app: &AppHandle, title: &str) -> Result<PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(data_dir
        .join("utabuild")
        .join("ruby")
        .join(format!("{}.json", safe_bridge_file_name(title))))
}

fn salt_pending_request_paths(app: &AppHandle) -> Result<Vec<PathBuf>, String> {
    let mut candidates = Vec::new();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    candidates.push(data_dir.join("utabuild").join("salt_pending_request.json"));
    if let Some(parent) = data_dir.parent() {
        candidates.push(parent.join("utabuild").join("salt_pending_request.json"));
    }
    if let Ok(cache_dir) = app.path().app_cache_dir() {
        candidates.push(cache_dir.join("utabuild").join("salt_pending_request.json"));
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn safe_bridge_file_name(title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return "untitled".to_string();
    }
    trimmed
        .chars()
        .map(|ch| match ch {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect()
}

#[tauri::command]
async fn take_salt_launch_request(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    let mut found_path = None;
    for path in salt_pending_request_paths(&app)? {
        if path.is_file() {
            found_path = Some(path);
            break;
        }
    }
    let Some(path) = found_path else {
        return Ok(None);
    };

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    for candidate in salt_pending_request_paths(&app)? {
        let _ = fs::remove_file(candidate);
    }
    let value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    write_app_lsp_log_if_enabled(
        &app,
        &state,
        "salt",
        &format!("take_salt_launch_request {}", compact_json(&value)),
    )
    .await;
    Ok(Some(value))
}

#[tauri::command]
async fn bind_salt_song_lyrics(
    app: AppHandle,
    state: State<'_, AppState>,
    salt_title: String,
    salt_artist: Option<String>,
    lyrics: serde_json::Value,
) -> Result<(), String> {
    let mut bound = lyrics;
    if let Some(object) = bound.as_object_mut() {
        object.insert(
            "salt_title".to_string(),
            serde_json::Value::String(salt_title.clone()),
        );
        object.insert(
            "salt_artist".to_string(),
            salt_artist
                .clone()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
        );
        object.insert(
            "salt_bound_at_ms".to_string(),
            serde_json::Value::Number(serde_json::Number::from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_millis() as u64,
            )),
        );
    }
    save_salt_bridge_cache_for_title(&app, &salt_title, &bound)?;
    write_app_lsp_log_if_enabled(
        &app,
        &state,
        "salt",
        &format!(
            "bind_salt_song_lyrics salt_title=\"{}\" salt_artist=\"{}\"",
            salt_title,
            salt_artist.as_deref().unwrap_or("")
        ),
    )
    .await;
    Ok(())
}

fn lyrics_success_response(
    title: String,
    artist: Option<String>,
    url: String,
    elements: &[LyricElement],
    album: Option<String>,
    cover_url: Option<String>,
) -> serde_json::Value {
    let ruby_annotations: Vec<serde_json::Value> = elements
        .iter()
        .map(|e| serde_json::to_value(e).unwrap_or_default())
        .collect();

    serde_json::json!({
        "status": "success",
        "found_title": title,
        "found_artist": artist,
        "found_album": album,
        "cover_url": cover_url,
        "lyrics_url": url,
        "ruby_annotations": ruby_annotations
    })
}

/// 开关应用自带的轻量lsp日志系统。
#[tauri::command]
async fn set_lsp_logging_enabled(
    app: AppHandle,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut logging_enabled = state.lsp_logging_enabled.lock().await;
        *logging_enabled = enabled;
    }

    write_app_lsp_log(
        &app,
        "settings",
        if enabled {
            "lsp logging enabled"
        } else {
            "lsp logging disabled"
        },
    )
}

/// 写入一条应用自带lsp日志。关闭日志时静默忽略。
#[tauri::command]
async fn append_lsp_log(
    app: AppHandle,
    scope: String,
    message: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    write_app_lsp_log_if_enabled(&app, &state, &scope, &message).await;
    Ok(())
}

/// 读取应用自带及可见LSPosed/lsp相关日志，供设置页按需查看。
#[tauri::command]
async fn get_lsp_logs(app: AppHandle) -> Result<String, String> {
    let mut candidates = Vec::new();

    if let Ok(data_dir) = app.path().app_data_dir() {
        candidates.push(data_dir.join("utabuild").join("lsp.log"));
        candidates.push(data_dir.join("utabuild").join("lsposed.log"));
        candidates.push(data_dir.join("utabuild").join("lsposed-module.log"));
        if let Some(parent) = data_dir.parent() {
            candidates.push(parent.join("utabuild").join("lsp.log"));
            candidates.push(parent.join("utabuild").join("lsposed.log"));
            candidates.push(parent.join("utabuild").join("lsposed-module.log"));
        }
    }

    if let Ok(cache_dir) = app.path().app_cache_dir() {
        candidates.push(cache_dir.join("utabuild").join("lsp.log"));
        candidates.push(cache_dir.join("utabuild").join("lsposed.log"));
        candidates.push(cache_dir.join("utabuild").join("lsposed-module.log"));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("2026.log"));

        let logs_dir = current_dir.join("logs");
        if let Ok(entries) = fs::read_dir(logs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if file_name.contains("lsp")
                    || file_name.contains("lsposed")
                    || file_name.contains("module")
                {
                    candidates.push(path);
                }
            }
        }
    }

    candidates.sort();
    candidates.dedup();

    let mut sections = Vec::new();
    for path in candidates {
        if !path.is_file() {
            continue;
        }

        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if content.trim().is_empty() {
            continue;
        }

        sections.push(format!(
            "===== {} =====\n{}",
            path.display(),
            tail_chars(&content, 64 * 1024)
        ));
    }

    if sections.is_empty() {
        Ok("暂无lsp日志".to_string())
    } else {
        Ok(sections.join("\n\n"))
    }
}

async fn write_app_lsp_log_if_enabled(
    app: &AppHandle,
    state: &State<'_, AppState>,
    scope: &str,
    message: &str,
) {
    if *state.lsp_logging_enabled.lock().await {
        let _ = write_app_lsp_log(app, scope, message);
    }
}

fn write_app_lsp_log(app: &AppHandle, scope: &str, message: &str) -> Result<(), String> {
    let path = app_lsp_log_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(format_app_log_line(scope, message).as_bytes())
        .map_err(|e| e.to_string())
}

fn app_lsp_log_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(data_dir.join("utabuild").join("lsp.log"))
}

fn format_app_log_line(scope: &str, message: &str) -> String {
    format!(
        "[{}] {}: {}\n",
        unix_timestamp_ms(),
        sanitize_log_token(scope),
        sanitize_log_message(message)
    )
}

fn unix_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn sanitize_log_token(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_control() || ch.is_whitespace() {
                '_'
            } else {
                ch
            }
        })
        .collect();

    if sanitized.is_empty() {
        "app".to_string()
    } else {
        sanitized.chars().take(32).collect()
    }
}

fn sanitize_log_message(message: &str) -> String {
    let sanitized = message.replace('\r', "\\r").replace('\n', "\\n");
    sanitized.chars().take(4_000).collect()
}

fn compact_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "<invalid-json>".to_string())
}

fn tail_chars(content: &str, max_chars: usize) -> String {
    let char_count = content.chars().count();
    if char_count <= max_chars {
        return content.to_string();
    }

    let start_byte = content
        .char_indices()
        .nth(char_count - max_chars)
        .map(|(index, _)| index)
        .unwrap_or(0);
    format!(
        "...（仅显示最后{}个字符）\n{}",
        max_chars,
        &content[start_byte..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lyrics_success_response_keeps_cached_and_live_shape_identical() {
        let elements = vec![
            LyricElement::new_text("hello".to_string()),
            LyricElement::new_ruby("世".to_string(), "せ".to_string()),
        ];

        let value = lyrics_success_response(
            "R".to_string(),
            Some("Roselia".to_string()),
            "/lyric/yb18072521/".to_string(),
            &elements,
            Some("Wahl".to_string()),
            Some("https://example.test/cover.jpg".to_string()),
        );

        assert_eq!(value["status"], "success");
        assert_eq!(value["found_title"], "R");
        assert_eq!(value["found_artist"], "Roselia");
        assert_eq!(value["lyrics_url"], "/lyric/yb18072521/");
        assert_eq!(value["found_album"], "Wahl");
        assert_eq!(value["cover_url"], "https://example.test/cover.jpg");
        assert_eq!(value["ruby_annotations"][0]["type"], "text");
        assert_eq!(value["ruby_annotations"][0]["base"], "hello");
        assert_eq!(value["ruby_annotations"][1]["type"], "ruby");
        assert_eq!(value["ruby_annotations"][1]["base"], "世");
        assert_eq!(value["ruby_annotations"][1]["ruby"], "せ");
    }

    #[test]
    fn safe_bridge_file_name_matches_android_provider_rules() {
        assert_eq!(safe_bridge_file_name("  春/日:影*  "), "春_日_影_");
        assert_eq!(safe_bridge_file_name(""), "untitled");
        assert_eq!(safe_bridge_file_name("R"), "R");
    }

    #[test]
    fn tail_chars_keeps_short_content_unchanged() {
        assert_eq!(tail_chars("abc", 10), "abc");
    }

    #[test]
    fn tail_chars_truncates_at_char_boundaries() {
        let truncated = tail_chars("春日影abcdef", 6);
        assert!(truncated.ends_with("abcdef"));
        assert!(truncated.starts_with("..."));
    }

    #[test]
    fn format_app_log_line_sanitizes_scope_and_message() {
        let line = format_app_log_line("ui events", "first\nsecond");
        assert!(line.contains("ui_events"));
        assert!(line.contains("first\\nsecond"));
    }
}

/// 一键搜索并获取歌词（如果搜索结果唯一）
#[tauri::command]
async fn search_and_get(
    app: AppHandle,
    title: String,
    artist: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let searcher = state.searcher.lock().await;
    write_app_lsp_log_if_enabled(
        &app,
        &state,
        "search",
        &format!(
            "search_and_get title=\"{}\" artist=\"{}\"",
            title,
            artist.as_deref().unwrap_or("")
        ),
    )
    .await;

    let process_result = searcher.process_song(&title, artist.as_deref()).await;

    // 如果有缓存的结果，直接返回
    if process_result.status == "success" {
        drop(searcher);
        let response = serde_json::to_value(process_result).map_err(|e| e.to_string())?;
        save_saved_lyrics_from_response(&response)?;
        save_salt_bridge_cache(&app, &response)?;
        write_app_lsp_log_if_enabled(&app, &state, "search", "search_and_get direct success").await;
        return Ok(response);
    }

    // 只要有搜索结果，就自动取第一条（用户已经点击选择了）
    if !process_result.search_results.is_empty() {
        let result = searcher.select_result(process_result, 0).await;
        drop(searcher);
        let response = serde_json::to_value(result).map_err(|e| e.to_string())?;
        save_saved_lyrics_from_response(&response)?;
        save_salt_bridge_cache(&app, &response)?;
        write_app_lsp_log_if_enabled(
            &app,
            &state,
            "search",
            "search_and_get selected first result",
        )
        .await;
        return Ok(response);
    }

    drop(searcher);
    write_app_lsp_log_if_enabled(&app, &state, "search", "search_and_get no results").await;
    serde_json::to_value(process_result).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_saved_lyrics(sort_by: Option<String>) -> Result<serde_json::Value, String> {
    let mut entries = list_lyrics_annotations_cache(None).map_err(|e| e.to_string())?;
    let sort_by = sort_by.unwrap_or_else(|| "title".to_string());

    entries.sort_by(|a, b| {
        let left = if sort_by == "artist" {
            a.artist.as_deref().unwrap_or("")
        } else {
            a.title.as_deref().unwrap_or("")
        };
        let right = if sort_by == "artist" {
            b.artist.as_deref().unwrap_or("")
        } else {
            b.title.as_deref().unwrap_or("")
        };
        left.to_lowercase()
            .cmp(&right.to_lowercase())
            .then_with(|| {
                a.title
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .cmp(&b.title.as_deref().unwrap_or("").to_lowercase())
            })
    });

    let summaries: Vec<serde_json::Value> = entries
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "title": entry.title.unwrap_or_else(|| "未命名歌曲".to_string()),
                "artist": entry.artist.unwrap_or_default(),
                "album": entry.album.unwrap_or_default(),
                "cover_url": entry.cover_url.unwrap_or_default(),
                "lyrics_url": entry.url,
                "saved_at": entry.timestamp.to_rfc3339(),
                "annotation_count": entry.annotations.len(),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "status": "success",
        "sort_by": sort_by,
        "songs": summaries,
    }))
}

#[tauri::command]
async fn get_saved_lyrics(url: String) -> Result<serde_json::Value, String> {
    let entry = get_lyrics_annotations_cache_entry(&url, None)
        .ok_or_else(|| "已保存歌词不存在".to_string())?;
    Ok(lyrics_success_response(
        entry.title.unwrap_or_else(|| "未命名歌曲".to_string()),
        entry.artist,
        entry.url,
        &entry.annotations,
        entry.album,
        entry.cover_url,
    ))
}

#[tauri::command]
async fn hydrate_saved_lyrics_metadata(
    url: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let entry = get_lyrics_annotations_cache_entry(&url, None)
        .ok_or_else(|| "已保存歌词不存在".to_string())?;

    if entry
        .cover_url
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Ok(serde_json::json!({
            "status": "success",
            "lyrics_url": entry.url,
            "album": entry.album.unwrap_or_default(),
            "cover_url": entry.cover_url.unwrap_or_default(),
        }));
    }

    let searcher = state.searcher.lock().await;
    let html = searcher
        .get_lyrics_with_ruby(&url)
        .await
        .ok_or_else(|| "无法从UtaTen读取歌曲页面".to_string())?;
    let metadata = UtaTenSearcher::extract_song_page_metadata(&html);
    drop(searcher);

    let album = metadata.album.or(entry.album);
    let cover_url = metadata.cover_url.or(entry.cover_url);
    save_lyrics_annotations_cache_with_metadata(
        &entry.url,
        &entry.annotations,
        entry.title.as_deref(),
        entry.artist.as_deref(),
        album.as_deref(),
        cover_url.as_deref(),
        None,
    )
    .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "status": "success",
        "lyrics_url": entry.url,
        "album": album.unwrap_or_default(),
        "cover_url": cover_url.unwrap_or_default(),
    }))
}

#[tauri::command]
async fn delete_saved_lyrics(url: String) -> Result<bool, String> {
    delete_lyrics_annotations_cache(&url, None).map_err(|e| e.to_string())
}

/// 获取缓存统计
#[tauri::command]
async fn get_cache_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let searcher = state.searcher.lock().await;
    let (lyrics_stats, search_stats) = searcher.cache().stats();
    let stats = serde_json::json!({
        "lyrics": {"total": lyrics_stats.total, "valid": lyrics_stats.valid},
        "search": {"total": search_stats.total, "valid": search_stats.valid}
    });
    serde_json::to_value(stats).map_err(|e| e.to_string())
}

/// 清除缓存
#[tauri::command]
async fn clear_cache(state: State<'_, AppState>) -> Result<(), String> {
    let searcher = state.searcher.lock().await;
    searcher.cache().clear_all().await;
    clear_search_response_cache(None).map_err(|e| e.to_string())?;
    clear_lyrics_annotations_cache(None).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }
            Ok(())
        })
        .manage(AppState {
            searcher: Mutex::new(create_searcher()),
            lsp_logging_enabled: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            search_lyrics,
            get_lyrics,
            search_and_get,
            take_salt_launch_request,
            bind_salt_song_lyrics,
            get_cache_stats,
            list_saved_lyrics,
            get_saved_lyrics,
            hydrate_saved_lyrics_metadata,
            delete_saved_lyrics,
            clear_cache,
            set_lsp_logging_enabled,
            append_lsp_log,
            get_lsp_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
