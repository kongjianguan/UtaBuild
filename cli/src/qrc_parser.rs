use regex::Regex;

#[derive(Debug, Clone)]
pub struct QrcWord {
    pub text: String,
    pub start_ms: u32,
    pub duration_ms: u32,
}

#[derive(Debug, Clone)]
pub struct QrcLine {
    pub start_ms: u32,
    pub end_ms: u32,
    pub words: Vec<QrcWord>,
}

/// Parse QRC XML content into structured lyric lines.
pub fn parse_qrc(xml: &str) -> Option<Vec<QrcLine>> {
    let content = extract_lyric_content(xml)?;

    let header_re = Regex::new(r"\[(ti|ar|kana|offset):[^\]]*\]").ok()?;
    let cleaned = header_re.replace_all(&content, "").trim().to_string();

    let line_re = Regex::new(r"\[(\d+),(\d+)\](.*)").ok()?;
    let word_re = Regex::new(r"([^(,]+)\((\d+),(\d+)\)").ok()?;

    let mut lines = Vec::new();

    for line_text in cleaned.lines() {
        let line_text = line_text.trim();
        if line_text.is_empty() {
            continue;
        }
        if let Some(caps) = line_re.captures(line_text) {
            let start_ms: u32 = caps.get(1)?.as_str().parse().ok()?;
            let duration_ms: u32 = caps.get(2)?.as_str().parse().ok()?;
            let rest = caps.get(3)?.as_str();

            let words: Vec<QrcWord> = word_re
                .captures_iter(rest)
                .filter_map(|cap| {
                    Some(QrcWord {
                        text: cap.get(1)?.as_str().to_string(),
                        start_ms: cap.get(2)?.as_str().parse().ok()?,
                        duration_ms: cap.get(3)?.as_str().parse().ok()?,
                    })
                })
                .collect();

            lines.push(QrcLine {
                start_ms,
                end_ms: start_ms + duration_ms,
                words,
            });
        }
    }

    Some(lines)
}

fn extract_lyric_content(xml: &str) -> Option<String> {
    let re = Regex::new(r#"LyricContent="([^"]*)""#).ok()?;
    let caps = re.captures(xml)?;
    Some(caps.get(1)?.as_str().to_string())
}

/// Align romaji lines to original lyric lines by time window.
/// For each original line, finds the romaji line whose start time falls within
/// the original line's time window.
pub fn align_romaji_to_original(
    original: &[QrcLine],
    romaji: &[QrcLine],
) -> Vec<(Vec<QrcWord>, Option<Vec<QrcWord>>)> {
    original
        .iter()
        .map(|orig_line| {
            let matching_roma = romaji.iter().find(|roma_line| {
                roma_line.start_ms >= orig_line.start_ms && roma_line.start_ms < orig_line.end_ms
            });
            (
                orig_line.words.clone(),
                matching_roma.map(|line| line.words.clone()),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qw(text: &str, start: u32, duration: u32) -> QrcWord {
        QrcWord {
            text: text.to_string(),
            start_ms: start,
            duration_ms: duration,
        }
    }

    #[test]
    fn parses_qrc_xml_to_lines() {
        let qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[ti:Test Song]
[ar:Test Artist]
[offset:0]
[0,405]起(0,50)死(51,50)開(102,50)戦(153,50)
[406,300]鮮(0,50)や(51,50)か(102,50)な(153,50)る(204,50)
"/>
  </LyricInfo>
</QrcInfos>"#;

        let lines = parse_qrc(qrc).unwrap();
        assert_eq!(lines.len(), 2);

        assert_eq!(lines[0].start_ms, 0);
        assert_eq!(lines[0].end_ms, 405);
        assert_eq!(lines[0].words.len(), 4);
        assert_eq!(lines[0].words[0].text, "起");

        assert_eq!(lines[1].start_ms, 406);
        assert_eq!(lines[1].end_ms, 706);
        assert_eq!(lines[1].words[0].text, "鮮");
    }

    #[test]
    fn aligns_romaji_to_original_by_time() {
        let original_lines = vec![QrcLine {
            start_ms: 0,
            end_ms: 405,
            words: vec![
                qw("起", 0, 50),
                qw("死", 51, 50),
                qw("開", 102, 50),
                qw("戦", 153, 50),
            ],
        }];
        let romaji_lines = vec![QrcLine {
            start_ms: 0,
            end_ms: 405,
            words: vec![
                qw("o", 0, 50),
                qw("ki", 51, 50),
                qw("shi", 102, 50),
                qw("ka", 153, 50),
                qw("i", 204, 50),
                qw("se", 255, 50),
                qw("n", 306, 50),
            ],
        }];

        let aligned = align_romaji_to_original(&original_lines, &romaji_lines);
        assert_eq!(aligned.len(), 1);
        let (orig_words, roma_words) = &aligned[0];
        assert_eq!(orig_words.len(), 4);
        assert!(roma_words.is_some());
    }

    #[test]
    fn skips_romaji_lines_outside_time_window() {
        let original_lines = vec![QrcLine {
            start_ms: 0,
            end_ms: 405,
            words: vec![qw("起", 0, 50)],
        }];
        let romaji_lines = vec![QrcLine {
            start_ms: 1000,
            end_ms: 1405,
            words: vec![qw("o", 0, 50)],
        }];

        let aligned = align_romaji_to_original(&original_lines, &romaji_lines);
        assert_eq!(aligned.len(), 1);
        let (_, roma_words) = &aligned[0];
        assert!(roma_words.is_none());
    }

    #[test]
    fn empty_qrc_returns_some_empty_vec() {
        let qrc = r#"<?xml version="1.0"?>
<QrcInfos>
  <LyricInfo LyricCount="0">
    <Lyric_1 LyricType="1" LyricContent="
[ti:Empty]
[ar:Nobody]
"/>
  </LyricInfo>
</QrcInfos>"#;
        let lines = parse_qrc(qrc).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn parse_invalid_xml_returns_none() {
        assert!(parse_qrc("not xml at all").is_none());
    }
}
