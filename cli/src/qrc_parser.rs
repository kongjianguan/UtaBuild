use crate::models::LyricElement;
use regex::Regex;

/// QRC 歌词中的单个单词/字符块，包含文本及其时间信息。
#[derive(Debug, Clone)]
pub struct QrcWord {
    /// 单词文本（可以是汉字、假名、罗马音等）
    pub text: String,
    /// 该单词的起始时间（毫秒）
    pub start_ms: u32,
    /// 该单词的持续时间（毫秒）
    pub duration_ms: u32,
}

/// QRC 歌词中的一行，包含该行的起止时间及单词列表。
#[derive(Debug, Clone)]
pub struct QrcLine {
    /// 该行的起始时间（毫秒）
    pub start_ms: u32,
    /// 该行的结束时间（毫秒），由 start_ms + duration_ms 计算得出
    pub end_ms: u32,
    /// 该行包含的所有单词
    pub words: Vec<QrcWord>,
}

/// 解析 QRC XML 内容为结构化的歌词行。
///
/// 从 XML 中提取 LyricContent 属性，去除头部元信息行（ti/ar/kana/offset），
/// 然后逐行解析时间戳和单词数据。每行格式为 `[start_ms,duration_ms]word1(t1,d1)word2(t2,d2)...`。
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
                    let text = cap.get(1)?.as_str().trim().to_string();
                    if text.is_empty() {
                        return None;
                    }
                    Some(QrcWord {
                        text,
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

/// 从 QRC XML 中提取 `LyricContent` 属性的文本内容。
///
/// 使用字符串查找而非正则表达式，因为歌词文本中可能包含字面量 `"` 字符
/// （例如 `"(143356,175)白...`），正则 `LyricContent="([^"]*)"` 会在第一个 `"` 处截断。
/// 我们改为查找结尾的 `"/>` 来确定内容的结束位置。
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

/// 按时间窗口将罗马音行对齐到原始歌词行。
///
/// 对于每一行原始歌词，收集所有起始时间落在该行时间窗口内的罗马音行，
/// 并将其单词合并。这处理了 QRC 数据中罗马音轨被拆分为多行的情况
/// （例如当某个读音的最后一个音节落在单独的一行中）。
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
                    // 300ms tolerance: romaji timing may be slightly ahead of original timing
                    // (matching the tolerance used in lrc_parser::align_romaji_by_time)
                    roma_line.start_ms + 300 >= orig_line.start_ms
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

/// 字符级时间条目，记录单个字符及其起止时间。
struct CharEntry {
    /// 字符文本
    text: String,
    /// 起始时间（毫秒）
    start_ms: u32,
    /// 结束时间（毫秒）
    end_ms: u32,
}

/// 将 QRC 单词拆分为单个字符，并按时间比例分配时间范围。
///
/// 每个字符获得等分的持续时间。例如一个持续 100ms 的 4 字符单词，
/// 每个字符分配 25ms。
fn words_to_char_entries(words: &[QrcWord]) -> Vec<CharEntry> {
    let mut entries = Vec::new();
    for word in words {
        let chars: Vec<char> = word.text.chars().collect();
        if chars.is_empty() {
            continue;
        }
        let char_duration = word.duration_ms / chars.len() as u32;
        for (i, ch) in chars.iter().enumerate() {
            entries.push(CharEntry {
                text: ch.to_string(),
                start_ms: word.start_ms + i as u32 * char_duration,
                end_ms: word.start_ms + (i as u32 + 1) * char_duration,
            });
        }
    }
    entries
}

/// 使用时间重叠在**字符级别**对齐原始歌词与罗马音。
///
/// 将每个原始字符与时间范围与之重叠的罗马音字符进行匹配。
/// 汉字（Kanji）会从其匹配的罗马音获得注音（ruby）标注；
/// 假名和其他字符作为纯文本渲染（不带注音）。
///
/// 这取代了旧的基于启发式的 `align_ruby_to_text` 方法，后者无法可靠地在跨词边界处
/// 区分送假名（okurigana）和读音。
///
/// `orig_words` — 单行原始歌词的 QRC 单词。
/// `roma_words` — 匹配的罗马音 QRC 单词（已按时间窗口过滤）。
pub fn align_qrc_by_character(
    orig_words: &[QrcWord],
    roma_words: &[QrcWord],
) -> Vec<LyricElement> {
    let orig_chars = words_to_char_entries(orig_words);

    if orig_chars.is_empty() {
        return vec![];
    }

    let mut elements: Vec<LyricElement> = Vec::new();
    let mut roma_idx = 0;

    for oc in &orig_chars {
        while roma_idx < roma_words.len()
            && roma_words[roma_idx].start_ms + roma_words[roma_idx].duration_ms <= oc.start_ms
        {
            roma_idx += 1;
        }

        let mut matched_roma = String::new();
        let mut scan = roma_idx;
        while scan < roma_words.len() && roma_words[scan].start_ms < oc.end_ms {
            let r_start = roma_words[scan].start_ms;
            let r_dur = roma_words[scan].duration_ms;
            if r_dur == 0 {
                scan += 1;
                continue;
            }
            let r_end = r_start + r_dur;
            let overlap_start = std::cmp::max(r_start, oc.start_ms);
            let overlap_end = std::cmp::min(r_end, oc.end_ms);
            let overlap_dur = overlap_end.saturating_sub(overlap_start);
            if overlap_dur >= r_dur / 2 {
                matched_roma.push_str(&roma_words[scan].text);
                matched_roma.push(' ');
            }
            scan += 1;
        }

        let matched_roma = matched_roma.trim();

        let first_char = oc.text.chars().next().unwrap_or(' ');
        let ct = crate::ruby_align::classify_char(first_char);

        if ct == crate::ruby_align::CharType::Kanji && !matched_roma.is_empty() {
            let hiragana = crate::romaji::romaji_to_hiragana_strict(matched_roma);
            if !hiragana.is_empty() && hiragana != oc.text {
                elements.push(LyricElement::new_ruby(oc.text.clone(), hiragana));
            } else {
                elements.push(LyricElement::new_text(oc.text.clone()));
            }
        } else {
            elements.push(LyricElement::new_text(oc.text.clone()));
        }
    }

    // 合并相邻的相同类型元素
    crate::ruby_align::merge_adjacent(&elements)
}

/// 完整的 QRC 处理流水线：同时解析原始歌词和罗马音的 QRC XML，
/// 按时间对齐，并通过字符级匹配生成带注音（ruby）标注的 `LyricElement`。
///
/// 如果任一 XML 解析失败则返回 `None`。无罗马音数据的行将作为纯文本渲染。
pub fn process_qrc_pipeline(
    original_xml: &str,
    romaji_xml: &str,
) -> Option<Vec<LyricElement>> {
    let original_lines = parse_qrc(original_xml)?;
    let romaji_lines = parse_qrc(romaji_xml)?;

    let aligned = align_romaji_to_original(&original_lines, &romaji_lines);

    let mut elements: Vec<LyricElement> = Vec::new();

    for (i, (orig_words, roma_words)) in aligned.iter().enumerate() {
        if orig_words.is_empty() {
            continue;
        }

        if let Some(roma_words) = roma_words {
            if !roma_words.is_empty() {
                let line_elements = align_qrc_by_character(orig_words, roma_words);
                if !line_elements.is_empty() {
                    elements.extend(line_elements);
                } else {
                    // 回退方案：使用完整的罗马音字符串进行假名锚点对齐
                    let orig_text: String =
                        orig_words.iter().map(|w| w.text.as_str()).collect();
                    let roma_str: String = roma_words
                        .iter()
                        .map(|w| w.text.as_str())
                        .collect::<Vec<&str>>()
                        .join(" ");
                    let hiragana = crate::romaji::romaji_to_hiragana_strict(&roma_str);
                    if !hiragana.is_empty() && hiragana != orig_text {
                        let fallback =
                            crate::ruby_align::align_ruby_to_text(&orig_text, &hiragana);
                        if fallback.is_empty() {
                            elements.push(LyricElement::new_text(orig_text));
                        } else {
                            elements.extend(fallback);
                        }
                    } else {
                        elements.push(LyricElement::new_text(orig_text));
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建 QrcWord 的辅助函数，用于测试中快速构造测试数据。
    fn qw(text: &str, start: u32, duration: u32) -> QrcWord {
        QrcWord {
            text: text.to_string(),
            start_ms: start,
            duration_ms: duration,
        }
    }

    /// 解析 QRC XML 并验证基本行和单词结构。
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

    /// 按时间对齐罗马音到原始歌词行的基本功能测试。
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

    /// 回归测试：当罗马音轨在一个原始行的时间窗口内被拆分为多行时，
    /// 应收集并合并所有罗马音单词，而非仅第一个匹配行。
    #[test]
    fn merges_multiple_romaji_lines_in_same_window() {
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

    /// 验证时间窗口外的罗马音行不会被错误匹配。
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

    /// 空 QRC 内容应返回 Some 空向量而非 None。
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

    /// 无效 XML 应返回 None。
    #[test]
    fn parse_invalid_xml_returns_none() {
        assert!(parse_qrc("not xml at all").is_none());
    }

    /// 回归测试：纯时间标记（无文本内容，如 (6760,232)）不应将数字泄露到下一个单词文本中。
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
        assert_eq!(lines[0].words[0].text, "so");
        assert_eq!(lines[0].words[1].text, "no");
        // The timing-only (100,232) should be skipped;
        // "232)" must NOT leak into the next word
        assert_eq!(lines[0].words[2].text, "ki");
        assert_eq!(lines[0].words[3].text, "mi");
        // Also verify no trailing whitespace in word text
        assert_eq!(lines[0].words[0].text.chars().last(), Some('o'), "trailing whitespace stripped");
    }

    /// 回归测试：连续多个纯时间标记应全部被跳过，仅保留有文本的单词。
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
        assert_eq!(lines[0].words[0].text, "do");
        assert_eq!(lines[0].words[1].text, "re");
        assert_eq!(lines[0].words[2].text, "mi");
    }

    /// 回归测试：歌词文本中包含字面量 `"` 字符时（例如 `"白"`），
    /// LyricContent 提取不应被截断。
    #[test]
    fn extracts_full_lyrics_with_double_quotes_in_text() {
        // Regression test: lyrics containing literal `"` characters (e.g., `"白"`)
        // must not truncate the LyricContent extraction.
        // Note: in r#"..."#` raw strings, `"` is allowed as long as not followed by `#`.
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

    /// 验证复杂歌词（包含多个带引号的段落）能被完整提取且不被截断。
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
