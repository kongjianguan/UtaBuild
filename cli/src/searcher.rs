use crate::cache_manager::{CacheManager, SearchResultEntry};
use crate::models::{LyricElement, LyricsSearchResponse, SearchResult, SearchPagination, SearchResponse};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

#[derive(Debug, Clone, Default)]
pub struct ArtistInfo {
    pub artist: String,
    pub lyricist: Option<String>,
    pub composer: Option<String>,
}

pub fn parse_artist_info(raw: &str) -> ArtistInfo {
    let cleaned: String = raw
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    
    let re_space = Regex::new(r"\s+").unwrap();
    let normalized = re_space.replace_all(&cleaned, " ").trim().to_string();
    
    let re_lyricist = Regex::new(r"作\s*詞[：:]\s*").unwrap();
    let re_composer = Regex::new(r"作\s*曲[：:]\s*").unwrap();
    
    let (artist_part, rest) = if let Some(m) = re_lyricist.find(&normalized) {
        (&normalized[..m.start()], &normalized[m.end()..])
    } else if let Some(m) = re_composer.find(&normalized) {
        (&normalized[..m.start()], &normalized[m.end()..])
    } else {
        return ArtistInfo {
            artist: normalized,
            lyricist: None,
            composer: None,
        };
    };
    
    let artist = artist_part.trim().to_string();
    
    let (lyricist, composer) = if let Some(m) = re_composer.find(rest) {
        let lyricist_text = rest[..m.start()].trim();
        let composer_text = rest[m.end()..].trim();
        (
            if lyricist_text.is_empty() { None } else { Some(lyricist_text.to_string()) },
            if composer_text.is_empty() { None } else { Some(composer_text.to_string()) },
        )
    } else {
        let text = rest.trim();
        (
            if text.is_empty() { None } else { Some(text.to_string()) },
            None,
        )
    };
    
    ArtistInfo {
        artist,
        lyricist,
        composer,
    }
}

const BASE_URL: &str = "https://utaten.com";
const REQUEST_DELAY_MS: u64 = 500;
const REQUEST_TIMEOUT_SECS: u64 = 15;

pub struct UtaTenSearcher {
    client: Client,
    pub cache: CacheManager,
    delay: Duration,
    last_request: Arc<Mutex<Instant>>,
}

impl UtaTenSearcher {
    pub fn new(cache: CacheManager) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::ACCEPT,
                    "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"
                        .parse()
                        .unwrap(),
                );
                headers.insert(
                    reqwest::header::ACCEPT_LANGUAGE,
                    "ja,en-US;q=0.7,en;q=0.3".parse().unwrap(),
                );
                headers
            })
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            cache,
            delay: Duration::from_millis(REQUEST_DELAY_MS),
            last_request: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10))),
        }
    }

    async fn rate_limit(&self) {
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        if elapsed < self.delay {
            tokio::time::sleep(self.delay - elapsed).await;
        }
        *last = Instant::now();
    }

    fn decode_response(bytes: &[u8], headers: &reqwest::header::HeaderMap) -> String {
        let content_encoding = headers
            .get(reqwest::header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok());

        let mut decoded_bytes = bytes.to_vec();

        if content_encoding == Some("gzip") {
            use flate2::read::GzDecoder;
            use std::io::Read;
            let mut decoder = GzDecoder::new(&bytes[..]);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                decoded_bytes = decompressed;
            }
        }

        if let Ok(s) = std::str::from_utf8(&decoded_bytes) {
            return s.to_string();
        }

        let (cow, _encoding, _had_errors) = encoding_rs::SHIFT_JIS.decode(&decoded_bytes);
        let result = cow.into_owned();
        
        if Self::has_japanese(&result) {
            return result;
        }

        let (cow, _, _) = encoding_rs::EUC_JP.decode(&decoded_bytes);
        cow.into_owned()
    }

    fn has_japanese(text: &str) -> bool {
        text.chars()
            .any(|c| ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c))
    }

    pub async fn search(&self, title: &str, artist: Option<&str>) -> Vec<SearchResult> {
        self.search_with_options(title, artist, "title", 1).await.results
    }

    pub async fn search_with_options(
        &self,
        query: &str,
        artist: Option<&str>,
        search_type: &str,
        page: u32,
    ) -> SearchResponse {
        let mut response = SearchResponse::new();
        response.query_title = Some(query.to_string());
        response.query_artist = artist.map(|s| s.to_string());
        response.search_type = search_type.to_string();
        response.page = page;

        self.rate_limit().await;

        let search_query = match (search_type, artist) {
            ("artist", _) => query.trim().to_string(),
            ("title", Some(a)) => format!("{} {}", query.trim(), a.trim()),
            _ => query.trim().to_string(),
        };

        let params = [
            ("layout_search_type", search_type),
            ("layout_search_text", &search_query),
            ("page", &page.to_string()),
        ];

        let url = format!("{}/search", BASE_URL);
        debug!("HTTP GET: {} with params: {:?}", url, params);

        let http_response = match self.client.get(&url).query(&params).send().await {
            Ok(r) => r,
            Err(e) => {
                error!("Search request failed: {}", e);
                response.status = "error".to_string();
                response.error = Some(format!("搜索请求失败: {}", e));
                return response;
            }
        };

        debug!(
            "Response: status={}, content-length={:?}",
            http_response.status(),
            http_response.content_length()
        );

        let headers = http_response.headers().clone();
        let bytes = match http_response.bytes().await {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to read response body: {}", e);
                response.status = "error".to_string();
                response.error = Some(format!("读取响应失败: {}", e));
                return response;
            }
        };

        let html_content = Self::decode_response(&bytes, &headers);
        let document = Html::parse_document(&html_content);

        let table_selector = Selector::parse("table.searchResult.artistLyricList").unwrap();
        let row_selector = Selector::parse("tr").unwrap();
        let artist_cell_selector = Selector::parse("td.searchResult__artist").unwrap();
        let link_selector = Selector::parse("a[href*=\"/lyric/\"]").unwrap();

        let mut results: Vec<SearchResult> = Vec::new();
        let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

        for table in document.select(&table_selector) {
            let mut current_artist = String::new();

            for row in table.select(&row_selector) {
                if let Some(artist_cell) = row.select(&artist_cell_selector).next() {
                    current_artist = artist_cell.text().collect::<String>();
                    current_artist = current_artist.trim().to_string();
                }

                for link in row.select(&link_selector) {
                    if let Some(href) = link.value().attr("href") {
                        if seen_urls.contains(href) {
                            continue;
                        }
                        seen_urls.insert(href.to_string());

                        let link_text: String = link.text().collect();
                        let link_text = link_text.trim().to_string();

                        if !link_text.is_empty() {
                            let artist_info = parse_artist_info(&current_artist);
                            results.push(SearchResult::with_artist_info(
                                link_text,
                                artist_info.artist,
                                href.to_string(),
                                artist_info.lyricist,
                                artist_info.composer,
                            ));
                        }
                    }
                }
            }
        }

        let pagination = self.extract_pagination(&document, page);
        response.pagination = Some(pagination.clone());

        debug!("Returning {} unique results", results.len());
        response.results = results;
        response.status = if response.results.is_empty() { "not_found" } else { "select" }.to_string();

        response
    }

    fn extract_pagination(&self, document: &Html, current_page: u32) -> SearchPagination {
        let pager_selector = Selector::parse("div.pager").unwrap();
        let current_selector = Selector::parse("span.current, span.pager__item--current").unwrap();
        let link_selector = Selector::parse("a[href*=\"page=\"]").unwrap();

        let mut total_pages = current_page;
        let mut has_next = false;

        if let Some(pager) = document.select(&pager_selector).next() {
            for link in pager.select(&link_selector) {
                if let Some(href) = link.value().attr("href") {
                    if let Some(page_num) = href.split("page=").last() {
                        if let Ok(num) = page_num.parse::<u32>() {
                            total_pages = total_pages.max(num);
                        }
                    }
                }
            }

            let next_selector = Selector::parse("a.next, a.pager__item--next").unwrap();
            has_next = pager.select(&next_selector).next().is_some();

            if pager.select(&current_selector).next().is_none() {
                let all_links: Vec<_> = pager.select(&link_selector).collect();
                if let Some(last_link) = all_links.last() {
                    if let Some(href) = last_link.value().attr("href") {
                        if let Some(page_num) = href.split("page=").last() {
                            if let Ok(num) = page_num.parse::<u32>() {
                                total_pages = total_pages.max(num);
                            }
                        }
                    }
                }
            }
        }

        SearchPagination {
            current_page,
            total_pages,
            has_next,
        }
    }

    fn match_artist(found_artist: &str, target: &str) -> bool {
        if found_artist.is_empty() || target.is_empty() {
            return false;
        }

        let found = found_artist.to_lowercase().trim().to_string();
        let target = target.to_lowercase().trim().to_string();

        if found == target {
            return true;
        }
        if found.contains(&target) || target.contains(&found) {
            return true;
        }
        if found.replace(' ', "") == target.replace(' ', "") {
            return true;
        }

        false
    }

    pub async fn get_lyrics_with_ruby(&self, lyric_url: &str) -> Option<String> {
        self.rate_limit().await;

        let full_url = if lyric_url.starts_with("http") {
            lyric_url.to_string()
        } else {
            format!("{}{}", BASE_URL, lyric_url)
        };

        debug!("HTTP GET (lyrics): {}", full_url);

        let response = match self.client.get(&full_url).send().await {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to get lyrics page: {}", e);
                return None;
            }
        };

        debug!(
            "Lyrics page: status={}, content-length={:?}",
            response.status(),
            response.content_length()
        );

        let headers = response.headers().clone();
        let bytes = match response.bytes().await {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to read lyrics response body: {}", e);
                return None;
            }
        };

        Some(Self::decode_response(&bytes, &headers))
    }

    pub fn extract_ruby_lyrics(&self, html_content: &str) -> Vec<LyricElement> {
        let mut elements: Vec<LyricElement> = Vec::new();

        let document = Html::parse_document(html_content);

        let lyric_body_selector = Selector::parse("div.lyricBody").unwrap();
        let medium_selector = Selector::parse("div.medium").unwrap();
        let hiragana_selector = Selector::parse("div.hiragana").unwrap();

        let lyric_body = match document.select(&lyric_body_selector).next() {
            Some(b) => b,
            None => {
                debug!("No div.lyricBody found");
                return elements;
            }
        };

        let medium = match lyric_body.select(&medium_selector).next() {
            Some(m) => m,
            None => {
                debug!("No div.medium found in lyricBody");
                return elements;
            }
        };

        let hiragana = match medium.select(&hiragana_selector).next() {
            Some(h) => h,
            None => {
                debug!("No div.hiragana found in medium");
                return elements;
            }
        };

        debug!("Found hiragana div, processing...");
        self.process_node(hiragana, &mut elements);

        let ruby_count = elements.iter().filter(|e| e.element_type == "ruby").count();
        let text_count = elements.iter().filter(|e| e.element_type == "text").count();
        let linebreak_count = elements
            .iter()
            .filter(|e| e.element_type == "linebreak")
            .count();

        debug!(
            "Extracted {} elements from hiragana (ruby={}, text={}, linebreak={})",
            elements.len(),
            ruby_count,
            text_count,
            linebreak_count
        );

        elements
    }

    fn process_node(&self, node: scraper::ElementRef, elements: &mut Vec<LyricElement>) {
        for child in node.children() {
            match child.value() {
                scraper::Node::Element(element) => {
                    match element.name() {
                        "br" => {
                            elements.push(LyricElement::new_linebreak());
                        }
                        "span" => {
                            let child_ref = scraper::ElementRef::wrap(child).unwrap();
                            let classes: Vec<&str> = child_ref.value().classes().collect();
                            let has_ruby_class = classes.contains(&"ruby");
                            let has_rb_class = classes.contains(&"rb");
                            let has_rt_class = classes.contains(&"rt");
                            
                            if has_ruby_class {
                                let (base_text, ruby_text) = self.extract_ruby_content(child_ref);
                                
                                if !base_text.is_empty()
                                    && !ruby_text.is_empty()
                                    && Self::has_japanese(&ruby_text)
                                {
                                    elements.push(LyricElement::new_ruby(
                                        base_text,
                                        ruby_text,
                                    ));
                                } else if !base_text.is_empty() {
                                    elements.push(LyricElement::new_text(base_text));
                                }
                            } else if has_rb_class || has_rt_class {
                                // 跳过 rb 和 rt，它们已经在 ruby 处理中被提取
                            } else {
                                self.process_node(child_ref, elements);
                            }
                        }
                        _ => {
                            let child_ref = scraper::ElementRef::wrap(child).unwrap();
                            self.process_node(child_ref, elements);
                        }
                    }
                }
                scraper::Node::Text(text_node) => {
                    let text = text_node.text.trim().to_string();
                    if !text.is_empty() {
                        elements.push(LyricElement::new_text(text));
                    }
                }
                _ => {}
            }
        }
    }

    fn extract_ruby_content(&self, ruby_element: scraper::ElementRef) -> (String, String) {
        let rb_selector = Selector::parse("span.rb").unwrap();
        let rt_selector = Selector::parse("span.rt").unwrap();

        let base_text = if let Some(rb_elem) = ruby_element.select(&rb_selector).next() {
            rb_elem.text().collect::<String>().trim().to_string()
        } else {
            String::new()
        };

        let ruby_text = if let Some(rt_elem) = ruby_element.select(&rt_selector).next() {
            rt_elem.text().collect::<String>().trim().to_string()
        } else {
            String::new()
        };

        (base_text, ruby_text)
    }

    pub async fn process_song(
        &self,
        title: &str,
        artist: Option<&str>,
    ) -> LyricsSearchResponse {
        let mut result = LyricsSearchResponse::new(title.to_string(), artist.map(|s| s.to_string()));

        if let Some(cached_entry) = self.cache.search().get(title, artist).await {
            info!("\n=== [SEARCH CACHE HIT] ===");
            info!("  Title: {}", title);
            info!("  Artist: {:?}", artist);
            info!("  Results: {}", cached_entry.search_results.len());
            info!("===================\n");

            result.search_results = cached_entry
                .search_results
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            result.matched = true;
            result.status = "select".to_string();
            result.found_title = cached_entry.found_title;
            result.found_artist = cached_entry.found_artist;
            result.lyrics_url = cached_entry.lyrics_url;
            result.from_cache = true;
            return result;
        }

        info!("\n=== [SEARCH CACHE MISS] ===");
        info!("  Title: {}", title);
        info!("  Artist: {:?}", artist);
        info!("  Fetching from UtaTen...");
        info!("===================\n");

        let search_results = self.search(title, artist).await;
        result.search_results = search_results.clone();

        if !search_results.is_empty() {
            result.matched = true;
            result.status = "select".to_string();

            let first_result = &search_results[0];
            result.found_title = first_result.title.clone();
            result.found_artist = first_result.artist.clone();
            result.lyrics_url = first_result.url.clone();

            let search_results_json: Vec<serde_json::Value> = search_results
                .iter()
                .filter_map(|r| serde_json::to_value(r).ok())
                .collect();

            self.cache.search().insert(
                title,
                artist,
                SearchResultEntry::new(
                    search_results_json,
                    result.found_title.clone(),
                    result.found_artist.clone(),
                    result.lyrics_url.clone(),
                ),
            ).await;
        } else {
            result.status = "not_found".to_string();
            result.error = Some("未找到匹配的歌词".to_string());
        }

        result
    }

    pub async fn select_result(
        &self,
        process_result: LyricsSearchResponse,
        index: usize,
    ) -> LyricsSearchResponse {
        let mut result = process_result.clone();

        if index >= result.search_results.len() {
            debug!(
                "select_result: index out of range, index={}, len={}",
                index,
                result.search_results.len()
            );
            result.status = "error".to_string();
            result.error = Some("无效的选择".to_string());
            return result;
        }

        let selected = &result.search_results[index];
        let lyrics_url = selected.url.clone();
        let found_title = selected.title.clone();
        let found_artist = selected.artist.clone();

        debug!("select_result: selected URL='{}'", lyrics_url);
        debug!("select_result: checking cache...");

        if let Some(cached_annotations) = self.cache.lyrics().get(&lyrics_url).await {
            info!("\n=== [CACHE HIT] ===");
            info!("  URL: {}", lyrics_url);
            info!("  Title: {}", found_title);
            info!("  Artist: {}", found_artist);
            info!("  Elements: {}", cached_annotations.len());
            info!("===================\n");

            result.ruby_annotations = cached_annotations;
            result.status = "success".to_string();
            result.found_title = found_title;
            result.found_artist = found_artist;
            result.lyrics_url = lyrics_url;
            result.selected_index = index as i32;
            return result;
        }

        info!("\n=== [CACHE MISS] ===");
        info!("  URL: {}", lyrics_url);
        info!("  Title: {}", found_title);
        info!("  Artist: {}", found_artist);
        info!("  Fetching from UtaTen...");
        info!("===================\n");

        if let Some(html) = self.get_lyrics_with_ruby(&lyrics_url).await {
            let annotations = self.extract_ruby_lyrics(&html);
            self.cache.lyrics().insert(lyrics_url.clone(), annotations.clone()).await;

            info!("\n=== [CACHE STORED] ===");
            info!("  URL: {}", lyrics_url);
            info!("  Title: {}", found_title);
            info!("  Artist: {}", found_artist);
            info!("  Elements: {}", annotations.len());
            info!("===================\n");

            result.ruby_annotations = annotations;
            result.status = "success".to_string();
            result.found_title = found_title;
            result.found_artist = found_artist;
            result.lyrics_url = lyrics_url;
            result.selected_index = index as i32;
        } else {
            result.status = "error".to_string();
            result.error = Some("无法获取歌词页面".to_string());
        }

        result
    }

    pub fn cache(&self) -> &CacheManager {
        &self.cache
    }
}

impl Default for UtaTenSearcher {
    fn default() -> Self {
        Self::new(CacheManager::new())
    }
}
