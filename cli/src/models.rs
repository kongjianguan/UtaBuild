//! 数据模型定义：歌词元素、搜索结果、响应结构等

use serde::{Deserialize, Serialize};

/// 歌词元素，表示歌词中的一个片段（汉字、假名、换行等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricElement {
    /// 元素类型：ruby（注音）、text（纯文本）、linebreak（换行）
    #[serde(rename = "type")]
    pub element_type: String,
    /// 基础文本（汉字或假名原文）
    pub base: Option<String>,
    /// 注音假名（ruby 类型时有效）
    pub ruby: Option<String>,
}

impl LyricElement {
    /// 创建一个带注音的 ruby 元素
    pub fn new_ruby(base: String, ruby: String) -> Self {
        Self {
            element_type: "ruby".to_string(),
            base: Some(base),
            ruby: Some(ruby),
        }
    }

    /// 创建一个纯文本元素
    pub fn new_text(base: String) -> Self {
        Self {
            element_type: "text".to_string(),
            base: Some(base),
            ruby: None,
        }
    }

    /// 创建一个换行元素
    pub fn new_linebreak() -> Self {
        Self {
            element_type: "linebreak".to_string(),
            base: None,
            ruby: None,
        }
    }
}

/// 歌词搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 歌曲标题
    pub title: String,
    /// 歌手名
    pub artist: String,
    /// 歌词页面 URL
    pub url: String,
    /// 是否精确匹配
    pub matched: bool,
    /// 作词人
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyricist: Option<String>,
    /// 作曲人
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composer: Option<String>,
    /// 专辑名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    /// 封面图 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    /// 数据来源（utaten / qq_music / netease）
    #[serde(default)]
    pub source: String,
}

impl SearchResult {
    /// 创建一个新的搜索结果项
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
            source: String::new(),
        }
    }

    /// 创建包含作词/作曲信息的搜索结果项
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
            source: String::new(),
        }
    }

    /// 设置数据来源标签
    pub fn with_source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }
}

/// 搜索结果分页信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchPagination {
    /// 当前页码
    pub current_page: u32,
    /// 总页数
    pub total_pages: u32,
    /// 是否有下一页
    pub has_next: bool,
}

/// 搜索响应，包含状态、分页、结果列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// 响应状态：pending / select / not_found / error
    pub status: String,
    /// 查询的歌名
    pub query_title: Option<String>,
    /// 查询的歌手
    pub query_artist: Option<String>,
    /// 搜索类型：title / artist
    pub search_type: String,
    /// 当前页码
    pub page: u32,
    /// 分页信息
    pub pagination: Option<SearchPagination>,
    /// 搜索结果列表
    pub results: Vec<SearchResult>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SearchResponse {
    /// 创建一个空的搜索响应，状态为 pending
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

/// 歌词搜索完整响应，包含结果和注音注释
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsSearchResponse {
    /// 请求的歌曲标题
    pub title: String,
    /// 请求的歌手
    pub artist: String,
    /// 响应状态
    pub status: String,
    /// 搜索结果列表
    pub search_results: Vec<SearchResult>,
    /// 用户选择的索引
    pub selected_index: i32,
    /// 歌词页面 URL
    pub lyrics_url: String,
    /// 注音注释列表
    pub ruby_annotations: Vec<LyricElement>,
    /// 时间戳
    pub timestamp: String,
    /// 错误信息
    pub error: Option<String>,
    /// 是否找到匹配
    pub matched: bool,
    /// 实际找到的标题
    pub found_title: String,
    /// 实际找到的歌手
    pub found_artist: String,
    /// 是否来自缓存
    pub from_cache: bool,
    /// 实际找到的专辑
    #[serde(skip_serializing_if = "Option::is_none")]
    pub found_album: Option<String>,
    /// 封面图 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
}

impl LyricsSearchResponse {
    /// 创建一个新的歌词搜索响应，初始状态为 pending
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
