//! HTML 输出模块：将歌词数据渲染为自包含的 HTML 页面。
//!
//! HTML 模板来自 `cli/src/templates/lyrics.html`，
//! 在编译期通过 `include_str!` 嵌入。占位符在运行时替换。

use crate::models::LyricElement;

/// 将 UtaTen 相对路径补全为完整 URL。
///
/// 只允许 http(s)、UtaTen 相对路径和 `ne:` / `qq_music:` 内部标识——
/// `javascript:` / `data:` / `vbscript:` 等危险协议原样透传会在导出的
/// HTML 中形成可点击的脚本入口，因此一律清空。
fn resolve_url(url: &str) -> String {
    if url.starts_with("/lyric/") {
        format!("https://utaten.com{}", url)
    } else if url.starts_with("http://") || url.starts_with("https://")
        || url.starts_with("ne:") || url.starts_with("qq_music:")
    {
        url.to_string()
    } else {
        String::new()
    }
}

/// HTML 实体转义：将特殊字符替换为对应的 HTML 实体。
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// 将歌词元素数组渲染为 HTML（ruby 注音 + 纯文本 + 换行）。
fn render_ruby_html(elements: &[LyricElement]) -> String {
    let mut html = String::new();
    for elem in elements {
        match elem.element_type.as_str() {
            "text" => {
                if let Some(ref base) = elem.base {
                    html.push_str(&html_escape(base));
                }
            }
            "ruby" => {
                let base = html_escape(elem.base.as_deref().unwrap_or(""));
                let ruby = html_escape(elem.ruby.as_deref().unwrap_or(""));
                html.push_str(&format!(
                    "<span class=\"ruby\"><span class=\"rb\">{}</span><span class=\"rt\">{}</span></span>",
                    base, ruby
                ));
            }
            "linebreak" => {
                html.push_str("<br>\n");
            }
            _ => {}
        }
    }
    html
}

/// 将歌词渲染为完整的 HTML 页面字符串。
///
/// # 参数
///
/// * `title` - 歌曲标题
/// * `artist` - 艺术家名称（空字符串表示未知）
/// * `lyrics_url` - 歌词来源 URL
/// * `elements` - 歌词元素切片（含 ruby 注音）
/// * `cover_url` - 可选的封面图片 URL
///
/// # 返回值
///
/// 返回一个自包含的 HTML 页面字符串。
pub fn render_lyrics_html(
    title: &str,
    artist: &str,
    lyrics_url: &str,
    elements: &[LyricElement],
    cover_url: Option<&str>,
) -> String {
    let template = include_str!("templates/lyrics.html");
    let ruby_html = render_ruby_html(elements);

    let (cover_class, cover_style) = match cover_url {
        Some(url) if !url.trim().is_empty() => (
            "has-cover",
            format!("style=\"background-image: url('{}')\"", html_escape(url)),
        ),
        _ => ("", String::new()),
    };

    let escaped_title = html_escape(title);
    let escaped_artist = html_escape(artist);
    let escaped_url = html_escape(&resolve_url(lyrics_url));
    let display_title = if title.is_empty() { "未知の曲" } else { title };
    let display_artist = if artist.is_empty() { "不明なアーティスト" } else { artist };

    template
        .replace("TITLE - ARTIST", &format!("{} - {}", escaped_title, escaped_artist))
        .replace("COVER_CLASS", cover_class)
        .replace("STYLE_COVER", &cover_style)
        .replace("LYRICS_URL", &escaped_url)
        .replace(">TITLE<", &format!(">{}<", html_escape(display_title)))
        .replace(">ARTIST<", &format!(">{}<", html_escape(display_artist)))
        .replace("RUBY_HTML", &ruby_html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a&b"), "a&amp;b");
        assert_eq!(html_escape("<tag>"), "&lt;tag&gt;");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("it's"), "it&#39;s");
        assert_eq!(html_escape("normal"), "normal");
    }

    #[test]
    fn test_render_ruby_html_text() {
        let elements = vec![LyricElement {
            element_type: "text".to_string(),
            base: Some("こんにちは".to_string()),
            ruby: None,
        }];
        let html = render_ruby_html(&elements);
        assert_eq!(html, "こんにちは");
    }

    #[test]
    fn test_render_ruby_html_ruby() {
        let elements = vec![LyricElement {
            element_type: "ruby".to_string(),
            base: Some("私".to_string()),
            ruby: Some("わたし".to_string()),
        }];
        let html = render_ruby_html(&elements);
        assert!(html.contains("class=\"ruby\""));
        assert!(html.contains("class=\"rb\""));
        assert!(html.contains("私"));
        assert!(html.contains("class=\"rt\""));
        assert!(html.contains("わたし"));
    }

    #[test]
    fn test_render_ruby_html_linebreak() {
        let elements = vec![
            LyricElement {
                element_type: "text".to_string(),
                base: Some("line1".to_string()),
                ruby: None,
            },
            LyricElement {
                element_type: "linebreak".to_string(),
                base: None,
                ruby: None,
            },
            LyricElement {
                element_type: "text".to_string(),
                base: Some("line2".to_string()),
                ruby: None,
            },
        ];
        let html = render_ruby_html(&elements);
        assert!(html.contains("<br>"));
        assert!(html.contains("line1"));
        assert!(html.contains("line2"));
    }

    #[test]
    fn test_render_lyrics_html_basic() {
        let elements = vec![LyricElement {
            element_type: "text".to_string(),
            base: Some("テスト".to_string()),
            ruby: None,
        }];
        let html = render_lyrics_html("テスト曲", "テスト歌手", "https://example.com", &elements, None);
        assert!(html.contains("テスト曲"));
        assert!(html.contains("テスト歌手"));
        assert!(html.contains("https://example.com"));
        assert!(html.contains("テスト"));
        assert!(html.contains("Powered by UtaBuild"));
    }

    #[test]
    fn test_render_lyrics_html_with_cover() {
        let elements = vec![];
        let html = render_lyrics_html("曲", "歌手", "https://example.com", &elements, Some("https://img.example.com/cover.jpg"));
        assert!(html.contains("has-cover"));
        assert!(html.contains("background-image: url('https://img.example.com/cover.jpg')"));
    }

    #[test]
    fn test_render_lyrics_html_empty_title_artist() {
        let elements = vec![];
        let html = render_lyrics_html("", "", "https://example.com", &elements, None);
        assert!(html.contains("未知の曲"));
        assert!(html.contains("不明なアーティスト"));
    }

    #[test]
    fn test_render_lyrics_html_xss_prevention() {
        let elements = vec![LyricElement {
            element_type: "text".to_string(),
            base: Some("<script>alert('xss')</script>".to_string()),
            ruby: None,
        }];
        let html = render_lyrics_html("<b>Title</b>", "&Artist", "https://x.com?a=1&b=2", &elements, None);
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<b>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&lt;b&gt;"));
        assert!(html.contains("&amp;Artist"));
        assert!(html.contains("a=1&amp;b=2"));
    }

    #[test]
    fn test_resolve_url_utaten_relative() {
        assert_eq!(
            resolve_url("/lyric/rq20031716/"),
            "https://utaten.com/lyric/rq20031716/"
        );
    }

    #[test]
    fn test_resolve_url_absolute() {
        assert_eq!(
            resolve_url("https://example.com/song"),
            "https://example.com/song"
        );
    }

    #[test]
    fn test_resolve_url_qq_music_unchanged() {
        assert_eq!(
            resolve_url("qq_music:003WFMXk4O5ywc"),
            "qq_music:003WFMXk4O5ywc"
        );
    }

    #[test]
    fn test_render_lyrics_html_utaten_url_resolved() {
        let elements = vec![];
        let html = render_lyrics_html("テスト曲", "歌手", "/lyric/rq20031716/", &elements, None);
        assert!(html.contains("https://utaten.com/lyric/rq20031716/"));
    }
}
