use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};
use crate::state::AppState;

#[tauri::command]
pub async fn take_salt_launch_request(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    let mut found_path = None;
    for path in crate::commands::lyrics::salt_pending_request_paths(&app)? {
        if path.is_file() {
            found_path = Some(path);
            break;
        }
    }
    let Some(path) = found_path else {
        return Ok(None);
    };

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    for candidate in crate::commands::lyrics::salt_pending_request_paths(&app)? {
        let _ = fs::remove_file(candidate);
    }
    let value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    write_app_lsp_log_if_enabled(
        &app,
        &state,
        "salt",
        &format!("take_salt_launch_request {}", crate::compact_json(&value)),
    )
    .await;
    Ok(Some(value))
}

#[tauri::command]
pub async fn bind_salt_song_lyrics(
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
    crate::commands::lyrics::save_salt_bridge_cache_for_title(&app, &salt_title, &bound)?;
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

/// 开关应用自带的轻量lsp日志系统。
#[tauri::command]
pub async fn set_lsp_logging_enabled(
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
pub async fn append_lsp_log(
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
pub async fn get_lsp_logs(app: AppHandle) -> Result<String, String> {
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
            crate::tail_chars(&content, 64 * 1024)
        ));
    }

    if sections.is_empty() {
        Ok("暂无lsp日志".to_string())
    } else {
        Ok(sections.join("\n\n"))
    }
}

pub(crate) async fn write_app_lsp_log_if_enabled(
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

fn lsp_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(data_dir.join("utabuild").join("lsp_settings.json"))
}

pub(crate) fn format_app_log_line(scope: &str, message: &str) -> String {
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

/// Write LSP settings to a JSON file that the ContentProvider can serve.
#[tauri::command]
pub async fn set_lsp_settings(
    app: AppHandle,
    settings: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Sync lspLogEnabled to AppState
    if let Some(enabled) = settings.get("lspLogEnabled").and_then(|v| v.as_bool()) {
        let mut logging_enabled = state.lsp_logging_enabled.lock().await;
        *logging_enabled = enabled;
    }

    // Write settings to file
    let path = lsp_settings_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;

    write_app_lsp_log(
        &app,
        "settings",
        &format!("lsp settings saved: {}", crate::compact_json(&settings)),
    )
}
