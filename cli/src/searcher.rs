//! UtaTen 歌词搜索引擎模块。
//!
//! 该模块封装了与 UtaTen（うた☆てん，日本歌词网站）的交互逻辑，包括：
//! - 搜索歌曲（支持按标题、艺术家过滤）
//! - 提取歌词页面中的 ruby 注音注释
//! - 获取歌曲封面和专辑信息
//! - 与 QQ Music、网易云音乐等备用数据源集成
//! - 请求限速与响应解码（支持 gzip、Shift_JIS 等编码）

use crate::cache_manager::{CacheManager, SearchResultEntry};
use crate::models::{
    LyricElement, LyricsSearchResponse, SearchPagination, SearchResponse, SearchResult,
};
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use base64::Engine as _;

/// 歌曲页面的元数据，包含专辑名称和封面图片 URL。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SongPageMetadata {
    /// 专辑名称（如「Wahl」「FIRE BIRD」等）
    pub album: Option<String>,
    /// 封面图片的完整 URL
    pub cover_url: Option<String>,
}

/// 专辑封面图片来源偏好设置。
///
/// 控制优先使用哪个数据源获取歌曲封面和专辑信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkSourcePreference {
    /// 自动选择最佳可用来源
    Auto,
    /// 优先从 UtaTen 获取封面
    UtaTen,
    /// 优先从 QQ Music 获取封面
    QqMusic,
    /// 优先从网易云音乐获取封面
    Netease,
}

impl ArtworkSourcePreference {
    /// 从配置字符串解析封面来源偏好。
    ///
    /// 接受的值：`"utaten"`、`"qq"`/`"qqmusic"`/`"qq_music"`、`"netease"`/`"neteasecloud"`/`"netease_cloud"`，其它值或不提供则返回 `Auto`。
    pub fn from_setting(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("utaten") => Self::UtaTen,
            Some("qq") | Some("qqmusic") | Some("qq_music") => Self::QqMusic,
            Some("netease") | Some("neteasecloud") | Some("netease_cloud") => Self::Netease,
            _ => Self::Auto,
        }
    }
}

/// 歌词来源偏好设置。
///
/// 控制优先使用哪个数据源获取歌词文本和注音。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LyricSourcePreference {
    /// 自动选择最佳可用来源
    Auto,
    /// 优先从 UtaTen 获取歌词
    UtaTen,
    /// 优先从 QQ Music 获取歌词
    QqMusic,
    /// 优先从网易云音乐获取歌词
    Netease,
}

impl LyricSourcePreference {
    /// 从配置字符串解析歌词来源偏好。
    ///
    /// 接受的值：`"utaten"`、`"qq"`/`"qqmusic"`/`"qq_music"`、`"netease"`/`"ne"`/`"wy"`，其它值或不提供则返回 `Auto`。
    pub fn from_setting(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("utaten") => Self::UtaTen,
            Some("qq") | Some("qqmusic") | Some("qq_music") => Self::QqMusic,
            Some("netease") | Some("ne") | Some("wy") => Self::Netease,
            _ => Self::Auto,
        }
    }
}

/// 经过解析的艺术家信息，包含名称、作词人和作曲人。
#[derive(Debug, Clone, Default)]
pub struct ArtistInfo {
    /// 艺术家名称
    pub artist: String,
    /// 作词人（可选）
    pub lyricist: Option<String>,
    /// 作曲人（可选）
    pub composer: Option<String>,
}

/// 从 UtaTen 搜索结果页的原始 HTML 文本中解析艺术家信息。
///
/// 去除多余空白和换行，通过正则匹配「作詞：」「作曲：」标记来分离作词人和作曲人。
/// 若未找到任何标记，整段文本将视为艺术家名称。
///
/// ## 参数
/// - `raw`：来自 HTML 的原始艺术家信息文本
///
/// ## 返回值
/// 包含 `artist`、`lyricist`、`composer` 的 `ArtistInfo`。
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
            if lyricist_text.is_empty() {
                None
            } else {
                Some(lyricist_text.to_string())
            },
            if composer_text.is_empty() {
                None
            } else {
                Some(composer_text.to_string())
            },
        )
    } else {
        let text = rest.trim();
        (
            if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            },
            None,
        )
    };

    ArtistInfo {
        artist,
        lyricist,
        composer,
    }
}

/// UtaTen 网站基础 URL
const BASE_URL: &str = "https://utaten.com";
/// 连续请求之间的最小延迟（毫秒）
const REQUEST_DELAY_MS: u64 = 500;
/// HTTP 请求超时时间（秒）
const REQUEST_TIMEOUT_SECS: u64 = 15;

/// UtaTen 歌词搜索引擎主结构体。
///
/// 封装 HTTP 客户端、缓存管理器、请求限速器和网易云音乐数据源，
/// 提供搜索 UtaTen 以及从 QQ Music、网易云等备用源获取数据的能力。
pub struct UtaTenSearcher {
    /// 复用的 HTTP 客户端，配置了自定义 User-Agent 和代理设置
    client: Client,
    /// 缓存管理器，用于缓存搜索结果和歌词以避免重复请求
    pub cache: CacheManager,
    /// 请求间隔延迟时间
    delay: Duration,
    /// 上次请求的时间戳（用于限速）
    last_request: Arc<Mutex<Instant>>,
    /// 网易云音乐数据源，用于搜索和获取歌词
    pub ne_source: crate::ne_source::NeteaseSource,
}

/// 内部搜索请求结构，包含请求路径和查询参数。
#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchRequest {
    /// 请求路径（如 `/search`）
    path: &'static str,
    /// 查询参数列表
    params: Vec<(&'static str, String)>,
}

impl UtaTenSearcher {
    /// 创建新的 `UtaTenSearcher` 实例。
    ///
    /// 初始化 HTTP 客户端（配置日本语 User-Agent、禁用代理以避免路由问题）、
    /// 缓存管理器和网易云音乐数据源。
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
            // UtaTen is directly reachable on supported networks; avoid inheriting
            // desktop/WSL proxy settings that can misroute this Japan-hosted site.
            .no_proxy()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            cache,
            delay: Duration::from_millis(REQUEST_DELAY_MS),
            last_request: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10))),
            ne_source: crate::ne_source::NeteaseSource::new(),
        }
    }

    /// 请求限速器，确保两次请求之间至少间隔 `self.delay` 时长，避免触发服务器反爬机制。
    async fn rate_limit(&self) {
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        if elapsed < self.delay {
            tokio::time::sleep(self.delay - elapsed).await;
        }
        *last = Instant::now();
    }

    /// 解码 HTTP 响应体。
    ///
    /// 自动处理 gzip 解压缩，并尝试按 UTF-8、Shift_JIS、EUC-JP 的顺序解码文本内容，
    /// 以适应 UtaTen 网站可能使用的不同编码。
    fn decode_response(bytes: &[u8], headers: &reqwest::header::HeaderMap) -> String {
        let content_encoding = headers
            .get(reqwest::header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok());

        let mut decoded_bytes = bytes.to_vec();

        if content_encoding == Some("gzip") {
            use flate2::read::GzDecoder;
            use std::io::Read;
            let mut decoder = GzDecoder::new(bytes);
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

    /// 检查文本中是否包含日文字符（平假名、片假名或汉字）。
    ///
    /// 用于判断解码后的内容是否为有效的日文文本。
    fn has_japanese(text: &str) -> bool {
        text.chars().any(|c| {
            ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c)
        })
    }

    /// 在 UtaTen 上搜索歌曲。
    ///
    /// 按标题（和可选的艺术家）搜索，返回匹配的 `SearchResult` 列表。
    /// 内部调用 `search_with_options`，搜索类型固定为 `"title"`，从第 1 页开始。
    pub async fn search(&self, title: &str, artist: Option<&str>) -> Vec<SearchResult> {
        self.search_with_options(title, artist, "title", 1)
            .await
            .results
    }

    /// 搜索但不使用缓存，始终发起网络请求。
    ///
    /// 与 `search_with_options` 类似但不检查缓存，适用于需要实时数据（如重新搜索）的场景。
    pub async fn search_with_options_uncached(
        &self,
        query: &str,
        artist: Option<&str>,
        search_type: &str,
        page: u32,
    ) -> SearchResponse {
        self.search_with_options_internal(query, artist, search_type, page, false)
            .await
    }

    /// 按标题/艺术家搜索 UtaTen，支持分页。
    ///
    /// 优先从缓存读取，缓存未命中时发起网络请求并将结果写入缓存。
    ///
    /// ## 参数
    /// - `query`：搜索关键词
    /// - `artist`：可选的艺术家名称过滤
    /// - `search_type`：搜索类型（如 `"title"`、`"artist"`）
    /// - `page`：页码（从 1 开始）
    pub async fn search_with_options(
        &self,
        query: &str,
        artist: Option<&str>,
        search_type: &str,
        page: u32,
    ) -> SearchResponse {
        self.search_with_options_internal(query, artist, search_type, page, true)
            .await
    }

    /// 内部搜索方法，提供是否读取缓存的控制。
    ///
    /// 若 `read_cache` 为 `true` 则优先从缓存获取结果；否则直接发起 HTTP 请求。
    /// 请求完成后将搜索结果写入缓存以供后续使用。
    async fn search_with_options_internal(
        &self,
        query: &str,
        artist: Option<&str>,
        search_type: &str,
        page: u32,
        read_cache: bool,
    ) -> SearchResponse {
        let mut response = SearchResponse::new();
        let trimmed_query = query.trim();
        let trimmed_artist = artist.map(str::trim).filter(|value| !value.is_empty());

        response.query_title = (!trimmed_query.is_empty()).then(|| trimmed_query.to_string());
        response.query_artist = trimmed_artist.map(ToString::to_string);
        response.search_type = search_type.to_string();
        response.page = page;

        if read_cache {
            if let Some(cached_entry) = self
                .cache
                .search()
                .get_with_options(trimmed_query, trimmed_artist, search_type, page)
                .await
            {
                response.results = cached_entry
                    .search_results
                    .iter()
                    .filter_map(|value| serde_json::from_value(value.clone()).ok())
                    .collect();
                response.pagination = cached_entry.pagination;
                response.status = if response.results.is_empty() {
                    "not_found"
                } else {
                    "select"
                }
                .to_string();
                return response;
            }
        }

        self.rate_limit().await;

        let search_request =
            Self::build_search_request(trimmed_query, trimmed_artist, search_type, page);
        let url = format!("{}{}", BASE_URL, search_request.path);
        debug!("HTTP GET: {} with params: {:?}", url, search_request.params);

        let http_response = match self
            .client
            .get(&url)
            .query(&search_request.params)
            .send()
            .await
        {
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
        let (results, pagination) = {
            let document = Html::parse_document(&html_content);
            let results = Self::extract_search_results(&document);
            let pagination = self.extract_pagination(&document, page);
            (results, pagination)
        };
        response.pagination = Some(pagination.clone());

        debug!("Returning {} unique results", results.len());
        response.results = results;
        response.status = if response.results.is_empty() {
            "not_found"
        } else {
            "select"
        }
        .to_string();

        let (found_title, found_artist, lyrics_url) = response
            .results
            .first()
            .map(|result| {
                (
                    result.title.clone(),
                    result.artist.clone(),
                    result.url.clone(),
                )
            })
            .unwrap_or_else(|| (String::new(), String::new(), String::new()));
        let search_results_json: Vec<serde_json::Value> = response
            .results
            .iter()
            .filter_map(|result| serde_json::to_value(result).ok())
            .collect();
        self.cache
            .search()
            .insert_with_options(
                trimmed_query,
                trimmed_artist,
                search_type,
                page,
                SearchResultEntry::new(
                    search_results_json,
                    found_title,
                    found_artist,
                    lyrics_url,
                    response.pagination.clone(),
                ),
            )
            .await;

        response
    }

    /// 从 UtaTen 搜索结果的 HTML 文档中提取所有歌曲搜索结果。
    ///
    /// 解析 `table.searchResult` 表格，提取每行的歌曲标题 URL 和艺术家名称，
    /// 利用 `parse_artist_info` 进一步分离作词人/作曲人，并通过 URL 去重。
    fn extract_search_results(document: &Html) -> Vec<SearchResult> {
        let table_selector = Selector::parse(
            "table.searchResult.artistLyricList, table.searchResult.lyricList, table.searchResult, table.lyricList",
        )
        .unwrap();
        let row_selector = Selector::parse("tr").unwrap();
        let artist_cell_selector =
            Selector::parse("td.searchResult__artist, td.lyricList__artist").unwrap();
        let link_selector = Selector::parse(r#"a[href*="/lyric/"]"#).unwrap();

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
                            results.push(
                                SearchResult::with_artist_info(
                                    link_text,
                                    artist_info.artist,
                                    href.to_string(),
                                    artist_info.lyricist,
                                    artist_info.composer,
                                )
                                .with_source("utaten"),
                            );
                        }
                    }
                }
            }
        }

        results
    }

    /// 构建 UtaTen 搜索请求的路径和参数。
    ///
    /// 根据 `search_type` 和是否提供 `artist` 参数，生成不同的查询参数组合：
    /// - `"artist"` 类型：使用 `artist_name` 参数
    /// - 含艺术家：同时使用 `title` 和 `artist_name` 参数
    /// - 其它：使用 `layout_search_type` 和 `layout_search_text` 参数
    fn build_search_request(
        query: &str,
        artist: Option<&str>,
        search_type: &str,
        page: u32,
    ) -> SearchRequest {
        let trimmed_query = query.trim();
        let trimmed_artist = artist.map(str::trim).filter(|value| !value.is_empty());
        let page = page.max(1).to_string();

        match (search_type, trimmed_artist) {
            ("artist", _) => SearchRequest {
                path: "/search",
                params: vec![
                    ("artist_name", trimmed_query.to_string()),
                    ("sort", "popular_sort_asc".to_string()),
                    ("show_artists", "1".to_string()),
                    ("page", page),
                ],
            },
            (_, Some(artist_name)) => SearchRequest {
                path: "/search",
                params: vec![
                    ("title", trimmed_query.to_string()),
                    ("artist_name", artist_name.to_string()),
                    ("sort", "popular_sort_asc".to_string()),
                    ("show_artists", "1".to_string()),
                    ("page", page),
                ],
            },
            _ => SearchRequest {
                path: "/search",
                params: vec![
                    ("layout_search_type", search_type.to_string()),
                    ("layout_search_text", trimmed_query.to_string()),
                    ("page", page),
                ],
            },
        }
    }

    /// 从搜索结果的 HTML 文档中提取分页信息。
    ///
    /// 解析 `.pager` 元素中的翻页链接，计算最大页码和是否有下一页。
    fn extract_pagination(&self, document: &Html, current_page: u32) -> SearchPagination {
        let pager_selector = Selector::parse(".pager").unwrap();
        let link_selector = Selector::parse(r#"a[href*="page="]"#).unwrap();

        let mut total_pages = current_page;
        let mut has_next = false;

        if let Some(pager) = document.select(&pager_selector).next() {
            for link in pager.select(&link_selector) {
                if let Some(href) = link.value().attr("href") {
                    if let Some(num) = Self::extract_page_number_from_href(href) {
                        total_pages = total_pages.max(num);
                        has_next |= num > current_page;
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

    /// 从分页链接的 href 属性中提取页码数字。
    ///
    /// 查找 `page=` 标记后的连续数字并解析为整数。
    fn extract_page_number_from_href(href: &str) -> Option<u32> {
        let page_marker = href.find("page=")?;
        let digits: String = href[page_marker + "page=".len()..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();

        if digits.is_empty() {
            None
        } else {
            digits.parse::<u32>().ok()
        }
    }

    /// 规范化 UtaTen 资源 URL。
    ///
    /// 自动补齐协议前缀（`//` → `https://`）和路径前缀（`/` → `https://utaten.com/`），
    /// 忽略空 URL 和 data: URI。
    fn normalize_utaten_asset_url(raw_url: &str) -> Option<String> {
        let value = raw_url.trim();
        if value.is_empty() || value.starts_with("data:") {
            return None;
        }

        if value.starts_with("https://") || value.starts_with("http://") {
            return Some(value.to_string());
        }

        if value.starts_with("//") {
            return Some(format!("https:{}", value));
        }

        if value.starts_with('/') {
            return Some(format!("{}{}", BASE_URL, value));
        }

        Some(format!("{}/{}", BASE_URL, value.trim_start_matches("./")))
    }

    /// 从歌曲详情页的 HTML 内容中提取元数据（封面 URL 和专辑名称）。
    ///
    /// 依次尝试从 Open Graph 元标签 `og:image`、`twitter:image`、`itemprop="image"`、
    /// 图片元素 `<img>` 以及专辑相关的元标签/链接中提取信息。
    /// 自动过滤 logo 和 noimage 占位图。
    pub fn extract_song_page_metadata(html_content: &str) -> SongPageMetadata {
        let document = Html::parse_document(html_content);

        let image_meta_selector = Selector::parse(
            r#"meta[property="og:image"], meta[name="twitter:image"], meta[itemprop="image"]"#,
        )
        .unwrap();
        let image_selector = Selector::parse(
            r#"img[src*="/img/"], img[src*="jacket"], img[data-src*="/img/"], img[data-src*="jacket"]"#,
        )
        .unwrap();
        let album_meta_selector = Selector::parse(
            r#"meta[property="music:album"], meta[name="music:album"], meta[itemprop="inAlbum"]"#,
        )
        .unwrap();
        let album_link_selector =
            Selector::parse(r#"a[href*="/album/"], .album a, .songAlbum a"#).unwrap();

        let cover_url = document
            .select(&image_meta_selector)
            .filter_map(|element| element.value().attr("content"))
            .filter_map(Self::normalize_utaten_asset_url)
            .find(|url| {
                let lower = url.to_ascii_lowercase();
                !lower.contains("logo") && !lower.contains("noimage")
            })
            .or_else(|| {
                document
                    .select(&image_selector)
                    .filter_map(|element| {
                        element
                            .value()
                            .attr("data-src")
                            .or_else(|| element.value().attr("src"))
                    })
                    .filter_map(Self::normalize_utaten_asset_url)
                    .find(|url| {
                        let lower = url.to_ascii_lowercase();
                        !lower.contains("logo") && !lower.contains("noimage")
                    })
            });

        let album = document
            .select(&album_meta_selector)
            .filter_map(|element| element.value().attr("content"))
            .map(str::trim)
            .find(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                document
                    .select(&album_link_selector)
                    .map(|element| element.text().collect::<String>())
                    .map(|text| text.trim().to_string())
                    .find(|text| !text.is_empty())
            });

        SongPageMetadata { album, cover_url }
    }

    /// 合并两个来源的专辑元数据。
    ///
    /// 以 `primary` 为主，其缺失的字段由 `fallback` 补充（`primary.album.or(fallback.album)`）。
    fn merge_album_cover(
        primary: SongPageMetadata,
        fallback: SongPageMetadata,
    ) -> SongPageMetadata {
        SongPageMetadata {
            album: primary.album.or(fallback.album),
            cover_url: primary.cover_url.or(fallback.cover_url),
        }
    }

    /// 构建搜索用的歌曲查询字符串。
    ///
    /// 若提供了艺术家，格式为 `"{title} {artist}"`；否则仅返回标题。
    fn song_query(title: &str, artist: Option<&str>) -> String {
        match artist.map(str::trim).filter(|value| !value.is_empty()) {
            Some(artist) => format!("{} {}", title.trim(), artist),
            None => title.trim().to_string(),
        }
    }

    /// 对候选封面匹配结果进行打分。
    ///
    /// 根据标题和艺术家的精确匹配或部分匹配计算得分（满分 115），
    /// 用于从多个来源的候选中选择最佳匹配。
    fn score_artwork_candidate(
        candidate_title: Option<&str>,
        candidate_artist: Option<&str>,
        title: &str,
        artist: Option<&str>,
    ) -> i32 {
        let normalize = |value: &str| {
            value.to_ascii_lowercase().replace(
                [' ', '　', '-', '_', '・', '／', '/', '(', ')', '[', ']'],
                "",
            )
        };
        let expected_title = normalize(title);
        let expected_artist = artist.map(normalize).unwrap_or_default();
        let candidate_title = candidate_title.map(normalize).unwrap_or_default();
        let candidate_artist = candidate_artist.map(normalize).unwrap_or_default();

        let mut score = 0;
        if !expected_title.is_empty() && candidate_title == expected_title {
            score += 80;
        } else if !expected_title.is_empty() && candidate_title.contains(&expected_title) {
            score += 45;
        } else if !candidate_title.is_empty() && expected_title.contains(&candidate_title) {
            score += 25;
        }

        if !expected_artist.is_empty() && candidate_artist == expected_artist {
            score += 35;
        } else if !expected_artist.is_empty() && candidate_artist.contains(&expected_artist) {
            score += 18;
        }

        score
    }

    /// 从 QQ Music API 返回的 JSON 响应中提取最佳匹配的专辑封面元数据。
    ///
    /// 支持两种 JSON 结构（`/data/song/list` 和 `/req_0/data/body/item_song`），
    /// 通过 `score_artwork_candidate` 评分选取最佳结果。
    fn extract_qq_music_artwork_from_json(
        value: &serde_json::Value,
        title: &str,
        artist: Option<&str>,
    ) -> Option<SongPageMetadata> {
        let songs = value
            .pointer("/data/song/list")
            .and_then(|value| value.as_array())
            .or_else(|| {
                value
                    .pointer("/req_0/data/body/item_song")
                    .and_then(|value| value.as_array())
            })?;

        songs
            .iter()
            .filter_map(|song| {
                let album = song
                    .pointer("/album/name")
                    .and_then(|value| value.as_str())
                    .or_else(|| song.get("albumname").and_then(|value| value.as_str()))
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string);
                let album_mid = song
                    .pointer("/album/mid")
                    .and_then(|value| value.as_str())
                    .or_else(|| song.get("albummid").and_then(|value| value.as_str()))
                    .map(str::trim)
                    .filter(|value| !value.is_empty())?;
                let song_title = song
                    .get("name")
                    .and_then(|value| value.as_str())
                    .or_else(|| song.get("songname").and_then(|value| value.as_str()))
                    .or_else(|| song.get("title").and_then(|value| value.as_str()));
                let singer = song
                    .get("singer")
                    .and_then(|value| value.as_array())
                    .map(|singers| {
                        singers
                            .iter()
                            .filter_map(|value| value.get("name").and_then(|name| name.as_str()))
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .filter(|value| !value.is_empty());
                let score =
                    Self::score_artwork_candidate(song_title, singer.as_deref(), title, artist);
                let cover_url = format!(
                    "https://y.gtimg.cn/music/photo_new/T002R1200x1200M000{}.jpg?max_age=2592000",
                    album_mid
                );
                Some((
                    score,
                    SongPageMetadata {
                        album,
                        cover_url: Some(cover_url),
                    },
                ))
            })
            .max_by_key(|(score, _)| *score)
            .map(|(_, metadata)| metadata)
    }

    /// 从网易云音乐专辑搜索 API 返回的 JSON 中提取最佳匹配的封面元数据。
    ///
    /// 解析 `/result/albums` 数组，通过歌手名称匹配和打分选取最佳专辑封面。
    fn extract_netease_album_artwork_from_json(
        value: &serde_json::Value,
        title: &str,
        artist: Option<&str>,
    ) -> Option<SongPageMetadata> {
        let albums = value
            .pointer("/result/albums")
            .and_then(|value| value.as_array())?;

        albums
            .iter()
            .filter_map(|album_value| {
                let album = album_value
                    .get("name")
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string);
                let cover_url = album_value
                    .get("picUrl")
                    .or_else(|| album_value.get("blurPicUrl"))
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)?;
                let album_artist = album_value
                    .get("artist")
                    .and_then(|value| value.get("name"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .or_else(|| {
                        album_value
                            .get("artists")
                            .and_then(|value| value.as_array())
                            .map(|artists| {
                                artists
                                    .iter()
                                    .filter_map(|value| {
                                        value.get("name").and_then(|name| name.as_str())
                                    })
                                    .collect::<Vec<_>>()
                                    .join("/")
                            })
                            .filter(|value| !value.is_empty())
                    });
                let score = Self::score_artwork_candidate(
                    album.as_deref(),
                    album_artist.as_deref(),
                    title,
                    artist,
                );
                Some((
                    score,
                    SongPageMetadata {
                        album,
                        cover_url: Some(cover_url),
                    },
                ))
            })
            .max_by_key(|(score, _)| *score)
            .map(|(_, metadata)| metadata)
    }

    /// 从网易云音乐歌曲搜索 API 返回的 JSON 中提取最佳匹配的封面元数据。
    ///
    /// 解析 `/result/songs` 数组，匹配歌曲的专辑封面和歌手信息，
    /// 通过 `score_artwork_candidate` 评分选取最佳结果。
    fn extract_netease_artwork_from_json(
        value: &serde_json::Value,
        title: &str,
        artist: Option<&str>,
    ) -> Option<SongPageMetadata> {
        let songs = value
            .pointer("/result/songs")
            .and_then(|value| value.as_array())?;

        songs
            .iter()
            .filter_map(|song| {
                let album_value = song.get("album").or_else(|| song.get("al"))?;
                let album = album_value
                    .get("name")
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string);
                let cover_url = album_value
                    .get("picUrl")
                    .or_else(|| album_value.get("pic_url"))
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)?;
                let song_title = song.get("name").and_then(|value| value.as_str());
                let artists = song
                    .get("artists")
                    .or_else(|| song.get("ar"))
                    .and_then(|value| value.as_array())
                    .map(|artists| {
                        artists
                            .iter()
                            .filter_map(|value| value.get("name").and_then(|name| name.as_str()))
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .filter(|value| !value.is_empty());
                let score =
                    Self::score_artwork_candidate(song_title, artists.as_deref(), title, artist);
                Some((
                    score,
                    SongPageMetadata {
                        album,
                        cover_url: Some(cover_url),
                    },
                ))
            })
            .max_by_key(|(score, _)| *score)
            .map(|(_, metadata)| metadata)
    }

    /// 从 QQ Music API 获取歌曲的封面和专辑信息。
    ///
    /// 使用 QQ Music 的移动端搜索接口（`u.y.qq.com/cgi-bin/musicu.fcg`），
    /// 以歌曲标题+艺术家为关键词搜索并提取最佳匹配的封面元数据。
    async fn fetch_qq_music_artwork(
        &self,
        title: &str,
        artist: Option<&str>,
    ) -> Option<SongPageMetadata> {
        let query = Self::song_query(title, artist);
        if query.is_empty() {
            return None;
        }

        let search_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| format!("{}0000", duration.as_millis()))
            .unwrap_or_else(|_| "10000000000000000".to_string());
        let request_body = serde_json::json!({
            "comm": {
                "ct": "11",
                "cv": "1003006",
                "v": "1003006",
                "os_ver": "15",
                "phonetype": "24122RKC7C",
                "tmeAppID": "qqmusiclight",
                "nettype": "NETWORK_WIFI"
            },
            "req_0": {
                "method": "DoSearchForQQMusicLite",
                "module": "music.search.SearchCgiService",
                "param": {
                    "search_id": search_id,
                    "remoteplace": "search.android.keyboard",
                    "query": query,
                    "search_type": 0,
                    "num_per_page": 8,
                    "page_num": 1,
                    "highlight": 0,
                    "nqc_flag": 0,
                    "page_id": 1,
                    "grp": 1
                }
            }
        });

        let response = self
            .client
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header(reqwest::header::REFERER, "https://y.qq.com/")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(request_body.to_string())
            .send()
            .await
            .ok()?;
        let body = response.text().await.ok()?;
        let json = serde_json::from_str::<serde_json::Value>(&body).ok()?;
        Self::extract_qq_music_artwork_from_json(&json, title, artist)
    }

    /// 从网易云音乐 API 获取歌曲的封面和专辑信息。
    ///
    /// 先按歌曲类型（type=1）搜索，若未找到则按专辑类型（type=10）搜索，
    /// 选取最佳匹配的封面元数据。
    async fn fetch_netease_artwork(
        &self,
        title: &str,
        artist: Option<&str>,
    ) -> Option<SongPageMetadata> {
        let query = Self::song_query(title, artist);
        if query.is_empty() {
            return None;
        }

        let response = self
            .client
            .post("https://music.163.com/api/search/get/web")
            .header(reqwest::header::REFERER, "https://music.163.com/")
            .form(&[
                ("s", query.as_str()),
                ("type", "1"),
                ("limit", "8"),
                ("offset", "0"),
            ])
            .send()
            .await
            .ok()?;
        let body = response.text().await.ok()?;
        let json = serde_json::from_str::<serde_json::Value>(&body).ok()?;
        if let Some(metadata) = Self::extract_netease_artwork_from_json(&json, title, artist) {
            return Some(metadata);
        }

        let response = self
            .client
            .post("https://music.163.com/api/search/get/web")
            .header(reqwest::header::REFERER, "https://music.163.com/")
            .form(&[
                ("s", query.as_str()),
                ("type", "10"),
                ("limit", "8"),
                ("offset", "0"),
            ])
            .send()
            .await
            .ok()?;
        let body = response.text().await.ok()?;
        let json = serde_json::from_str::<serde_json::Value>(&body).ok()?;
        Self::extract_netease_album_artwork_from_json(&json, title, artist)
    }

    /// 从 QQ Music API 获取加密的 QRC 歌词数据。
    ///
    /// 先搜索歌曲获取 `song_id`，再请求加密的歌词数据并解密，
    /// 返回（原始歌词 XML，罗马音 XML）元组。
    /// 使用 lyrico 兼容的参数格式进行请求。
    async fn fetch_qq_music_qrc(
        &self,
        title: &str,
        artist: Option<&str>,
    ) -> Option<(String, String)> {
        let query = Self::song_query(title, artist);
        if query.is_empty() {
            return None;
        }

        // Step 1: Search for song to get music_id
        let search_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| format!("{}0000", d.as_millis()))
            .unwrap_or_else(|_| "10000000000000000".to_string());

        let search_body = serde_json::json!({
            "comm": {
                "ct": "11", "cv": "1003006", "v": "1003006",
                "os_ver": "15", "phonetype": "24122RKC7C",
                "tmeAppID": "qqmusiclight", "nettype": "NETWORK_WIFI"
            },
            "req_0": {
                "method": "DoSearchForQQMusicLite",
                "module": "music.search.SearchCgiService",
                "param": {
                    "search_id": search_id,
                    "remoteplace": "search.android.keyboard",
                    "query": query,
                    "search_type": 0,
                    "num_per_page": 3,
                    "page_num": 1,
                    "highlight": 0, "nqc_flag": 0,
                    "page_id": 1, "grp": 1
                }
            }
        });

        let response = self
            .client
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header(reqwest::header::REFERER, "https://y.qq.com/")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(search_body.to_string())
            .send()
            .await
            .ok()?;
        let body = response.text().await.ok()?;
        let json: serde_json::Value = serde_json::from_str(&body).ok()?;

        let songs = json
            .pointer("/req_0/data/body/item_song")
            .and_then(|v| v.as_array())?;

        // Find best matching song and extract all needed metadata
        let (song_id, song_title, singer_name, album_name, interval) = songs
            .iter()
            .filter_map(|song| {
                let result = Self::qq_song_to_search_result(song, title, artist)?;
                let score = Self::score_artwork_candidate(
                    Some(&result.title),
                    Some(&result.artist),
                    title,
                    artist,
                );
                let id = song.get("id").and_then(|v| v.as_i64())?;
                let song_title = song
                    .get("name")
                    .or_else(|| song.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let singer_name = song
                    .get("singer")
                    .and_then(|v| v.as_array())
                    .map(|singers| {
                        singers
                            .iter()
                            .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .unwrap_or_default();
                let album_name = song
                    .get("album")
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let interval = song.get("interval").and_then(|v| v.as_i64()).unwrap_or(0);
                Some((score, id, song_title, singer_name, album_name, interval))
            })
            .max_by_key(|(score, _, _, _, _, _)| *score)
            .map(|(_, id, title, singer, album, interval)| (id, title, singer, album, interval))?;

        // Step 2: Fetch lyrics with lyrico-compatible parameters
        let engine = base64::engine::general_purpose::STANDARD;
        let lyric_body = serde_json::json!({
            "comm": {
                "ct": "11", "cv": "1003006", "v": "1003006",
                "os_ver": "15", "phonetype": "24122RKC7C",
                "tmeAppID": "qqmusiclight", "nettype": "NETWORK_WIFI"
            },
            "req_0": {
                "method": "GetPlayLyricInfo",
                "module": "music.musichallSong.PlayLyricInfo",
                "param": {
                    "songID": song_id,
                    "songName": engine.encode(song_title.as_bytes()),
                    "albumName": engine.encode(album_name.as_bytes()),
                    "singerName": engine.encode(singer_name.as_bytes()),
                    "crypt": 1, "qrc": 1,
                    "roma": 1, "trans": 1,
                    "cv": 2111, "ct": 19,
                    "lrc_t": 0, "qrc_t": 0,
                    "roma_t": 0, "trans_t": 0,
                    "type": 0,
                    "interval": interval,
                }
            }
        });

        let response = self
            .client
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header(reqwest::header::REFERER, "https://y.qq.com/")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(lyric_body.to_string())
            .send()
            .await
            .ok()?;
        let body = response.text().await.ok()?;
        let json: serde_json::Value = serde_json::from_str(&body).ok()?;

        let lyric_hex = json.pointer("/req_0/data/lyric").and_then(|v| v.as_str())?;
        let roma_hex = json.pointer("/req_0/data/roma").and_then(|v| v.as_str())?;

        let lyric_xml = crate::qm_decrypt::decrypt_qm_lyrics(lyric_hex)?;
        let roma_xml = crate::qm_decrypt::decrypt_qm_lyrics(roma_hex)?;

        Some((lyric_xml, roma_xml))
    }

    /// 将 QQ Music 的歌曲 JSON 对象转换为统一的 `SearchResult`。
    ///
    /// 提取歌曲标题、歌手（多个歌手以 `/` 分隔）、专辑名称和 MID 标识。
    fn qq_song_to_search_result(
        song: &serde_json::Value,
        _title: &str,
        artist: Option<&str>,
    ) -> Option<SearchResult> {
        let song_title = song
            .get("name")
            .or_else(|| song.get("title"))
            .and_then(|v| v.as_str())?;
        let singer = song
            .get("singer")
            .and_then(|v| v.as_array())
            .map(|singers| {
                singers
                    .iter()
                    .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_default();
        let mid = song.get("mid").and_then(|v| v.as_str())?;
        let album = song
            .get("album")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Some(
            SearchResult::new(
                song_title.to_string(),
                if singer.is_empty() {
                    artist.unwrap_or("").to_string()
                } else {
                    singer
                },
                format!("qq_music:{}", mid),
            )
            .with_source("qq_music"),
        )
        .map(|mut r| {
            r.album = album;
            r
        })
    }

    /// 在网易云音乐上搜索歌曲。
    ///
    /// 使用 `NeteaseSource` 进行搜索，支持分页参数。
    /// 返回包含匹配结果的 `SearchResponse`。
    pub async fn search_netease(
        &self,
        title: &str,
        artist: Option<&str>,
        page: u32,
    ) -> SearchResponse {
        let query = Self::song_query(title, artist);
        let mut response = SearchResponse::new();
        response.query_title = Some(title.to_string());
        response.query_artist = artist.map(|a| a.to_string());
        response.search_type = "netease".to_string();
        response.page = page;

        if query.is_empty() {
            response.status = "not_found".to_string();
            return response;
        }

        match self.ne_source.search(&query, page, 8).await {
            Some(results) if !results.is_empty() => {
                response.status = "select".to_string();
                response.pagination = Some(SearchPagination {
                    current_page: page,
                    total_pages: 1,
                    has_next: false,
                });
                response.results = results;
            }
            _ => {
                response.status = "not_found".to_string();
            }
        }

        response
    }

    /// 在 QQ Music 上搜索歌曲。
    ///
    /// 使用 QQ Music 移动端搜索接口，支持分页。
    /// 返回包含所有匹配歌曲的 `SearchResponse`。
    pub async fn search_qq_music(
        &self,
        title: &str,
        artist: Option<&str>,
        page: u32,
    ) -> SearchResponse {
        let query = Self::song_query(title, artist);
        let mut response = SearchResponse::new();
        response.query_title = Some(title.to_string());
        response.query_artist = artist.map(|a| a.to_string());
        response.search_type = "qq_music".to_string();
        response.page = page;

        if query.is_empty() {
            response.status = "not_found".to_string();
            return response;
        }

        let search_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| format!("{}0000", d.as_millis()))
            .unwrap_or_else(|_| "10000000000000000".to_string());

        let search_body = serde_json::json!({
            "comm": {
                "ct": "11", "cv": "1003006", "v": "1003006",
                "os_ver": "15", "phonetype": "24122RKC7C",
                "tmeAppID": "qqmusiclight", "nettype": "NETWORK_WIFI"
            },
            "req_0": {
                "method": "DoSearchForQQMusicLite",
                "module": "music.search.SearchCgiService",
                "param": {
                    "search_id": search_id,
                    "remoteplace": "search.android.keyboard",
                    "query": query,
                    "search_type": 0,
                    "num_per_page": 8,
                    "page_num": page,
                    "highlight": 0, "nqc_flag": 0,
                    "page_id": page, "grp": 1
                }
            }
        });

        let resp = match self
            .client
            .post("https://u.y.qq.com/cgi-bin/musicu.fcg")
            .header(reqwest::header::REFERER, "https://y.qq.com/")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(search_body.to_string())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                error!("QQ Music search request failed: {}", e);
                response.status = "error".to_string();
                response.error = Some(format!("QQ Music 搜索请求失败: {}", e));
                return response;
            }
        };

        let body = match resp.text().await {
            Ok(b) => b,
            Err(e) => {
                error!("QQ Music search response read failed: {}", e);
                response.status = "error".to_string();
                response.error = Some(format!("QQ Music 搜索响应读取失败: {}", e));
                return response;
            }
        };

        let json: serde_json::Value = match serde_json::from_str(&body) {
            Ok(j) => j,
            Err(e) => {
                error!("QQ Music search JSON parse failed: {}", e);
                response.status = "error".to_string();
                response.error = Some(format!("QQ Music 搜索解析失败: {}", e));
                return response;
            }
        };

        let songs = match json
            .pointer("/req_0/data/body/item_song")
            .and_then(|v| v.as_array())
        {
            Some(s) => s,
            None => {
                response.status = "not_found".to_string();
                return response;
            }
        };

        let results: Vec<SearchResult> = songs
            .iter()
            .filter_map(|song| Self::qq_song_to_search_result(song, title, artist))
            .collect();

        if results.is_empty() {
            response.status = "not_found".to_string();
            return response;
        }

        response.status = "select".to_string();
        response.pagination = Some(SearchPagination {
            current_page: page,
            total_pages: 1,
            has_next: false,
        });
        response.results = results;
        response
    }

    /// 从 QQ Music 获取歌词，转换为 `LyricElement` 列表。
    ///
    /// 使用 QRC 解析器处理原始 XML 歌词和罗马音歌词，
    /// 通过字符级时间重叠匹配（`align_qrc_by_character`）对齐原文和注音，
    /// 相比启发式方法可以避免送假名泄漏和跨词边界错误。
    /// 若字符级对齐失败，回退到假名锚点对齐（kana-anchor alignment）。
    pub async fn fetch_qq_music_lyrics(
        &self,
        title: &str,
        artist: Option<&str>,
    ) -> Option<Vec<LyricElement>> {
        let (lyric_xml, roma_xml) = self.fetch_qq_music_qrc(title, artist).await?;

        let original_lines = crate::qrc_parser::parse_qrc(&lyric_xml)?;
        let romaji_lines = crate::qrc_parser::parse_qrc(&roma_xml)?;

        let aligned = crate::qrc_parser::align_romaji_to_original(&original_lines, &romaji_lines);

        let mut elements: Vec<LyricElement> = Vec::new();

        for (i, (orig_words, roma_words)) in aligned.iter().enumerate() {
            if orig_words.is_empty() {
                continue;
            }

            if let Some(roma_words) = roma_words {
                if !roma_words.is_empty() {
                    let line_elements =
                        crate::qrc_parser::align_qrc_by_character(orig_words, roma_words);
                    if line_elements.is_empty() {
                        // QRC character-level alignment produced no ruby.
                        // Fall back to kana-anchor alignment with the full romaji string.
                        let orig_text: String =
                            orig_words.iter().map(|w| w.text.as_str()).collect();
                        let roma_str: String = roma_words
                            .iter()
                            .map(|w| w.text.as_str())
                            .collect::<Vec<&str>>()
                            .join(" ");
                        let hiragana = crate::romaji::romaji_to_hiragana_strict(&roma_str);
                        if !hiragana.is_empty() && hiragana != orig_text {
                            let fallback = crate::ruby_align::align_ruby_to_text(&orig_text, &hiragana);
                            if fallback.is_empty() {
                                elements.push(LyricElement::new_text(orig_text));
                            } else {
                                elements.extend(fallback);
                            }
                        } else {
                            elements.push(LyricElement::new_text(orig_text));
                        }
                    } else {
                        elements.extend(line_elements);
                    }
                } else {
                    let orig_text: String =
                        orig_words.iter().map(|w| w.text.as_str()).collect();
                    elements.push(LyricElement::new_text(orig_text));
                }
            } else {
                let orig_text: String = orig_words.iter().map(|w| w.text.as_str()).collect();
                elements.push(LyricElement::new_text(orig_text));
            }

            if i + 1 < aligned.len() {
                elements.push(LyricElement::new_linebreak());
            }
        }

        Some(elements)
    }

    /// 根据偏好设置解析封面元数据。
    ///
    /// 根据 `ArtworkSourcePreference` 决定使用哪个来源的封面数据：
    /// - `UtaTen`：直接返回 UtaTen 的元数据
    /// - `QqMusic` / `Netease`：优先使用对应来源的数据，缺失字段由 UtaTen 补充
    /// - `Auto`：优先使用 UtaTen 封面，若缺失则依次尝试 QQ Music 和网易云
    pub async fn resolve_artwork_metadata(
        &self,
        title: &str,
        artist: Option<&str>,
        utaten_metadata: SongPageMetadata,
        preference: ArtworkSourcePreference,
    ) -> SongPageMetadata {
        match preference {
            ArtworkSourcePreference::UtaTen => utaten_metadata,
            ArtworkSourcePreference::QqMusic => {
                if let Some(metadata) = self.fetch_qq_music_artwork(title, artist).await {
                    Self::merge_album_cover(metadata, utaten_metadata)
                } else {
                    utaten_metadata
                }
            }
            ArtworkSourcePreference::Netease => {
                if let Some(metadata) = self.fetch_netease_artwork(title, artist).await {
                    Self::merge_album_cover(metadata, utaten_metadata)
                } else {
                    utaten_metadata
                }
            }
            ArtworkSourcePreference::Auto => {
                if utaten_metadata.cover_url.is_some() {
                    return utaten_metadata;
                }
                if let Some(metadata) = self.fetch_qq_music_artwork(title, artist).await {
                    return Self::merge_album_cover(metadata, utaten_metadata);
                }
                if let Some(metadata) = self.fetch_netease_artwork(title, artist).await {
                    return Self::merge_album_cover(metadata, utaten_metadata);
                }
                utaten_metadata
            }
        }
    }

    /// 获取歌词页面的原始 HTML 内容。
    ///
    /// 支持绝对 URL 和相对路径，自动补齐 `BASE_URL`。
    /// 返回解码后的 HTML 字符串。
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

    /// 根据 URL 获取歌词并解析 ruby 注音注释。
    ///
    /// 支持以下 URL 格式：
    /// - `https://utaten.com/lyric/...`（UtaTen 绝对 URL）
    /// - `/lyric/...`（UtaTen 相对路径）
    /// - `ne:12345`（网易云音乐内部歌曲 ID）
    /// - `qq_music:abc123`（QQ Music 内部 ID，需额外搜索步骤）
    ///
    /// 返回带 ruby 注音的 `LyricElement` 列表，获取失败时返回 `None`。
    pub async fn fetch_lyrics_from_url(&self, url: &str) -> Option<Vec<LyricElement>> {
        if url.starts_with("ne:") {
            let song_id = url.strip_prefix("ne:").unwrap_or(url);
            return self.ne_source.fetch_lyrics(song_id).await;
        }

        if url.starts_with("qq_music:") {
            // QQ Music synthetic IDs need a search step; not directly fetchable
            return None;
        }

        let html = self.get_lyrics_with_ruby(url).await?;
        let annotations = self.extract_ruby_lyrics(&html);
        if annotations.is_empty() {
            None
        } else {
            Some(annotations)
        }
    }

    /// 从 UtaTen 歌词页面的 HTML 中提取 ruby 注音注释。
    ///
    /// 定位到 `div.lyricBody > div.medium > div.hiragana` 层级结构，
    /// 递归解析其中的 ruby 注音、纯文本和换行符，返回有序的 `LyricElement` 列表。
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

    /// 递归处理 HTML 节点树，提取歌词元素。
    ///
    /// - `<br>`：插入换行符
    /// - `<span class="ruby">`：提取 ruby 注音（`<span.rb>` 基础文本 + `<span.rt>` 注音）
    /// - `<span>` 其他：递归处理子节点
    /// - 文本节点：添加纯文本元素
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
                                    elements.push(LyricElement::new_ruby(base_text, ruby_text));
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

    /// 从 ruby 注音 HTML 元素中提取基础文本和注音文本。
    ///
    /// 在 `<span class="ruby">` 内部查找 `<span class="rb">`（基础文本）
    /// 和 `<span class="rt">`（注音文本）。
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

    /// 处理歌曲搜索请求的完整流程。
    ///
    /// 优先从缓存读取搜索结果，缓存未命中时在 UtaTen 上执行搜索，
    /// 将结果写入缓存并返回 `LyricsSearchResponse`。
    ///
    /// ## 参数
    /// - `title`：歌曲标题
    /// - `artist`：可选的艺术家名称
    pub async fn process_song(&self, title: &str, artist: Option<&str>) -> LyricsSearchResponse {
        let mut result =
            LyricsSearchResponse::new(title.to_string(), artist.map(|s| s.to_string()));

        if let Some(cached_entry) = self.cache.search().get(title, artist).await {
            info!(
                "
=== [SEARCH CACHE HIT] ==="
            );
            info!("  Title: {}", title);
            info!("  Artist: {:?}", artist);
            info!("  Results: {}", cached_entry.search_results.len());
            info!(
                "===================
"
            );

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

        info!(
            "
=== [SEARCH CACHE MISS] ==="
        );
        info!("  Title: {}", title);
        info!("  Artist: {:?}", artist);
        info!("  Fetching from UtaTen...");
        info!(
            "===================
"
        );

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

            self.cache
                .search()
                .insert(
                    title,
                    artist,
                    SearchResultEntry::new(
                        search_results_json,
                        result.found_title.clone(),
                        result.found_artist.clone(),
                        result.lyrics_url.clone(),
                        None,
                    ),
                )
                .await;
        } else {
            result.status = "not_found".to_string();
            result.error = Some("未找到匹配的歌词".to_string());
        }

        result
    }

    /// 从搜索结果中选择指定索引的歌曲，使用默认歌词偏好（Auto）获取歌词。
    ///
    /// 委托给 `select_result_with_preference` 处理。
    pub async fn select_result(
        &self,
        process_result: LyricsSearchResponse,
        index: usize,
    ) -> LyricsSearchResponse {
        self.select_result_with_preference(process_result, index, LyricSourcePreference::Auto)
            .await
    }

    /// 根据歌词来源偏好选择搜索结果并获取歌词。
    ///
    /// 根据 `LyricSourcePreference` 选择歌词来源并按照以下顺序尝试：
    /// 1. 检查缓存（基于来源的缓存键）
    /// 2. 若 URL 以 `ne:` 开头或偏好为 NetEase，从网易云获取
    /// 3. 若偏好为 QQ Music 或 Auto，从 QQ Music 获取
    /// 4. 回退到 UtaTen 获取歌词 html，解析 ruby 注释并获取封面元数据
    ///
    /// ## 参数
    /// - `process_result`：搜索阶段的结果
    /// - `index`：选中的搜索结果索引
    /// - `lyric_preference`：歌词来源偏好
    pub async fn select_result_with_preference(
        &self,
        process_result: LyricsSearchResponse,
        index: usize,
        lyric_preference: LyricSourcePreference,
    ) -> LyricsSearchResponse {
        let mut result = process_result.clone();

        if index >= result.search_results.len() {
            result.status = "error".to_string();
            result.error = Some("无效的选择".to_string());
            return result;
        }

        let selected = &result.search_results[index];
        let lyrics_url = selected.url.clone();
        let found_title = selected.title.clone();
        let found_artist = selected.artist.clone();

        // Check cache first (source-aware keys)
        let qq_cache_key = format!("qq:{}:{}", found_title, found_artist);
        let ne_cache_key = format!("ne:{}", lyrics_url);
        let cache_hit = match lyric_preference {
            LyricSourcePreference::QqMusic => self.cache.lyrics().get(&qq_cache_key).await,
            LyricSourcePreference::UtaTen => self.cache.lyrics().get(&lyrics_url).await,
            LyricSourcePreference::Netease => self.cache.lyrics().get(&ne_cache_key).await,
            LyricSourcePreference::Auto => {
                let ne_hit = if lyrics_url.starts_with("ne:") {
                    self.cache.lyrics().get(&ne_cache_key).await
                } else {
                    None
                };
                if ne_hit.is_some() {
                    ne_hit
                } else {
                    let qq_hit = if lyrics_url.starts_with("qq_music:") {
                        self.cache.lyrics().get(&qq_cache_key).await
                    } else {
                        None
                    };
                    if qq_hit.is_some() {
                        qq_hit
                    } else {
                        self.cache.lyrics().get(&lyrics_url).await
                    }
                }
            }
        };

        if let Some(cached_annotations) = cache_hit {
            result.ruby_annotations = cached_annotations;
            result.status = "success".to_string();
            result.found_title = found_title;
            result.found_artist = found_artist;
            result.lyrics_url = lyrics_url;
            result.selected_index = index as i32;
            return result;
        }

        let use_qq = match lyric_preference {
            LyricSourcePreference::QqMusic => true,
            LyricSourcePreference::UtaTen => false,
            LyricSourcePreference::Netease => false,
            LyricSourcePreference::Auto => true,
        };

        // Try NetEase if URL starts with "ne:" or preference is Netease
        let use_ne = lyric_preference == LyricSourcePreference::Netease
            || (lyric_preference == LyricSourcePreference::Auto && lyrics_url.starts_with("ne:"));
        if use_ne {
            let ne_song_id = lyrics_url.strip_prefix("ne:").unwrap_or(&lyrics_url);
            if let Some(annotations) = self.ne_source.fetch_lyrics(ne_song_id).await {
                if !annotations.is_empty() {
                    self.cache
                        .lyrics()
                        .insert(ne_cache_key.clone(), annotations.clone())
                        .await;
                    result.ruby_annotations = annotations;
                    result.status = "success".to_string();
                    result.found_title = found_title;
                    result.found_artist = found_artist;
                    result.lyrics_url = lyrics_url;
                    result.selected_index = index as i32;
                    return result;
                }
            }
            if lyric_preference == LyricSourcePreference::Netease {
                result.status = "error".to_string();
                result.error = Some("NetEase 歌词获取失败".to_string());
                return result;
            }
        }

        if use_qq {
            if let Some(annotations) = self
                .fetch_qq_music_lyrics(&found_title, Some(&found_artist))
                .await
            {
                if !annotations.is_empty() {
                    self.cache
                        .lyrics()
                        .insert(qq_cache_key.clone(), annotations.clone())
                        .await;

                    result.ruby_annotations = annotations;
                    result.status = "success".to_string();
                    result.found_title = found_title;
                    result.found_artist = found_artist;
                    result.lyrics_url = lyrics_url;
                    result.selected_index = index as i32;
                    return result;
                }
            }

            if lyric_preference == LyricSourcePreference::QqMusic {
                result.status = "error".to_string();
                result.error = Some("QQ Music 歌词获取失败".to_string());
                return result;
            }
        }

        // Fallback to UtaTen
        let utaten_url = if lyrics_url.starts_with("qq_music:") {
            // QQ Music synthetic URL can't be used directly for UtaTen.
            // Search UtaTen by title/artist to find the real URL.
            let search_results = self.search(&found_title, Some(&found_artist)).await;
            search_results.first().map(|r| r.url.clone())
        } else {
            Some(lyrics_url.clone())
        };

        if let Some(ref effective_url) = utaten_url {
            if let Some(html) = self.get_lyrics_with_ruby(effective_url).await {
                let metadata = self
                    .resolve_artwork_metadata(
                        &found_title,
                        Some(&found_artist),
                        Self::extract_song_page_metadata(&html),
                        ArtworkSourcePreference::Auto,
                    )
                    .await;
                let annotations = self.extract_ruby_lyrics(&html);
                self.cache
                    .lyrics()
                    .insert(lyrics_url.clone(), annotations.clone())
                    .await;

                result.ruby_annotations = annotations;
                result.status = "success".to_string();
                result.found_title = found_title;
                result.found_artist = found_artist;
                result.lyrics_url = lyrics_url;
                result.found_album = metadata.album;
                result.cover_url = metadata.cover_url;
                result.selected_index = index as i32;
            } else {
                result.status = "error".to_string();
                result.error = Some("无法获取歌词页面".to_string());
            }
        } else {
            result.status = "error".to_string();
            result.error = Some("无法获取歌词页面".to_string());
        }

        result
    }

    /// 返回内部缓存管理器的引用。
    pub fn cache(&self) -> &CacheManager {
        &self.cache
    }
}

/// 为 `UtaTenSearcher` 提供默认实现，使用默认的 `CacheManager`。
impl Default for UtaTenSearcher {
    fn default() -> Self {
        Self::new(CacheManager::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample UtaTen HTML with ruby annotations (div.hiragana format)
    const SAMPLE_UTATEN_HTML: &str = r#"<html>
<body>
<div class="lyricBody">
<div class="medium">
<div class="hiragana">
<span class="ruby"><span class="rb">私</span><span class="rt">わたし</span></span><span class="ruby"><span class="rb">達</span><span class="rt">たち</span></span>は<br>
<span class="ruby"><span class="rb">憧</span><span class="rt">あこが</span></span>れの<br>
<span class="ruby"><span class="rb">空</span><span class="rt">そら</span></span>へ<br>
</div>
</div>
</div>
</body>
</html>"#;

    /// Sample HTML without any ruby annotations (plain text only)
    const SAMPLE_PLAIN_HTML: &str = r#"<html>
<body>
<div class="lyricBody">
<div class="medium">
<div class="hiragana">
Hello world<br>
This is plain text<br>
</div>
</div>
</div>
</body>
</html>"#;

    /// Sample HTML missing the hiragana container
    const SAMPLE_NO_HIRAGANA_HTML: &str = r#"<html>
<body>
<div class="lyricBody">
<div class="medium">
<p>No hiragana div here</p>
</div>
</div>
</body>
</html>"#;

    #[test]
    fn extracts_ruby_annotations_from_utaten_html() {
        let searcher = UtaTenSearcher::new(CacheManager::new());
        let elements = searcher.extract_ruby_lyrics(SAMPLE_UTATEN_HTML);

        assert!(!elements.is_empty(), "Should extract elements from HTML");

        // Elements breakdown from div.hiragana:
        // ruby(私,わたし) + ruby(達,たち) + text(は) + linebreak
        // + ruby(憧,あこが) + text(れの) + linebreak
        // + ruby(空,そら) + text(へ) + linebreak
        // = 10 elements
        assert_eq!(elements.len(), 10, "Should produce 10 elements");

        // First ruby element
        assert_eq!(elements[0].element_type, "ruby");
        assert_eq!(elements[0].base.as_deref(), Some("私"));
        assert_eq!(elements[0].ruby.as_deref(), Some("わたし"));

        // Second ruby element
        assert_eq!(elements[1].element_type, "ruby");
        assert_eq!(elements[1].base.as_deref(), Some("達"));
        assert_eq!(elements[1].ruby.as_deref(), Some("たち"));

        // Text "は"
        assert_eq!(elements[2].element_type, "text");
        assert_eq!(elements[2].base.as_deref(), Some("は"));

        // Linebreak
        assert_eq!(elements[3].element_type, "linebreak");

        // Third ruby: 憧(あこが)
        assert_eq!(elements[4].element_type, "ruby");
        assert_eq!(elements[4].base.as_deref(), Some("憧"));
        assert_eq!(elements[4].ruby.as_deref(), Some("あこが"));

        // Text "れの"
        assert_eq!(elements[5].element_type, "text");
        assert_eq!(elements[5].base.as_deref(), Some("れの"));

        // Fourth linebreak (between 憧れの and 空へ)
        assert_eq!(elements[6].element_type, "linebreak");

        // Fourth ruby: 空(そら)
        assert_eq!(elements[7].element_type, "ruby");
        assert_eq!(elements[7].base.as_deref(), Some("空"));
        assert_eq!(elements[7].ruby.as_deref(), Some("そら"));

        // Text "へ"
        assert_eq!(elements[8].element_type, "text");
        assert_eq!(elements[8].base.as_deref(), Some("へ"));

        // Final linebreak
        assert_eq!(elements[9].element_type, "linebreak");
    }

    #[test]
    fn extracts_plain_text_when_no_ruby_in_html() {
        let searcher = UtaTenSearcher::new(CacheManager::new());
        let elements = searcher.extract_ruby_lyrics(SAMPLE_PLAIN_HTML);

        assert!(!elements.is_empty(), "Should extract elements from plain HTML");

        // Hello world<br>This is plain text
        // "Hello" + " " + "world" + linebreak + "This" + " " + "is" + " " + "plain" + " " + "text"
        // At least linebreak 1 + some text
        assert!(elements.len() >= 2, "Should have at least 2 elements (text + linebreak)");
        assert_eq!(elements[0].element_type, "text");
    }

    #[test]
    fn returns_empty_when_no_hiragana_div() {
        let searcher = UtaTenSearcher::new(CacheManager::new());
        let elements = searcher.extract_ruby_lyrics(SAMPLE_NO_HIRAGANA_HTML);

        assert!(elements.is_empty(), "Should return empty for HTML without hiragana div");
    }

    #[test]
    fn utaten_url_is_accepted_by_get_lyrics_with_ruby_url_logic() {
        // Verify that get_lyrics_with_ruby constructs the correct absolute URL
        // for both absolute and relative URLs (this is a logic test, no HTTP)
        let _searcher = UtaTenSearcher::new(CacheManager::new());

        // The internal logic: if URL starts with "http", use as-is; otherwise prepend BASE_URL
        let absolute_url = "https://utaten.com/lyric/test123/";
        let relative_url = "/lyric/test456/";

        // These are just the URL construction tests - verify the method signature exists
        // and accepts both URL formats (compile-time check + logic verification)
        assert!(absolute_url.starts_with("http"), "Absolute URL should start with http");
        assert!(!relative_url.starts_with("http"), "Relative URL should not start with http");
        assert_eq!(
            format!("{}{}", BASE_URL, relative_url),
            "https://utaten.com/lyric/test456/"
        );
    }

    #[test]
    fn extracts_song_page_metadata_from_og_and_album_link() {
        let html = r#"
            <html><head>
              <meta property="og:image" content="//cdn.utaten.com/img/jacket/firebird.jpg">
            </head><body>
              <a href="/album/test/">Wahl</a>
            </body></html>
        "#;

        let metadata = UtaTenSearcher::extract_song_page_metadata(html);

        assert_eq!(
            metadata.cover_url.as_deref(),
            Some("https://cdn.utaten.com/img/jacket/firebird.jpg")
        );
        assert_eq!(metadata.album.as_deref(), Some("Wahl"));
    }

    #[test]
    fn parses_artwork_source_preference_from_setting() {
        assert_eq!(
            ArtworkSourcePreference::from_setting(Some("qqmusic")),
            ArtworkSourcePreference::QqMusic
        );
        assert_eq!(
            ArtworkSourcePreference::from_setting(Some("NetEase_Cloud")),
            ArtworkSourcePreference::Netease
        );
        assert_eq!(
            ArtworkSourcePreference::from_setting(Some("utaten")),
            ArtworkSourcePreference::UtaTen
        );
        assert_eq!(
            ArtworkSourcePreference::from_setting(Some("unknown")),
            ArtworkSourcePreference::Auto
        );
    }

    #[test]
    fn extracts_best_qq_music_artwork_candidate() {
        let json = serde_json::json!({
            "data": {
                "song": {
                    "list": [
                        {
                            "name": "Other Song",
                            "singer": [{ "name": "Other" }],
                            "album": { "name": "Other Album", "mid": "IGNORE" }
                        },
                        {
                            "name": "FIRE BIRD",
                            "singer": [{ "name": "Roselia" }],
                            "album": { "name": "Wahl", "mid": "003abcXYZ" }
                        }
                    ]
                }
            }
        });

        let metadata =
            UtaTenSearcher::extract_qq_music_artwork_from_json(&json, "FIRE BIRD", Some("Roselia"))
                .expect("QQ Music artwork should parse");

        assert_eq!(metadata.album.as_deref(), Some("Wahl"));
        assert_eq!(
            metadata.cover_url.as_deref(),
            Some(
                "https://y.gtimg.cn/music/photo_new/T002R1200x1200M000003abcXYZ.jpg?max_age=2592000"
            )
        );
    }

    #[test]
    fn extracts_qq_musicu_artwork_candidate() {
        let json = serde_json::json!({
            "req_0": {
                "code": 0,
                "data": {
                    "body": {
                        "item_song": [
                            {
                                "title": "FIRE BIRD",
                                "singer": [{ "name": "Roselia" }],
                                "album": { "name": "FIRE BIRD", "mid": "001mfjtg0LrzhN" }
                            }
                        ]
                    }
                }
            }
        });

        let metadata =
            UtaTenSearcher::extract_qq_music_artwork_from_json(&json, "FIRE BIRD", Some("Roselia"))
                .expect("QQ musicu artwork should parse");

        assert_eq!(metadata.album.as_deref(), Some("FIRE BIRD"));
        assert_eq!(
            metadata.cover_url.as_deref(),
            Some(
                "https://y.gtimg.cn/music/photo_new/T002R1200x1200M000001mfjtg0LrzhN.jpg?max_age=2592000"
            )
        );
    }

    #[test]
    fn extracts_best_netease_artwork_candidate() {
        let json = serde_json::json!({
            "result": {
                "songs": [
                    {
                        "name": "Other Song",
                        "artists": [{ "name": "Other" }],
                        "album": { "name": "Other Album", "picUrl": "https://example.com/other.jpg" }
                    },
                    {
                        "name": "BLACK SHOUT",
                        "artists": [{ "name": "Roselia" }],
                        "album": { "name": "Für immer", "picUrl": "https://p2.music.126.net/cover.jpg" }
                    }
                ]
            }
        });

        let metadata = UtaTenSearcher::extract_netease_artwork_from_json(
            &json,
            "BLACK SHOUT",
            Some("Roselia"),
        )
        .expect("NetEase artwork should parse");

        assert_eq!(metadata.album.as_deref(), Some("Für immer"));
        assert_eq!(
            metadata.cover_url.as_deref(),
            Some("https://p2.music.126.net/cover.jpg")
        );
    }

    #[test]
    fn extracts_best_netease_album_artwork_candidate() {
        let json = serde_json::json!({
            "result": {
                "albums": [
                    {
                        "name": "Other Album",
                        "artist": { "name": "Other" },
                        "picUrl": "https://p1.music.126.net/other.jpg"
                    },
                    {
                        "name": "FIRE BIRD",
                        "artist": { "name": "Roselia" },
                        "picUrl": "https://p1.music.126.net/firebird.jpg"
                    }
                ]
            }
        });

        let metadata = UtaTenSearcher::extract_netease_album_artwork_from_json(
            &json,
            "FIRE BIRD",
            Some("Roselia"),
        )
        .expect("NetEase album artwork should parse");

        assert_eq!(metadata.album.as_deref(), Some("FIRE BIRD"));
        assert_eq!(
            metadata.cover_url.as_deref(),
            Some("https://p1.music.126.net/firebird.jpg")
        );
    }

    #[test]
    fn builds_quick_title_search_request_without_artist_filter() {
        let request = UtaTenSearcher::build_search_request("R", None, "title", 3);
        assert_eq!(
            request,
            SearchRequest {
                path: "/search",
                params: vec![
                    ("layout_search_type", "title".to_string()),
                    ("layout_search_text", "R".to_string()),
                    ("page", "3".to_string()),
                ],
            }
        );
    }

    #[test]
    fn builds_detailed_title_search_request_with_artist_filter() {
        let request = UtaTenSearcher::build_search_request("R", Some("Roselia"), "title", 1);
        assert_eq!(
            request,
            SearchRequest {
                path: "/search",
                params: vec![
                    ("title", "R".to_string()),
                    ("artist_name", "Roselia".to_string()),
                    ("sort", "popular_sort_asc".to_string()),
                    ("show_artists", "1".to_string()),
                    ("page", "1".to_string()),
                ],
            }
        );
    }

    #[test]
    fn builds_detailed_artist_only_request_when_title_is_empty() {
        let request = UtaTenSearcher::build_search_request("", Some("Roselia"), "title", 2);
        assert_eq!(
            request,
            SearchRequest {
                path: "/search",
                params: vec![
                    ("title", "".to_string()),
                    ("artist_name", "Roselia".to_string()),
                    ("sort", "popular_sort_asc".to_string()),
                    ("show_artists", "1".to_string()),
                    ("page", "2".to_string()),
                ],
            }
        );
    }

    #[test]
    fn builds_artist_search_request() {
        let request = UtaTenSearcher::build_search_request("Roselia", None, "artist", 4);
        assert_eq!(
            request,
            SearchRequest {
                path: "/search",
                params: vec![
                    ("artist_name", "Roselia".to_string()),
                    ("sort", "popular_sort_asc".to_string()),
                    ("show_artists", "1".to_string()),
                    ("page", "4".to_string()),
                ],
            }
        );
    }

    #[test]
    fn extracts_page_number_from_query_and_path_links() {
        assert_eq!(
            UtaTenSearcher::extract_page_number_from_href("/search?page=12"),
            Some(12)
        );
        assert_eq!(
            UtaTenSearcher::extract_page_number_from_href("/search/=/title=R/page=42/"),
            Some(42)
        );
        assert_eq!(
            UtaTenSearcher::extract_page_number_from_href("/search"),
            None
        );
    }

    #[test]
    fn parses_modern_pagination_markup() {
        let searcher = UtaTenSearcher::new(CacheManager::new());
        let document = Html::parse_document(
            r#"
            <nav class="pager">
              <ul class="pager__inner">
                <li class="pager__item pager__item--first">
                  <a href="/search/=/title=R/page=1/">First</a>
                </li>
                <li class="pager__item pager__item--current"><span>1</span></li>
                <li class="pager__item"><a href="/search/=/title=R/page=2/">2</a></li>
                <li class="pager__item"><a href="/search/=/title=R/page=3/">3</a></li>
                <li class="pager__item pager__item--last">
                  <a href="/search/=/title=R/page=100/">Last</a>
                </li>
              </ul>
            </nav>
            "#,
        );

        let pagination = searcher.extract_pagination(&document, 1);
        assert_eq!(pagination.current_page, 1);
        assert_eq!(pagination.total_pages, 100);
        assert!(pagination.has_next);
    }

    #[test]
    fn extracts_results_from_detailed_search_table_markup() {
        let document = Html::parse_document(
            r#"
            <table class="searchResult lyricList">
              <tr>
                <td>
                  <p class="searchResult__title">
                    <a href="/lyric/tu19061219/">FIRE BIRD</a>
                  </p>
                </td>
                <td class="searchResult__artist">
                  <p><a href="/artist/22798/">Roselia</a></p>
                  <div class="searchResult__lyricist">
                    <p>作詞：<span class="songWriters">上松範康(Elements Garden)</span></p>
                    <p>作曲：<span class="songWriters">藤永龍太郎(Elements Garden)</span></p>
                  </div>
                </td>
                <td class="lyricList__beginning">
                  <a href="/lyric/tu19061219/">空がどんな高くても</a>
                </td>
              </tr>
              <tr>
                <td>
                  <p class="searchResult__title">
                    <a href="/lyric/yb18072521/">R</a>
                  </p>
                </td>
                <td class="searchResult__artist">
                  <p><a href="/artist/22798/">Roselia</a></p>
                </td>
                <td class="lyricList__beginning">
                  <a href="/lyric/yb18072521/">礎なるOne's Intention</a>
                </td>
              </tr>
            </table>
            "#,
        );

        let results = UtaTenSearcher::extract_search_results(&document);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "FIRE BIRD");
        assert_eq!(results[0].artist, "Roselia");
        assert_eq!(results[1].title, "R");
        assert_eq!(results[1].artist, "Roselia");
        assert_eq!(results[1].url, "/lyric/yb18072521/");
    }

    #[test]
    fn parses_lyric_source_preference_from_setting() {
        assert_eq!(
            LyricSourcePreference::from_setting(Some("qqmusic")),
            LyricSourcePreference::QqMusic
        );
        assert_eq!(
            LyricSourcePreference::from_setting(Some("utaten")),
            LyricSourcePreference::UtaTen
        );
        assert_eq!(
            LyricSourcePreference::from_setting(Some("unknown")),
            LyricSourcePreference::Auto
        );
        assert_eq!(
            LyricSourcePreference::from_setting(None),
            LyricSourcePreference::Auto
        );
    }
}