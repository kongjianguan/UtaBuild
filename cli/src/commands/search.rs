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

fn sanitize_filename(s: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut result = s.to_string();
    for c in invalid_chars {
        result = result.replace(c, "_");
    }
    result
}

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

pub async fn execute(
    title: Option<String>,
    artist: Option<String>,
    page: u32,
    select: Option<u32>,
    cache_dir: Option<PathBuf>,
    output: Option<String>,
    output_default: bool,
) -> anyhow::Result<()> {
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
        let cache = CacheManager::new();
        let searcher = Arc::new(UtaTenSearcher::new(cache));

        let search_response = searcher
            .search_with_options(&title, artist_ref, "title", page)
            .await;
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
            } else if output_default {
                let artist_str = cached_lyrics.artist.as_deref().unwrap_or("");
                let title_str = cached_lyrics.title.as_deref().unwrap_or("");
                let filename = generate_default_filename(artist_str, title_str);
                write_output_to_file(&filename, &json_content)?;
                info!("已输出到文件: {}", filename);
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
            } else if output_default {
                let filename = generate_default_filename(
                    &selected_result.found_artist,
                    &selected_result.found_title,
                );
                write_output_to_file(&filename, &json_content)?;
                info!("已输出到文件: {}", filename);
            } else {
                println!("{}", json_content);
            }
        } else {
            let output =
                ErrorOutput::error(selected_result.error.as_deref().unwrap_or("获取歌词失败"));
            println!("{}", output.to_json()?);
        }
    } else {
        let cache = CacheManager::new();
        let searcher = Arc::new(UtaTenSearcher::new(cache));

        let search_response = searcher
            .search_with_options(&title, artist_ref, "title", page)
            .await;
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

        let exact_matches: Vec<_> = process_result
            .search_results
            .iter()
            .filter(|r| is_exact_match(&title, artist_ref, r))
            .collect();

        if exact_matches.len() == 1 {
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
                } else if output_default {
                    let artist_str = cached_lyrics.artist.as_deref().unwrap_or("");
                    let title_str = cached_lyrics.title.as_deref().unwrap_or("");
                    let filename = generate_default_filename(artist_str, title_str);
                    write_output_to_file(&filename, &json_content)?;
                    info!("已输出到文件: {}", filename);
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
                } else if output_default {
                    let filename = generate_default_filename(
                        &selected_result.found_artist,
                        &selected_result.found_title,
                    );
                    write_output_to_file(&filename, &json_content)?;
                    info!("已输出到文件: {}", filename);
                } else {
                    println!("{}", json_content);
                }
            } else {
                let output =
                    ErrorOutput::error(selected_result.error.as_deref().unwrap_or("获取歌词失败"));
                println!("{}", output.to_json()?);
            }
        } else {
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
