use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricElement {
    #[serde(rename = "type")]
    pub element_type: String,
    pub base: Option<String>,
    pub ruby: Option<String>,
}

impl LyricElement {
    pub fn new_ruby(base: String, ruby: String) -> Self {
        Self {
            element_type: "ruby".to_string(),
            base: Some(base),
            ruby: Some(ruby),
        }
    }

    pub fn new_text(base: String) -> Self {
        Self {
            element_type: "text".to_string(),
            base: Some(base),
            ruby: None,
        }
    }

    pub fn new_linebreak() -> Self {
        Self {
            element_type: "linebreak".to_string(),
            base: None,
            ruby: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyricist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

impl SearchResult {
    pub fn new(title: String, artist: String, url: String) -> Self {
        Self {
            title,
            artist,
            url,
            matched: false,
            lyricist: None,
            composer: None,
            album: None,
            cover_url: None,
        }
    }

    pub fn with_artist_info(
        title: String,
        artist: String,
        url: String,
        lyricist: Option<String>,
        composer: Option<String>,
    ) -> Self {
        Self {
            title,
            artist,
            url,
            matched: false,
            lyricist,
            composer,
            album: None,
            cover_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchPagination {
    pub current_page: u32,
    pub total_pages: u32,
    pub has_next: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub status: String,
    pub query_title: Option<String>,
    pub query_artist: Option<String>,
    pub search_type: String,
    pub page: u32,
    pub pagination: Option<SearchPagination>,
    pub results: Vec<SearchResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SearchResponse {
    pub fn new() -> Self {
        Self {
            status: "pending".to_string(),
            query_title: None,
            query_artist: None,
            search_type: "title".to_string(),
            page: 1,
            pagination: None,
            results: Vec::new(),
            error: None,
        }
    }
}

impl Default for SearchResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsSearchResponse {
    pub title: String,
    pub artist: String,
    pub status: String,
    pub search_results: Vec<SearchResult>,
    pub selected_index: i32,
    pub lyrics_url: String,
    pub ruby_annotations: Vec<LyricElement>,
    pub timestamp: String,
    pub error: Option<String>,
    pub matched: bool,
    pub found_title: String,
    pub found_artist: String,
    pub from_cache: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub found_album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

impl LyricsSearchResponse {
    pub fn new(title: String, artist: Option<String>) -> Self {
        Self {
            title,
            artist: artist.unwrap_or_default(),
            status: "pending".to_string(),
            search_results: Vec::new(),
            selected_index: -1,
            lyrics_url: String::new(),
            ruby_annotations: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            error: None,
            matched: false,
            found_title: String::new(),
            found_artist: String::new(),
            from_cache: false,
            found_album: None,
            cover_url: None,
        }
    }
}
