use crate::{CacheManager, UtaTenSearcher};
use anyhow::{ensure, Context, Result};
use std::path::Path;

/// 获取歌曲原始逐字歌词和 UtaTen 注音，保存可复核的来源数据。
pub async fn prepare_source(
    title: &str,
    artist: &str,
    lyric_response: &Path,
    output: &Path,
) -> Result<()> {
    ensure!(!title.trim().is_empty(), "歌曲名称不能为空");
    ensure!(!artist.trim().is_empty(), "歌手名称不能为空");
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let searcher = UtaTenSearcher::new(CacheManager::new());
    let response: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(lyric_response)?)?;
    let original = crate::qm_decrypt::decrypt_qm_lyrics(
        response
            .pointer("/req_0/data/lyric")
            .and_then(|v| v.as_str())
            .context("原文 QRC 不存在")?,
    )
    .context("原文 QRC 解密失败")?;
    let romaji = crate::qm_decrypt::decrypt_qm_lyrics(
        response
            .pointer("/req_0/data/roma")
            .and_then(|v| v.as_str())
            .context("罗马音 QRC 不存在")?,
    )
    .context("罗马音 QRC 解密失败")?;
    let ruby_search = searcher
        .search_with_options_uncached(title, Some(artist), "title", 1)
        .await;
    std::fs::write(
        output.with_file_name("ruby-search.json"),
        serde_json::to_string_pretty(&ruby_search)?,
    )?;
    let ruby_song = ruby_search
        .results
        .iter()
        .find(|song| {
            let name = song.title.trim().to_lowercase();
            let requested_name = title.trim().to_lowercase();
            song.source == "utaten"
                && (name == requested_name
                    || name
                        .strip_prefix(&requested_name)
                        .is_some_and(|suffix| suffix.trim_start().starts_with('(')))
                && song.artist.trim().to_lowercase() == artist.trim().to_lowercase()
        })
        .context("没有找到歌名和歌手完全匹配的 UtaTen 注音来源")?;
    let html = searcher
        .get_lyrics_with_ruby(&ruby_song.url)
        .await
        .context("获取 UtaTen 注音失败")?;
    let annotations = searcher.extract_ruby_lyrics(&html);
    ensure!(!annotations.is_empty(), "UtaTen 注音为空");
    let han = regex::Regex::new(r"\p{Han}")?;
    for element in &annotations {
        if let Some(base) = element.base.as_deref().filter(|base| han.is_match(base)) {
            ensure!(
                element.element_type == "ruby"
                    && element
                        .ruby
                        .as_deref()
                        .is_some_and(|reading| !reading.trim().is_empty()),
                "UtaTen 汉字缺少注音：{}",
                base
            );
        }
    }
    let document = serde_json::json!({
        "title": title,
        "artist": artist,
        "ruby_url": format!("https://utaten.com{}", ruby_song.url),
        "ruby_annotations": annotations,
        "original_qrc": original,
        "romaji_qrc": romaji,
    });
    std::fs::write(output.with_file_name("ruby-source.html"), html)?;
    std::fs::write(output, serde_json::to_string_pretty(&document)?)?;
    println!("{}", output.display());
    Ok(())
}
