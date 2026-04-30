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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SongPageMetadata {
    pub album: Option<String>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkSourcePreference {
    Auto,
    UtaTen,
    QqMusic,
    Netease,
}

impl ArtworkSourcePreference {
    pub fn from_setting(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("utaten") => Self::UtaTen,
            Some("qq") | Some("qqmusic") | Some("qq_music") => Self::QqMusic,
            Some("netease") | Some("neteasecloud") | Some("netease_cloud") => Self::Netease,
            _ => Self::Auto,
        }
    }
}

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

const BASE_URL: &str = "https://utaten.com";
const REQUEST_DELAY_MS: u64 = 500;
const REQUEST_TIMEOUT_SECS: u64 = 15;

pub struct UtaTenSearcher {
    client: Client,
    pub cache: CacheManager,
    delay: Duration,
    last_request: Arc<Mutex<Instant>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchRequest {
    path: &'static str,
    params: Vec<(&'static str, String)>,
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

    fn has_japanese(text: &str) -> bool {
        text.chars().any(|c| {
            ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c)
        })
    }

    pub async fn search(&self, title: &str, artist: Option<&str>) -> Vec<SearchResult> {
        self.search_with_options(title, artist, "title", 1)
            .await
            .results
    }

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

        results
    }

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

    fn merge_album_cover(
        primary: SongPageMetadata,
        fallback: SongPageMetadata,
    ) -> SongPageMetadata {
        SongPageMetadata {
            album: primary.album.or(fallback.album),
            cover_url: primary.cover_url.or(fallback.cover_url),
        }
    }

    fn song_query(title: &str, artist: Option<&str>) -> String {
        match artist.map(str::trim).filter(|value| !value.is_empty()) {
            Some(artist) => format!("{} {}", title.trim(), artist),
            None => title.trim().to_string(),
        }
    }

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
            info!(
                "
=== [CACHE HIT] ==="
            );
            info!("  URL: {}", lyrics_url);
            info!("  Title: {}", found_title);
            info!("  Artist: {}", found_artist);
            info!("  Elements: {}", cached_annotations.len());
            info!(
                "===================
"
            );

            result.ruby_annotations = cached_annotations;
            result.status = "success".to_string();
            result.found_title = found_title;
            result.found_artist = found_artist;
            result.lyrics_url = lyrics_url;
            result.selected_index = index as i32;
            return result;
        }

        info!(
            "
=== [CACHE MISS] ==="
        );
        info!("  URL: {}", lyrics_url);
        info!("  Title: {}", found_title);
        info!("  Artist: {}", found_artist);
        info!("  Fetching from UtaTen...");
        info!(
            "===================
"
        );

        if let Some(html) = self.get_lyrics_with_ruby(&lyrics_url).await {
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

            info!(
                "
=== [CACHE STORED] ==="
            );
            info!("  URL: {}", lyrics_url);
            info!("  Title: {}", found_title);
            info!("  Artist: {}", found_artist);
            info!("  Elements: {}", annotations.len());
            info!(
                "===================
"
            );

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
