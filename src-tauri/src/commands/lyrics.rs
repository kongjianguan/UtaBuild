use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
use serde_json;
use utabuild_cli::cache::{
    clear_lyrics_annotations_cache, clear_search_response_cache, delete_lyrics_annotations_cache,
    get_lyrics_annotations_cache_entry, list_lyrics_annotations_cache,
    save_lyrics_annotations_cache_with_metadata,
};
use utabuild_cli::LyricElement;
use utabuild_cli::{ArtworkSourcePreference, LyricSourcePreference, UtaTenSearcher};
use crate::state::AppState;

/// 选择搜索结果，获取歌词
#[tauri::command]
pub async fn get_lyrics(
    app: AppHandle,
    url: String,
    title: String,
    artist: Option<String>,
    use_cache: Option<bool>,
    save_salt_bridge: Option<bool>,
    artwork_source: Option<String>,
    lyric_source: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let use_cache = use_cache.unwrap_or(true);
    let save_salt_bridge = save_salt_bridge.unwrap_or(true);
    let artwork_source_preference =
        ArtworkSourcePreference::from_setting(artwork_source.as_deref());
    let lyric_preference = LyricSourcePreference::from_setting(lyric_source.as_deref());

    crate::commands::lsp::write_app_lsp_log_if_enabled(
        &app,
        &state,
        "lyrics",
        &format!(
            "get_lyrics title=\"{}\" artist=\"{}\" url=\"{}\" use_cache={} save_salt_bridge={} source={:?}",
            title,
            artist.as_deref().unwrap_or(""),
            url,
            use_cache,
            save_salt_bridge,
            lyric_preference
        ),
    )
    .await;

    let searcher = state.searcher.lock().await;

    // QQ Music path
    if lyric_preference == LyricSourcePreference::QqMusic {
        let qq_cache_key = format!("qq:{}:{}", title, artist.as_deref().unwrap_or(""));

        if use_cache {
            if let Some(cached_annotations) = searcher.cache().lyrics().get(&qq_cache_key).await {
                drop(searcher);
                let response = lyrics_success_response(
                    title.clone(),
                    artist.clone(),
                    qq_cache_key.clone(),
                    &cached_annotations,
                    None,
                    None,
                );
                if save_salt_bridge {
                    save_salt_bridge_cache(&app, &response)?;
                }
                crate::commands::lsp::write_app_lsp_log_if_enabled(
                    &app,
                    &state,
                    "lyrics",
                    "get_lyrics QQ Music cache hit",
                )
                .await;
                return Ok(response);
            }
        }

        let annotations = searcher
            .fetch_qq_music_lyrics(&title, artist.as_deref())
            .await
            .unwrap_or_default();

        if annotations.is_empty() {
            drop(searcher);
            crate::commands::lsp::write_app_lsp_log_if_enabled(
                &app,
                &state,
                "lyrics",
                "get_lyrics QQ Music failed",
            )
            .await;
            return serde_json::to_value(serde_json::json!({
                "status": "error",
                "error": "QQ Music 歌词获取失败"
            }))
            .map_err(|e| e.to_string());
        }

        searcher
            .cache()
            .lyrics()
            .insert(qq_cache_key.clone(), annotations.clone())
            .await;
        drop(searcher);

        let response = lyrics_success_response(
            title,
            artist,
            qq_cache_key,
            &annotations,
            None,
            None,
        );
        if save_salt_bridge {
            save_salt_bridge_cache(&app, &response)?;
        }
        save_saved_lyrics_from_response(&response)?;
        crate::commands::lsp::write_app_lsp_log_if_enabled(
            &app,
            &state,
            "lyrics",
            &format!("get_lyrics QQ Music success annotations={}", annotations.len()),
        )
        .await;
        return Ok(response);
    }

    // NetEase path
    if lyric_preference == LyricSourcePreference::Netease || url.starts_with("ne:") {
        let ne_song_id = url.strip_prefix("ne:").unwrap_or(&url);
        let ne_cache_key = format!("ne:{}", ne_song_id);

        if use_cache {
            if let Some(cached_annotations) = searcher.cache().lyrics().get(&ne_cache_key).await {
                drop(searcher);
                let response = lyrics_success_response(
                    title.clone(),
                    artist.clone(),
                    ne_cache_key.clone(),
                    &cached_annotations,
                    None,
                    None,
                );
                if save_salt_bridge {
                    save_salt_bridge_cache(&app, &response)?;
                }
                return Ok(response);
            }
        }

        let annotations: Vec<LyricElement> = searcher.ne_source.fetch_lyrics(ne_song_id).await.unwrap_or_default();
        if !annotations.is_empty() {
            searcher.cache().lyrics().insert(ne_cache_key.clone(), annotations.clone()).await;
            drop(searcher);
            let response = lyrics_success_response(title, artist, ne_cache_key, &annotations, None, None);
            if save_salt_bridge {
                save_salt_bridge_cache(&app, &response)?;
            }
            save_saved_lyrics_from_response(&response)?;
            return Ok(response);
        }

        if lyric_preference == LyricSourcePreference::Netease {
            drop(searcher);
            return serde_json::to_value(serde_json::json!({
                "status": "error",
                "error": "NetEase 歌词获取失败"
            })).map_err(|e| e.to_string());
        }
        // Fall through to UtaTen on Auto with ne: URL
    }

    // UtaTen path (existing logic)
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
            crate::commands::lsp::write_app_lsp_log_if_enabled(&app, &state, "lyrics", "get_lyrics memory cache hit")
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
            crate::commands::lsp::write_app_lsp_log_if_enabled(&app, &state, "lyrics", "get_lyrics disk cache hit").await;
            return Ok(response);
        }
    }

    // 按CLI逻辑：直接用URL获取歌词，返回前端期望的格式
    match searcher.get_lyrics_with_ruby(&url).await {
        Some(html_content) => {
            let metadata = searcher
                .resolve_artwork_metadata(
                    &title,
                    artist.as_deref(),
                    UtaTenSearcher::extract_song_page_metadata(&html_content),
                    artwork_source_preference,
                )
                .await;
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
            crate::commands::lsp::write_app_lsp_log_if_enabled(
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
            crate::commands::lsp::write_app_lsp_log_if_enabled(&app, &state, "lyrics", "get_lyrics failed").await;
            serde_json::to_value(serde_json::json!({
                "status": "error",
                "error": "歌詞の取得に失敗しました"
            }))
            .map_err(|e| e.to_string())
        }
    }
}

pub(crate) fn save_salt_bridge_cache(app: &AppHandle, response: &serde_json::Value) -> Result<(), String> {
    let title = response
        .get("found_title")
        .and_then(|value| value.as_str())
        .unwrap_or("untitled");
    save_salt_bridge_cache_for_title(app, title, response)
}

pub(crate) fn save_saved_lyrics_from_response(response: &serde_json::Value) -> Result<(), String> {
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

pub(crate) fn save_salt_bridge_cache_for_title(
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

pub(crate) fn salt_pending_request_paths(app: &AppHandle) -> Result<Vec<PathBuf>, String> {
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

pub(crate) fn safe_bridge_file_name(title: &str) -> String {
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

pub(crate) fn lyrics_success_response(
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

#[tauri::command]
pub async fn list_saved_lyrics(sort_by: Option<String>) -> Result<serde_json::Value, String> {
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
pub async fn get_saved_lyrics(url: String) -> Result<serde_json::Value, String> {
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
pub async fn hydrate_saved_lyrics_metadata(
    url: String,
    force_refresh: Option<bool>,
    artwork_source: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let entry = get_lyrics_annotations_cache_entry(&url, None)
        .ok_or_else(|| "已保存歌词不存在".to_string())?;

    if !force_refresh.unwrap_or(false)
        && entry
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

    // For non-UtaTen URLs, skip the UtaTen page fetch and use matching artwork source
    let (utaten_metadata, artwork_preference) = if url.starts_with("ne:") {
        (
            utabuild_cli::searcher::SongPageMetadata { album: None, cover_url: None },
            ArtworkSourcePreference::Netease,
        )
    } else if url.starts_with("qq:") {
        (
            utabuild_cli::searcher::SongPageMetadata { album: None, cover_url: None },
            ArtworkSourcePreference::QqMusic,
        )
    } else {
        let html = searcher
            .get_lyrics_with_ruby(&url)
            .await
            .ok_or_else(|| "无法从UtaTen读取歌曲页面".to_string())?;
        (
            UtaTenSearcher::extract_song_page_metadata(&html),
            ArtworkSourcePreference::from_setting(artwork_source.as_deref()),
        )
    };

    let metadata = searcher
        .resolve_artwork_metadata(
            entry.title.as_deref().unwrap_or(""),
            entry.artist.as_deref(),
            utaten_metadata,
            artwork_preference,
        )
        .await;
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
pub async fn delete_saved_lyrics(url: String) -> Result<bool, String> {
    delete_lyrics_annotations_cache(&url, None).map_err(|e| e.to_string())
}

/// 获取缓存统计
#[tauri::command]
pub async fn get_cache_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
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
pub async fn clear_cache(state: State<'_, AppState>) -> Result<(), String> {
    let searcher = state.searcher.lock().await;
    searcher.cache().clear_all().await;
    clear_search_response_cache(None).map_err(|e| e.to_string())?;
    clear_lyrics_annotations_cache(None).map_err(|e| e.to_string())?;
    Ok(())
}
