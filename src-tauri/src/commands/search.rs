use tauri::{AppHandle, State};
use serde_json;
use utabuild_cli::cache::{get_search_response_cache, save_search_response_cache};
use crate::state::AppState;

/// 搜索歌词
#[tauri::command]
pub async fn search_lyrics(
    app: AppHandle,
    title: String,
    artist: Option<String>,
    page: Option<u32>,
    use_cache: Option<bool>,
    lyric_source: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let page = page.unwrap_or(1);
    let use_cache = use_cache.unwrap_or(true);
    let is_qq = lyric_source.as_deref() == Some("qq_music");
    let is_ne = lyric_source.as_deref() == Some("netease");
    let search_type = if is_qq {
        "qq_music"
    } else if is_ne {
        "netease"
    } else {
        "title"
    };

    crate::commands::lsp::write_app_lsp_log_if_enabled(
        &app,
        &state,
        "search",
        &format!(
            "search_lyrics title=\"{}\" artist=\"{}\" page={} use_cache={} source={}",
            title,
            artist.as_deref().unwrap_or(""),
            page,
            use_cache,
            search_type
        ),
    )
    .await;

    if use_cache {
        if let Some(cached_response) =
            get_search_response_cache(&title, artist.as_deref(), search_type, page, None)
        {
            crate::commands::lsp::write_app_lsp_log_if_enabled(&app, &state, "search", "search_lyrics cache hit").await;
            return serde_json::to_value(cached_response).map_err(|e| e.to_string());
        }
    }

    let searcher = state.searcher.lock().await;
    let result = if is_qq {
        searcher
            .search_qq_music(&title, artist.as_deref(), page)
            .await
    } else if is_ne {
        searcher
            .search_netease(&title, artist.as_deref(), page)
            .await
    } else if use_cache {
        searcher
            .search_with_options(&title, artist.as_deref(), search_type, page)
            .await
    } else {
        searcher
            .search_with_options_uncached(&title, artist.as_deref(), search_type, page)
            .await
    };
    drop(searcher);

    if result.error.is_none() {
        save_search_response_cache(
            &title,
            artist.as_deref(),
            search_type,
            page,
            result.clone(),
            None,
        )
        .map_err(|e| e.to_string())?;
        crate::commands::lsp::write_app_lsp_log_if_enabled(
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
        crate::commands::lsp::write_app_lsp_log_if_enabled(
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

/// 一键搜索并获取歌词（如果搜索结果唯一）
#[tauri::command]
pub async fn search_and_get(
    app: AppHandle,
    title: String,
    artist: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let searcher = state.searcher.lock().await;
    crate::commands::lsp::write_app_lsp_log_if_enabled(
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
        crate::commands::lyrics::save_saved_lyrics_from_response(&response)?;
        crate::commands::lyrics::save_salt_bridge_cache(&app, &response)?;
        crate::commands::lsp::write_app_lsp_log_if_enabled(&app, &state, "search", "search_and_get direct success").await;
        return Ok(response);
    }

    // 只要有搜索结果，就自动取第一条（用户已经点击选择了）
    if !process_result.search_results.is_empty() {
        let result = searcher.select_result(process_result, 0).await;
        drop(searcher);
        let response = serde_json::to_value(result).map_err(|e| e.to_string())?;
        crate::commands::lyrics::save_saved_lyrics_from_response(&response)?;
        crate::commands::lyrics::save_salt_bridge_cache(&app, &response)?;
        crate::commands::lsp::write_app_lsp_log_if_enabled(
            &app,
            &state,
            "search",
            "search_and_get selected first result",
        )
        .await;
        return Ok(response);
    }

    drop(searcher);
    crate::commands::lsp::write_app_lsp_log_if_enabled(&app, &state, "search", "search_and_get no results").await;
    serde_json::to_value(process_result).map_err(|e| e.to_string())
}
