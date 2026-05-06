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
    let word_re = Regex::new(r"([^(,)]+)\((\d+),(\d+)\)").ok()?;

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
    // Use string-based extraction instead of regex because the lyric content
    // may contain literal `"` characters (e.g., `"(143356,175)白...`).
    // The regex `LyricContent="([^"]*)"` would stop at the first `"` inside lyrics,
    // truncating everything after it. We instead look for the closing `"/>`.
    let prefix = "LyricContent=\"";
    let start = xml.find(prefix)? + prefix.len();
    let end = xml[start..].find("\"/>")?;
    Some(xml[start..start + end].to_string())
}

/// Align romaji lines to original lyric lines by time window.
/// For each original line, collects ALL romaji lines whose start time falls
/// within the original line's time window and merges their words.
/// This handles QRC data where the romaji track is split across multiple
/// lines (e.g. when the final syllable of a reading falls in a separate line).
pub fn align_romaji_to_original(
    original: &[QrcLine],
    romaji: &[QrcLine],
) -> Vec<(Vec<QrcWord>, Option<Vec<QrcWord>>)> {
    original
        .iter()
        .map(|orig_line| {
            let matching_roma_words: Vec<QrcWord> = romaji
                .iter()
                .filter(|roma_line| {
                    roma_line.start_ms >= orig_line.start_ms
                        && roma_line.start_ms < orig_line.end_ms
                })
                .flat_map(|roma_line| roma_line.words.clone())
                .collect();
            let roma = if matching_roma_words.is_empty() {
                None
            } else {
                Some(matching_roma_words)
            };
            (orig_line.words.clone(), roma)
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
    fn merges_multiple_romaji_lines_in_same_window() {
        // Regression: when the romaji track splits a reading across multiple
        // lines within one original line's time window, all romaji words
        // should be collected and merged (not just the first matching line).
        let original_lines = vec![QrcLine {
            start_ms: 0,
            end_ms: 500,
            words: vec![qw("救", 0, 50), qw("済", 100, 50)],
        }];
        let romaji_lines = vec![
            QrcLine {
                start_ms: 0,
                end_ms: 300,
                words: vec![qw("kyu", 0, 50), qw("u", 100, 50), qw("sa", 200, 50)],
            },
            QrcLine {
                start_ms: 350,
                end_ms: 500,
                words: vec![qw("i", 350, 50)],
            },
        ];

        let aligned = align_romaji_to_original(&original_lines, &romaji_lines);
        assert_eq!(aligned.len(), 1);
        let (_, roma_words) = &aligned[0];
        assert!(roma_words.is_some(), "should find merged romaji words");
        let words = roma_words.as_ref().unwrap();
        assert_eq!(words.len(), 4, "should have 4 words: kyu, u, sa, i");
        let romaji_str: String = words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
        assert_eq!(romaji_str, "kyu u sa i", "all four syllables should be merged");
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

    #[test]
    fn skips_timing_only_entries_without_leaking_numbers_into_word_text() {
        // Regression test: timing-only entries like (6760,232) with no text
        // before '(' must not leak "232)" into the next word's text.
        let qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[ti:Test]
[ar:Test]
[offset:0]
[0,1000]so (0,50)no (51,50)(100,232)ki (332,50)mi (382,50)
"/>
  </LyricInfo>
</QrcInfos>"#;
        let lines = parse_qrc(qrc).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].words.len(), 4);
        assert_eq!(lines[0].words[0].text, "so ");
        assert_eq!(lines[0].words[1].text, "no ");
        // The timing-only (100,232) should be skipped;
        // "232)" must NOT leak into the next word
        assert_eq!(lines[0].words[2].text, "ki ");
        assert_eq!(lines[0].words[3].text, "mi ");
    }

    #[test]
    fn skips_multiple_consecutive_timing_only_entries() {
        let qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[ti:Test]
[offset:0]
[0,500]do (0,50)(100,50)(200,50)re (300,50)mi (350,50)
"/>
  </LyricInfo>
</QrcInfos>"#;
        let lines = parse_qrc(qrc).unwrap();
        assert_eq!(lines[0].words.len(), 3);
        assert_eq!(lines[0].words[0].text, "do ");
        assert_eq!(lines[0].words[1].text, "re ");
        assert_eq!(lines[0].words[2].text, "mi ");
    }

    #[test]
    fn extracts_full_lyrics_with_double_quotes_in_text() {
        // Regression test: lyrics containing literal `"` characters (e.g., `"白"`)
        // must not truncate the LyricContent extraction.
        // Note: in r#"..."# raw strings, `"` is allowed as long as not followed by `#`.
        let qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[ti:Test]
[ar:Test]
[0,100]foo(0,50)
[100,200]"(100,50)bar(150,50)
[200,300]baz(200,50)
"/>
  </LyricInfo>
</QrcInfos>"#;
        let lines = parse_qrc(qrc).unwrap();
        assert_eq!(lines.len(), 3, "should parse all 3 lines, not truncated");
        assert_eq!(lines[0].words[0].text, "foo");
        assert_eq!(lines[1].words[0].text, "\u{22}");
        assert_eq!(lines[1].words[1].text, "bar");
        assert_eq!(lines[2].words[0].text, "baz");
    }

    #[test]
    fn extracts_full_kishikaisen_lyrics() {
        // Verify that the full lyrics are correctly extracted when
        // `"` characters appear in lyric text (e.g., `"白"は...`).
        let qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[ti:Test]
[ar:Test]
[0,100]a(0,50)
[100,200]b(100,50)c(150,50)
[200,100]d(200,50)
[300,200]e(300,50)f(350,50)
"/>
  </LyricInfo>
</QrcInfos>"#;
        let lines = parse_qrc(qrc).unwrap();
        assert_eq!(lines.len(), 4, "all 4 lines must be parsed, none truncated");
        assert_eq!(lines[3].words[0].text, "e");
        assert_eq!(lines[3].words[1].text, "f");
    }
}
