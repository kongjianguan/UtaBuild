//! 搜索模块
//!
//! 本模块实现了歌词搜索的核心逻辑，包括按标题/艺术家搜索、通过 URL 直接获取歌词、
//! 精确匹配检测、多数据源并行搜索、结果输出等功能。

use crate::cache::{get_lyrics_cache, save_lyrics_cache};
use crate::cache_manager::CacheManager;
use crate::commands::history::add_to_history;
use crate::models::SearchResult;
use crate::models::{LyricsSearchResponse, SearchResponse};
use crate::output::{ErrorOutput, LyricsOutput, SearchOutput};
use crate::searcher::UtaTenSearcher;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

/// 执行基于 URL 的歌词获取：跳过搜索步骤，直接从 URL 获取歌词。
///
/// - `url`: 歌词页面的 URL
/// - `output`: 可选的输出文件路径
/// - `format`: 输出格式（"json" 或 "html"）
pub async fn execute_from_url(
    url: Option<String>,
    output: Option<String>,
    format: String,
    _cache_dir: Option<PathBuf>,
) -> anyhow::Result<()> {
    let _format = format;
    let url = match url {
        Some(u) if !u.trim().is_empty() => u.trim().to_string(),
        _ => {
            let output = ErrorOutput::error("必须提供 --url 参数，且不能为空");
            println!("{}", output.to_json()?);
            return Ok(());
        }
    };

    debug!("从 URL 获取歌词: {}", url);

    // 首先检查缓存
    if let Some(cached_lyrics) = get_lyrics_cache(&url) {
        info!("歌词缓存命中，直接输出");
        let json_content = cached_lyrics.to_json()?;
        if let Some(output_path) = output {
            write_output_to_file(&output_path, &json_content)?;
            info!("已输出到文件: {}", output_path);
        } else {
            println!("{}", json_content);
        }
        return Ok(());
    }

    let cache = CacheManager::new();
    let searcher = Arc::new(UtaTenSearcher::new(cache));

    info!("从 URL 获取歌词: {}", url);
    match searcher.fetch_lyrics_from_url(&url).await {
        Some(annotations) if !annotations.is_empty() => {
            let lyrics_output = LyricsOutput::success(
                String::new(),
                String::new(),
                url.clone(),
                &annotations,
            );

            info!("保存歌词到缓存: {}", url);
            if let Err(e) = save_lyrics_cache(&url, lyrics_output.clone()) {
                debug!("保存歌词缓存失败: {}", e);
            }

            let json_content = lyrics_output.to_json()?;
            if let Some(output_path) = output {
                write_output_to_file(&output_path, &json_content)?;
                info!("已输出到文件: {}", output_path);
            } else {
                println!("{}", json_content);
            }
        }
        _ => {
            let output = ErrorOutput::error("无法从该 URL 获取歌词");
            println!("{}", output.to_json()?);
        }
    }

    Ok(())
}

/// 清理文件名中的非法字符，将 `< > : " / \ | ? *` 替换为下划线。
///
/// - `s`: 原始文件名
/// 返回: 过滤后的安全文件名
fn sanitize_filename(s: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut result = s.to_string();
    for c in invalid_chars {
        result = result.replace(c, "_");
    }
    result
}

/// 根据艺术家和标题生成默认的输出文件名（格式：`艺术家 - 标题.json`）。
///
/// - `artist`: 艺术家名称
/// - `title`: 歌曲标题
/// 返回: 生成的文件名字符串
fn generate_default_filename(artist: &str, title: &str) -> String {
    let artist = sanitize_filename(artist);
    let title = sanitize_filename(title);

    if artist.is_empty() && title.is_empty() {
        "unknown.json".to_string()
    } else if artist.is_empty() {
        format!("{}.json", title)
    } else if title.is_empty() {
        format!("{}.json", artist)
    } else {
        format!("{} - {}.json", artist, title)
    }
}

/// 将字符串内容写入指定的文件路径，自动创建父目录。
///
/// - `path`: 输出文件路径
/// - `content`: 要写入的文件内容
/// 返回: 写入成功返回 Ok(())，失败返回错误
fn write_output_to_file(path: &str, content: &str) -> anyhow::Result<()> {
    let path = Path::new(path);

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(path, content)?;
    Ok(())
}

/// 根据格式验证并补全输出文件路径。
///
/// 如果 `output_path` 有后缀且与 `format` 不匹配，返回错误。
/// 如果 `output_path` 无后缀或后缀不是 json/html，自动补上格式对应的后缀。
///
/// - `output_path`: 用户指定的输出文件路径
/// - `format`: 输出格式（"json" 或 "html"）
/// 返回: 验证通过并可能补全后缀的文件路径
pub fn validate_output_format(output_path: &str, format: &str) -> anyhow::Result<String> {
    let path = std::path::Path::new(output_path);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => {
            let lower = ext.to_lowercase();
            match (format, lower.as_str()) {
                ("json", "json") | ("html", "html") => Ok(output_path.to_string()),
                ("json", "html") | ("html", "json") => {
                    return Err(anyhow::anyhow!(
                        "格式冲突: --format {} 与文件后缀 .{} 不匹配",
                        format,
                        ext
                    ));
                }
                _ => Ok(format!("{}.{}", output_path, format)),
            }
        }
        None => Ok(format!("{}.{}", output_path, format)),
    }
}

/// 判断搜索结果的标题和艺术家是否与查询条件精确匹配。
///
/// 比较时会忽略括号内的副标题以及大小写差异。
///
/// - `title`: 查询的歌曲标题
/// - `artist`: 可选的查询艺术家
/// - `result`: 搜索结果条目
/// 返回: 是否精确匹配
fn is_exact_match(title: &str, artist: Option<&str>, result: &SearchResult) -> bool {
    let title_match = {
        let clean_query = title
            .split('(')
            .next()
            .unwrap_or(title)
            .trim()
            .to_lowercase();
        let clean_result = result
            .title
            .split('(')
            .next()
            .unwrap_or(&result.title)
            .trim()
            .to_lowercase();
        clean_query == clean_result
    };

    if !title_match {
        return false;
    }

    if let Some(artist_query) = artist {
        let artist_query_lower = artist_query.to_lowercase().trim().to_string();
        let artist_result_lower = result.artist.to_lowercase().trim().to_string();

        if artist_query_lower.is_empty() {
            return true;
        }

        artist_query_lower == artist_result_lower
            || artist_result_lower.contains(&artist_query_lower)
            || artist_query_lower.contains(&artist_result_lower)
    } else {
        true
    }
}

/// 将 SearchResponse 转换为 LyricsSearchResponse，提取搜索结果中的关键信息。
///
/// - `title`: 查询的歌曲标题
/// - `artist`: 可选的查询艺术家
/// - `response`: 原始搜索响应
/// 返回: 处理后的搜索结果
fn search_response_to_process_result(
    title: &str,
    artist: Option<&str>,
    response: SearchResponse,
) -> LyricsSearchResponse {
    let mut result =
        LyricsSearchResponse::new(title.to_string(), artist.map(|value| value.to_string()));

    result.status = response.status.clone();
    result.error = response.error.clone();
    result.search_results = response.results;
    result.matched = !result.search_results.is_empty();

    if let Some(first_result) = result.search_results.first() {
        result.found_title = first_result.title.clone();
        result.found_artist = first_result.artist.clone();
        result.lyrics_url = first_result.url.clone();
    }

    result
}

/// 并行从 UtaTen、QQ 音乐和网易云音乐三个数据源搜索，然后合并结果。
///
/// - `searcher`: UtaTen 搜索器实例
/// - `title`: 查询的歌曲标题
/// - `artist`: 可选的查询艺术家
/// - `page`: 分页页码
/// 返回: 合并后的搜索响应
async fn parallel_search_all(
    searcher: &UtaTenSearcher,
    title: &str,
    artist: Option<&str>,
    page: u32,
) -> SearchResponse {
    info!("Performing parallel search across UtaTen, QQ Music, and NetEase");

    let (utaten, qq, ne) = tokio::join!(
        searcher.search_with_options(title, artist, "title", page),
        searcher.search_qq_music(title, artist, page),
        searcher.search_netease(title, artist, page),
    );

    let total = utaten.results.len() + qq.results.len() + ne.results.len();
    info!("Parallel search: UtaTen={}, QQ={}, NetEase={} (total={})",
        utaten.results.len(), qq.results.len(), ne.results.len(), total);

    // 在取出 results 之前收集错误信息
    let utaten_err = utaten.error.clone();
    let qq_err = qq.error.clone();
    let ne_err = ne.error.clone();

    let mut merged = SearchResponse::new();
    merged.query_title = Some(title.to_string());
    merged.query_artist = artist.map(|a| a.to_string());
    merged.search_type = "title".to_string();
    merged.page = page;

    let mut all = utaten.results;
    all.extend(qq.results);
    all.extend(ne.results);
    merged.results = all;
    merged.pagination = utaten.pagination;

    if total > 0 {
        merged.status = "select".to_string();
    } else {
        let errors: Vec<String> = [utaten_err, qq_err, ne_err]
            .into_iter()
            .flatten()
            .collect();
        if errors.is_empty() {
            merged.status = "not_found".to_string();
        } else {
            merged.status = "error".to_string();
            merged.error = Some(errors.join("; "));
        }
    }

    merged
}

/// 执行歌词搜索的主函数
///
/// 支持两种模式：
/// 1. `--select` 模式：直接指定搜索结果索引获取歌词
/// 2. 自动模式：先搜索，若只有一个精确匹配则自动获取歌词，否则展示搜索结果列表。
///
/// - `title`: 可选的歌曲标题
/// - `artist`: 可选的艺术家名称
/// - `page`: 分页页码
/// - `select`: 可选的选择索引
/// - `cache_dir`: 可选的缓存目录路径
/// - `output`: 可选的输出文件路径
/// - `format`: 输出格式（"json" 或 "html"）
pub async fn execute(
    title: Option<String>,
    artist: Option<String>,
    page: u32,
    select: Option<u32>,
    cache_dir: Option<PathBuf>,
    output: Option<String>,
    format: String,
) -> anyhow::Result<()> {
    let _format = format;
    debug!(
        "执行搜索: title={:?}, artist={:?}, page={}, select={:?}",
        title, artist, page, select
    );

    if title.is_none() && artist.is_none() {
        let output = ErrorOutput::error("必须提供 --title 或 --artist 参数");
        println!("{}", output.to_json()?);
        return Ok(());
    }

    let title = title.unwrap_or_default();
    let artist_ref = artist.as_deref();

    info!("正在搜索歌词: {} - {:?}", title, artist);

    if let Some(index) = select {
        // --select 模式：直接按索引获取歌词
        let cache = CacheManager::new();
        let searcher = Arc::new(UtaTenSearcher::new(cache));

        let search_response = parallel_search_all(
            &searcher, &title, artist_ref, page,
        ).await;

        let process_result = search_response_to_process_result(&title, artist_ref, search_response);

        if process_result.search_results.is_empty() {
            let output = if let Some(error) = process_result.error.as_deref() {
                ErrorOutput::error(error)
            } else {
                ErrorOutput::no_results("未找到匹配的歌词")
            };
            println!("{}", output.to_json()?);
            return Ok(());
        }

        let index = index as usize;
        if index >= process_result.search_results.len() {
            let output = ErrorOutput::error(&format!(
                "无效的选择: 索引 {} 超出范围 (0-{})",
                index,
                process_result.search_results.len() - 1
            ));
            println!("{}", output.to_json()?);
            return Ok(());
        }

        let selected_search_result = &process_result.search_results[index];
        let lyrics_url = &selected_search_result.url;

        info!("检查歌词缓存: {}", lyrics_url);
        if let Some(cached_lyrics) = get_lyrics_cache(lyrics_url) {
            info!("歌词缓存命中，直接输出");

            add_to_history(
                &selected_search_result.title,
                &selected_search_result.artist,
                lyrics_url,
                selected_search_result.lyricist.clone(),
                selected_search_result.composer.clone(),
                cache_dir.as_ref(),
            )?;

            let json_content = cached_lyrics.to_json()?;

            if let Some(output_path) = output {
                write_output_to_file(&output_path, &json_content)?;
                info!("已输出到文件: {}", output_path);
            } else {
                println!("{}", json_content);
            }
            return Ok(());
        }

        info!("歌词缓存未命中，从 UtaTen 获取歌词");

        add_to_history(
            &selected_search_result.title,
            &selected_search_result.artist,
            lyrics_url,
            selected_search_result.lyricist.clone(),
            selected_search_result.composer.clone(),
            cache_dir.as_ref(),
        )?;

        let selected_result = searcher.select_result(process_result, index).await;

        if selected_result.status == "success" {
            let lyrics_output = LyricsOutput::success(
                selected_result.found_title.clone(),
                selected_result.found_artist.clone(),
                selected_result.lyrics_url.clone(),
                &selected_result.ruby_annotations,
            );

            info!("保存歌词到缓存: {}", selected_result.lyrics_url);
            if let Err(e) = save_lyrics_cache(&selected_result.lyrics_url, lyrics_output.clone()) {
                debug!("保存歌词缓存失败: {}", e);
            }

            let json_content = lyrics_output.to_json()?;

            if let Some(output_path) = output {
                write_output_to_file(&output_path, &json_content)?;
                info!("已输出到文件: {}", output_path);
            } else {
                println!("{}", json_content);
            }
        } else {
            let output =
                ErrorOutput::error(selected_result.error.as_deref().unwrap_or("获取歌词失败"));
            println!("{}", output.to_json()?);
        }
    } else {
        // 自动模式：搜索并判断是否自动获取
        let cache = CacheManager::new();
        let searcher = Arc::new(UtaTenSearcher::new(cache));

        let search_response = parallel_search_all(
            &searcher, &title, artist_ref, page,
        ).await;

        let process_result =
            search_response_to_process_result(&title, artist_ref, search_response.clone());

        if process_result.search_results.is_empty() {
            let output = if let Some(error) = process_result.error.as_deref() {
                ErrorOutput::error(error)
            } else {
                ErrorOutput::no_results("未找到匹配的歌词")
            };
            println!("{}", output.to_json()?);
            return Ok(());
        }

        // 统计精确匹配的结果数量
        let exact_matches: Vec<_> = process_result
            .search_results
            .iter()
            .filter(|r| is_exact_match(&title, artist_ref, r))
            .collect();

        if exact_matches.len() == 1 {
            // 只有一个精确匹配，自动获取歌词
            let exact_result = exact_matches[0];
            let lyrics_url = &exact_result.url;

            info!(
                "找到精确匹配: {} - {}",
                exact_result.title, exact_result.artist
            );
            info!("检查歌词缓存: {}", lyrics_url);

            if let Some(cached_lyrics) = get_lyrics_cache(lyrics_url) {
                info!("歌词缓存命中，直接输出");

                add_to_history(
                    &exact_result.title,
                    &exact_result.artist,
                    lyrics_url,
                    exact_result.lyricist.clone(),
                    exact_result.composer.clone(),
                    cache_dir.as_ref(),
                )?;

                let json_content = cached_lyrics.to_json()?;

                if let Some(output_path) = output {
                    write_output_to_file(&output_path, &json_content)?;
                    info!("已输出到文件: {}", output_path);
                } else {
                    println!("{}", json_content);
                }
                return Ok(());
            }

            info!("歌词缓存未命中，从 UtaTen 获取歌词");

            add_to_history(
                &exact_result.title,
                &exact_result.artist,
                lyrics_url,
                exact_result.lyricist.clone(),
                exact_result.composer.clone(),
                cache_dir.as_ref(),
            )?;

            let index = process_result
                .search_results
                .iter()
                .position(|r| r.url == exact_result.url)
                .unwrap_or(0);

            let selected_result = searcher.select_result(process_result, index).await;

            if selected_result.status == "success" {
                let lyrics_output = LyricsOutput::success(
                    selected_result.found_title.clone(),
                    selected_result.found_artist.clone(),
                    selected_result.lyrics_url.clone(),
                    &selected_result.ruby_annotations,
                );

                info!("保存歌词到缓存: {}", selected_result.lyrics_url);
                if let Err(e) =
                    save_lyrics_cache(&selected_result.lyrics_url, lyrics_output.clone())
                {
                    debug!("保存歌词缓存失败: {}", e);
                }

                let json_content = lyrics_output.to_json()?;

                if let Some(output_path) = output {
                    write_output_to_file(&output_path, &json_content)?;
                    info!("已输出到文件: {}", output_path);
                } else {
                    println!("{}", json_content);
                }
            } else {
                let output =
                    ErrorOutput::error(selected_result.error.as_deref().unwrap_or("获取歌词失败"));
                println!("{}", output.to_json()?);
            }
        } else {
            // 没有精确匹配或匹配过多，展示搜索结果列表供用户选择
            let total_pages = search_response
                .pagination
                .as_ref()
                .map(|pagination| pagination.total_pages)
                .unwrap_or(page.max(1));
            let output = SearchOutput::new(
                Some(title),
                artist,
                page,
                total_pages,
                &process_result.search_results,
            );
            println!("{}", output.to_json()?);
        }
    }

    Ok(())
}

#[cfg(test)]
mod format_tests {
    use super::*;

    #[test]
    fn test_validate_output_format_json_match() {
        let result = validate_output_format("output.json", "json").unwrap();
        assert_eq!(result, "output.json");
    }

    #[test]
    fn test_validate_output_format_html_match() {
        let result = validate_output_format("output.html", "html").unwrap();
        assert_eq!(result, "output.html");
    }

    #[test]
    fn test_validate_output_format_conflict() {
        let result = validate_output_format("output.json", "html");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("格式冲突"));
        assert!(err.contains("html"));
        assert!(err.contains("json"));
    }

    #[test]
    fn test_validate_output_format_conflict_reverse() {
        let result = validate_output_format("output.html", "json");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_output_format_no_extension_json() {
        let result = validate_output_format("output", "json").unwrap();
        assert_eq!(result, "output.json");
    }

    #[test]
    fn test_validate_output_format_no_extension_html() {
        let result = validate_output_format("output", "html").unwrap();
        assert_eq!(result, "output.html");
    }

    #[test]
    fn test_validate_output_format_unknown_extension() {
        let result = validate_output_format("output.txt", "html").unwrap();
        assert_eq!(result, "output.txt.html");
    }
}
