use tauri::{Manager, State};
use tokio::sync::Mutex;
use utabuild_cli::{CacheManager, UtaTenSearcher};

/// 应用状态
struct AppState {
    searcher: Mutex<UtaTenSearcher>,
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
    title: String,
    artist: Option<String>,
    page: Option<u32>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let searcher = state.searcher.lock().await;
    let result = searcher
        .search_with_options(&title, artist.as_deref(), "title", page.unwrap_or(1))
        .await;

    serde_json::to_value(result).map_err(|e| e.to_string())
}

/// 选择搜索结果，获取歌词
#[tauri::command]
async fn get_lyrics(
    url: String,
    title: String,
    artist: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let searcher = state.searcher.lock().await;

    // 按CLI逻辑：直接用URL获取歌词，返回前端期望的格式
    match searcher.get_lyrics_with_ruby(&url).await {
        Some(html_content) => {
            // 解析歌词和ruby
            let elements = searcher.extract_ruby_lyrics(&html_content);

            // 直接序列化LyricElement，serde会自动把element_type转成type
            let ruby_annotations: Vec<serde_json::Value> = elements
                .iter()
                .map(|e| serde_json::to_value(e).unwrap_or_default())
                .collect();

            let resp = serde_json::json!({
                "status": "success",
                "found_title": title,
                "found_artist": artist,
                "lyrics_url": url,
                "ruby_annotations": ruby_annotations
            });

            Ok(resp)
        }
        None => serde_json::to_value(serde_json::json!({
            "status": "error",
            "error": "歌詞の取得に失敗しました"
        }))
        .map_err(|e| e.to_string()),
    }
}

/// 一键搜索并获取歌词（如果搜索结果唯一）
#[tauri::command]
async fn search_and_get(
    title: String,
    artist: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let searcher = state.searcher.lock().await;

    let process_result = searcher.process_song(&title, artist.as_deref()).await;

    // 如果有缓存的结果，直接返回
    if process_result.status == "success" {
        return serde_json::to_value(process_result).map_err(|e| e.to_string());
    }

    // 只要有搜索结果，就自动取第一条（用户已经点击选择了）
    if !process_result.search_results.is_empty() {
        let result = searcher.select_result(process_result, 0).await;
        return serde_json::to_value(result).map_err(|e| e.to_string());
    }

    serde_json::to_value(process_result).map_err(|e| e.to_string())
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
        })
        .invoke_handler(tauri::generate_handler![
            search_lyrics,
            get_lyrics,
            search_and_get,
            get_cache_stats,
            clear_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
