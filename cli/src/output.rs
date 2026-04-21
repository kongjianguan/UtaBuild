use crate::models::{LyricElement, SearchResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsElement {
    #[serde(rename = "type")]
    pub element_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruby: Option<String>,
}

impl LyricsElement {
    pub fn from_model(elem: &LyricElement) -> Self {
        Self {
            element_type: elem.element_type.clone(),
            base: elem.base.clone(),
            ruby: elem.ruby.clone(),
        }
    }

    pub fn ruby(base: String, ruby: String) -> Self {
        Self {
            element_type: "ruby".to_string(),
            base: Some(base),
            ruby: Some(ruby),
        }
    }

    pub fn text(base: String) -> Self {
        Self {
            element_type: "text".to_string(),
            base: Some(base),
            ruby: None,
        }
    }

    pub fn linebreak() -> Self {
        Self {
            element_type: "linebreak".to_string(),
            base: None,
            ruby: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsLine {
    #[serde(rename = "elements")] // 保持向后兼容
    pub units: Vec<LyricsElement>,
}

impl LyricsLine {
    pub fn new(units: Vec<LyricsElement>) -> Self {
        Self { units }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsContent {
    pub lines: Vec<LyricsLine>,
}

impl LyricsContent {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsOutput {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics: Option<LyricsContent>,
}

impl LyricsOutput {
    pub fn success(title: String, artist: String, url: String, elements: &[LyricElement]) -> Self {
        Self {
            status: "success".to_string(),
            title: Some(title),
            artist: Some(artist),
            url: Some(url),
            lyrics: Some(LyricsContent::from_elements(elements)),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
}

impl SearchQuery {
    pub fn new(title: Option<String>, artist: Option<String>) -> Self {
        Self { title, artist }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub index: usize,
    pub title: String,
    pub artist: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyricist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composer: Option<String>,
}

impl SearchResultItem {
    pub fn from_model(index: usize, result: &SearchResult) -> Self {
        Self {
            index,
            title: result.title.clone(),
            artist: result.artist.clone(),
            url: result.url.clone(),
            matched: if result.matched { Some(true) } else { None },
            lyricist: result.lyricist.clone(),
            composer: result.composer.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOutput {
    pub status: String,
    pub query: SearchQuery,
    pub page: u32,
    pub total_pages: u32,
    pub results: Vec<SearchResultItem>,
    pub hint: String,
}

impl SearchOutput {
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

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOutput {
    pub status: String,
    pub message: String,
}

impl ErrorOutput {
    pub fn no_results(message: &str) -> Self {
        Self {
            status: "no_results".to_string(),
            message: message.to_string(),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: message.to_string(),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub index: usize,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyricist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryOutput {
    pub status: String,
    pub count: usize,
    pub items: Vec<HistoryItem>,
}

impl HistoryOutput {
    pub fn new(items: Vec<HistoryItem>) -> Self {
        let count = items.len();
        Self {
            status: "success".to_string(),
            count,
            items,
        }
    }

    pub fn empty() -> Self {
        Self {
            status: "success".to_string(),
            count: 0,
            items: Vec::new(),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

pub enum Output {
    Lyrics(LyricsOutput),
    Search(SearchOutput),
    Error(ErrorOutput),
    History(HistoryOutput),
}

impl Output {
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
    fn test_text_element_serialization() {
        let text_elem = LyricsElement::text("こんにちは".to_string());

        assert_eq!(text_elem.element_type, "text");
        assert_eq!(text_elem.base, Some("こんにちは".to_string()));
        assert_eq!(text_elem.ruby, None);

        let json = serde_json::to_string(&text_elem).unwrap();
        assert!(json.contains("text"));
        assert!(json.contains("こんにちは"));
        assert!(!json.contains("ruby"));
    }

    #[test]
    fn test_ruby_element_serialization() {
        let ruby_elem = LyricsElement::ruby("私".to_string(), "わたし".to_string());

        assert_eq!(ruby_elem.element_type, "ruby");
        assert_eq!(ruby_elem.base, Some("私".to_string()));
        assert_eq!(ruby_elem.ruby, Some("わたし".to_string()));

        let json = serde_json::to_string(&ruby_elem).unwrap();
        assert!(json.contains("ruby"));
        assert!(json.contains("私"));
        assert!(json.contains("わたし"));
    }

    #[test]
    fn test_linebreak_element() {
        let linebreak_elem = LyricsElement::linebreak();

        assert_eq!(linebreak_elem.element_type, "linebreak");
        assert_eq!(linebreak_elem.base, None);
        assert_eq!(linebreak_elem.ruby, None);

        let json = serde_json::to_string(&linebreak_elem).unwrap();
        assert!(json.contains("linebreak"));
        assert!(!json.contains("base"));
    }

    #[test]
    fn test_lyrics_line() {
        let elements = vec![
            LyricsElement::ruby("私".to_string(), "わたし".to_string()),
            LyricsElement::text("は".to_string()),
        ];

        let line = LyricsLine::new(elements);
        assert_eq!(line.units.len(), 2);
    }

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

    #[test]
    fn test_lyrics_output_serialization() {
        let elements = vec![LyricElement {
            element_type: "text".to_string(),
            base: Some("テスト".to_string()),
            ruby: None,
        }];

        let output = LyricsOutput::success(
            "テスト曲".to_string(),
            "テストアーティスト".to_string(),
            "https://example.com/test".to_string(),
            &elements,
        );

        assert_eq!(output.status, "success");
        assert_eq!(output.title, Some("テスト曲".to_string()));
        assert_eq!(output.artist, Some("テストアーティスト".to_string()));

        let json = output.to_json().unwrap();
        assert!(json.contains("\"status\": \"success\""));
        assert!(json.contains("\"title\": \"テスト曲\""));
        assert!(json.contains("\"artist\": \"テストアーティスト\""));
        assert!(json.contains("\"url\": \"https://example.com/test\""));
    }

    #[test]
    fn test_error_output_no_results() {
        let error = ErrorOutput::no_results("検索結果が見つかりませんでした");

        assert_eq!(error.status, "no_results");
        assert_eq!(error.message, "検索結果が見つかりませんでした");

        let json = error.to_json().unwrap();
        assert!(json.contains("\"status\": \"no_results\""));
        assert!(json.contains("\"message\": \"検索結果が見つかりませんでした\""));
    }

    #[test]
    fn test_error_output_error() {
        let error = ErrorOutput::error("エラーが発生しました");

        assert_eq!(error.status, "error");
        assert_eq!(error.message, "エラーが発生しました");

        let json = error.to_json().unwrap();
        assert!(json.contains("\"status\": \"error\""));
        assert!(json.contains("\"message\": \"エラーが発生しました\""));
    }

    #[test]
    fn test_search_query_serialization() {
        let query_with_both =
            SearchQuery::new(Some("曲名".to_string()), Some("アーティスト".to_string()));

        let json = serde_json::to_string(&query_with_both).unwrap();
        assert!(json.contains("曲名"));
        assert!(json.contains("アーティスト"));

        let query_with_title_only = SearchQuery::new(Some("曲名".to_string()), None);
        let json = serde_json::to_string(&query_with_title_only).unwrap();
        assert!(json.contains("曲名"));
        assert!(!json.contains("artist"));
    }

    #[test]
    fn test_history_output() {
        let items = vec![
            HistoryItem {
                index: 0,
                title: "曲1".to_string(),
                artist: Some("アーティスト1".to_string()),
                url: Some("https://example.com/1".to_string()),
                timestamp: "2024-01-01T12:00:00".to_string(),
                lyricist: Some("作詞者".to_string()),
                composer: Some("作曲者".to_string()),
            },
            HistoryItem {
                index: 1,
                title: "曲2".to_string(),
                artist: None,
                url: None,
                timestamp: "2024-01-02T12:00:00".to_string(),
                lyricist: None,
                composer: None,
            },
        ];

        let output = HistoryOutput::new(items);

        assert_eq!(output.status, "success");
        assert_eq!(output.count, 2);
        assert_eq!(output.items.len(), 2);

        let json = output.to_json().unwrap();
        assert!(json.contains("\"status\": \"success\""));
        assert!(json.contains("\"count\": 2"));
    }

    #[test]
    fn test_history_output_empty() {
        let output = HistoryOutput::empty();

        assert_eq!(output.status, "success");
        assert_eq!(output.count, 0);
        assert!(output.items.is_empty());

        let json = output.to_json().unwrap();
        assert!(json.contains("\"status\": \"success\""));
        assert!(json.contains("\"count\": 0"));
        assert!(json.contains("\"items\": []"));
    }

    #[test]
    fn test_output_enum_lyrics() {
        let output = Output::Lyrics(LyricsOutput {
            status: "success".to_string(),
            title: Some("テスト".to_string()),
            artist: None,
            url: None,
            lyrics: None,
        });

        let json = output.to_json().unwrap();
        assert!(json.contains("\"status\": \"success\""));
    }

    #[test]
    fn test_output_enum_error() {
        let output = Output::Error(ErrorOutput::error("テストエラー"));
        let json = output.to_json().unwrap();
        assert!(json.contains("\"status\": \"error\""));
    }
} // end mod tests
