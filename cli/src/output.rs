//! 输出数据结构，用于 JSON 序列化。
//!
//! 该模块定义了所有 CLI 命令返回的 JSON 输出结构，
//! 包括歌词、搜索、错误和历史记录等输出类型。

use crate::models::{LyricElement, SearchResult};
use serde::{Deserialize, Serialize};

/// 歌词元素，表示歌词中的一个基本单元。
///
/// 可以是普通文本、注音（ruby）或换行符。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsElement {
    /// 元素类型：`"text"`（文本）、`"ruby"`（注音）或 `"linebreak"`（换行）
    #[serde(rename = "type")]
    pub element_type: String,
    /// 元素的基准文本（对于注音元素为汉字，对于文本元素为普通文字）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// 注音假名（仅当 `element_type` 为 `"ruby"` 时存在）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruby: Option<String>,
}

impl LyricsElement {
    /// 从 [`LyricElement`] 模型创建 [`LyricsElement`] 实例。
    ///
    /// # 参数
    /// * `elem` - 来自内部模型的歌词元素
    ///
    /// # 返回值
    /// 返回一个 JSON 序列化用的歌词元素
    pub fn from_model(elem: &LyricElement) -> Self {
        Self {
            element_type: elem.element_type.clone(),
            base: elem.base.clone(),
            ruby: elem.ruby.clone(),
        }
    }

    /// 创建一个注音元素（ruby）。
    ///
    /// # 参数
    /// * `base` - 基准文本（汉字）
    /// * `ruby` - 注音假名
    ///
    /// # 返回值
    /// 返回一个类型为 `"ruby"` 的歌词元素
    pub fn ruby(base: String, ruby: String) -> Self {
        Self {
            element_type: "ruby".to_string(),
            base: Some(base),
            ruby: Some(ruby),
        }
    }

    /// 创建一个纯文本元素。
    ///
    /// # 参数
    /// * `base` - 文本内容
    ///
    /// # 返回值
    /// 返回一个类型为 `"text"` 的歌词元素
    pub fn text(base: String) -> Self {
        Self {
            element_type: "text".to_string(),
            base: Some(base),
            ruby: None,
        }
    }

    /// 创建一个换行元素。
    ///
    /// # 返回值
    /// 返回一个类型为 `"linebreak"` 的歌词元素
    pub fn linebreak() -> Self {
        Self {
            element_type: "linebreak".to_string(),
            base: None,
            ruby: None,
        }
    }
}

/// 歌词行，包含一行中的所有歌词元素。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsLine {
    /// 该行包含的歌词元素列表
    #[serde(rename = "elements")] // 保持向后兼容
    pub units: Vec<LyricsElement>,
}

impl LyricsLine {
    /// 创建一个新的歌词行。
    ///
    /// # 参数
    /// * `units` - 该行的歌词元素列表
    ///
    /// # 返回值
    /// 返回一个包含指定元素的歌词行
    pub fn new(units: Vec<LyricsElement>) -> Self {
        Self { units }
    }
}

/// 歌词内容，包含所有歌词行。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsContent {
    /// 歌词行列表
    pub lines: Vec<LyricsLine>,
}

impl LyricsContent {
    /// 从原始歌词元素数组构造歌词内容。
    ///
    /// 根据换行元素自动将元素分组为多行。
    ///
    /// # 参数
    /// * `elements` - 原始歌词元素切片
    ///
    /// # 返回值
    /// 返回一个包含按行分组的歌词内容
    pub fn from_elements(elements: &[LyricElement]) -> Self {
        let mut lines: Vec<LyricsLine> = Vec::new();
        let mut current_line: Vec<LyricsElement> = Vec::new();

        for elem in elements {
            if elem.element_type == "linebreak" {
                if !current_line.is_empty() {
                    lines.push(LyricsLine::new(current_line));
                    current_line = Vec::new();
                }
            } else {
                current_line.push(LyricsElement::from_model(elem));
            }
        }

        if !current_line.is_empty() {
            lines.push(LyricsLine::new(current_line));
        }

        Self { lines }
    }
}

/// 歌词输出，表示获取歌词成功后的 JSON 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsOutput {
    /// 状态标识，固定为 `"success"`
    pub status: String,
    /// 歌曲标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 歌手名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    /// 歌词来源 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 歌词内容（按行组织）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics: Option<LyricsContent>,
}

impl LyricsOutput {
    /// 创建一个成功的歌词输出。
    ///
    /// # 参数
    /// * `title` - 歌曲标题
    /// * `artist` - 歌手名称
    /// * `url` - 歌词来源 URL
    /// * `elements` - 原始歌词元素切片
    ///
    /// # 返回值
    /// 返回一个状态为 `"success"` 的歌词输出
    pub fn success(title: String, artist: String, url: String, elements: &[LyricElement]) -> Self {
        Self {
            status: "success".to_string(),
            title: Some(title),
            artist: Some(artist),
            url: Some(url),
            lyrics: Some(LyricsContent::from_elements(elements)),
        }
    }

    /// 将输出序列化为格式化的 JSON 字符串。
    ///
    /// # 返回值
    /// 返回包含格式化 JSON 的字符串，或返回序列化错误
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// 搜索查询参数，用于构建搜索请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// 歌曲标题关键词（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 歌手名称关键词（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
}

impl SearchQuery {
    /// 创建一个新的搜索查询。
    ///
    /// # 参数
    /// * `title` - 可选的歌曲标题关键词
    /// * `artist` - 可选的歌手名称关键词
    ///
    /// # 返回值
    /// 返回一个搜索查询实例
    pub fn new(title: Option<String>, artist: Option<String>) -> Self {
        Self { title, artist }
    }
}

/// 搜索结果项，表示单个搜索匹配结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// 结果序号（从 0 开始）
    pub index: usize,
    /// 歌曲标题
    pub title: String,
    /// 歌手名称
    pub artist: String,
    /// 歌词详情页 URL
    pub url: String,
    /// 是否匹配到精确结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<bool>,
    /// 作词者
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyricist: Option<String>,
    /// 作曲者
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composer: Option<String>,
    /// 数据来源
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl SearchResultItem {
    /// 从 [`SearchResult`] 模型创建 [`SearchResultItem`] 实例。
    ///
    /// # 参数
    /// * `index` - 结果序号
    /// * `result` - 来自内部模型的搜索结果
    ///
    /// # 返回值
    /// 返回一个 JSON 序列化用的搜索结果项
    pub fn from_model(index: usize, result: &SearchResult) -> Self {
        Self {
            index,
            title: result.title.clone(),
            artist: result.artist.clone(),
            url: result.url.clone(),
            matched: if result.matched { Some(true) } else { None },
            lyricist: result.lyricist.clone(),
            composer: result.composer.clone(),
            source: Some(result.source.clone()).filter(|s| !s.is_empty()),
        }
    }
}

/// 搜索输出，表示搜索命令的 JSON 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOutput {
    /// 状态标识，固定为 `"select"`
    pub status: String,
    /// 本次搜索的查询参数
    pub query: SearchQuery,
    /// 当前页码
    pub page: u32,
    /// 总页数
    pub total_pages: u32,
    /// 搜索结果列表
    pub results: Vec<SearchResultItem>,
    /// 操作提示信息
    pub hint: String,
}

impl SearchOutput {
    /// 创建一个新的搜索输出。
    ///
    /// # 参数
    /// * `title` - 可选的搜索标题关键词
    /// * `artist` - 可选的搜索歌手关键词
    /// * `page` - 当前页码
    /// * `total_pages` - 总页数
    /// * `results` - 搜索结果切片
    ///
    /// # 返回值
    /// 返回一个包含搜索结果和选择提示的输出
    pub fn new(
        title: Option<String>,
        artist: Option<String>,
        page: u32,
        total_pages: u32,
        results: &[SearchResult],
    ) -> Self {
        let items: Vec<SearchResultItem> = results
            .iter()
            .enumerate()
            .map(|(i, r)| SearchResultItem::from_model(i, r))
            .collect();

        Self {
            status: "select".to_string(),
            query: SearchQuery::new(title, artist),
            page,
            total_pages,
            results: items,
            hint: "使用 --select <index> 选择结果".to_string(),
        }
    }

    /// 将输出序列化为格式化的 JSON 字符串。
    ///
    /// # 返回值
    /// 返回包含格式化 JSON 的字符串，或返回序列化错误
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// 错误输出，表示操作失败的 JSON 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOutput {
    /// 状态标识，取值为 `"no_results"` 或 `"error"`
    pub status: String,
    /// 错误描述信息
    pub message: String,
}

impl ErrorOutput {
    /// 创建一个"无结果"错误输出。
    ///
    /// # 参数
    /// * `message` - 错误描述信息
    ///
    /// # 返回值
    /// 返回一个状态为 `"no_results"` 的错误输出
    pub fn no_results(message: &str) -> Self {
        Self {
            status: "no_results".to_string(),
            message: message.to_string(),
        }
    }

    /// 创建一个通用错误输出。
    ///
    /// # 参数
    /// * `message` - 错误描述信息
    ///
    /// # 返回值
    /// 返回一个状态为 `"error"` 的错误输出
    pub fn error(message: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: message.to_string(),
        }
    }

    /// 将输出序列化为格式化的 JSON 字符串。
    ///
    /// # 返回值
    /// 返回包含格式化 JSON 的字符串，或返回序列化错误
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// 历史记录项，表示单条查询历史。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    /// 记录序号（从 0 开始）
    pub index: usize,
    /// 歌曲标题
    pub title: String,
    /// 歌手名称（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    /// 歌词来源 URL（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 查询时间戳
    pub timestamp: String,
    /// 作词者（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyricist: Option<String>,
    /// 作曲者（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composer: Option<String>,
}

/// 历史记录输出，表示历史查询命令的 JSON 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryOutput {
    /// 状态标识，固定为 `"success"`
    pub status: String,
    /// 历史记录总数
    pub count: usize,
    /// 历史记录列表
    pub items: Vec<HistoryItem>,
}

impl HistoryOutput {
    /// 创建一个包含历史记录的输出。
    ///
    /// # 参数
    /// * `items` - 历史记录项列表
    ///
    /// # 返回值
    /// 返回一个状态为 `"success"` 的历史记录输出
    pub fn new(items: Vec<HistoryItem>) -> Self {
        let count = items.len();
        Self {
            status: "success".to_string(),
            count,
            items,
        }
    }

    /// 创建一个空的历史记录输出。
    ///
    /// # 返回值
    /// 返回一个不含任何记录的历史记录输出
    pub fn empty() -> Self {
        Self {
            status: "success".to_string(),
            count: 0,
            items: Vec::new(),
        }
    }

    /// 将输出序列化为格式化的 JSON 字符串。
    ///
    /// # 返回值
    /// 返回包含格式化 JSON 的字符串，或返回序列化错误
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// 输出枚举，统一表示所有可能的命令输出类型。
pub enum Output {
    /// 歌词输出
    Lyrics(LyricsOutput),
    /// 搜索输出
    Search(SearchOutput),
    /// 错误输出
    Error(ErrorOutput),
    /// 历史记录输出
    History(HistoryOutput),
}

impl Output {
    /// 将当前输出序列化为格式化的 JSON 字符串。
    ///
    /// 自动根据枚举类型分发到对应的输出结构进行序列化。
    ///
    /// # 返回值
    /// 返回包含格式化 JSON 的字符串，或返回序列化错误
    pub fn to_json(&self) -> anyhow::Result<String> {
        match self {
            Output::Lyrics(o) => o.to_json(),
            Output::Search(o) => o.to_json(),
            Output::Error(o) => o.to_json(),
            Output::History(o) => o.to_json(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lyrics_content_from_elements() {
        let model_elements = vec![
            LyricElement {
                element_type: "text".to_string(),
                base: Some("こんにちは".to_string()),
                ruby: None,
            },
            LyricElement {
                element_type: "linebreak".to_string(),
                base: None,
                ruby: None,
            },
            LyricElement {
                element_type: "ruby".to_string(),
                base: Some("私".to_string()),
                ruby: Some("わたし".to_string()),
            },
        ];

        let content = LyricsContent::from_elements(&model_elements);

        assert_eq!(content.lines.len(), 2);
        assert_eq!(content.lines[0].units.len(), 1);
        assert_eq!(content.lines[1].units.len(), 1);
    }
} // end mod tests
