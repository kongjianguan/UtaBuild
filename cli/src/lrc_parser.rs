//! LRC / YRC lyric parser with romaji-based ruby annotation.
//!
//! Parses LRC (standard `[mm:ss.xx]text`) and YRC (NetEase word-timed
//! `[start,duration]text(word_start,word_dur,0)word...`) formats.
//!
//! When romaji LRC data is available (NetEase `romalrc` field), aligns
//! it with the original lyrics by time window and produces ruby-annotated
//! `Vec<LyricElement>` via the romaji→hiragana→ruby_align pipeline.

use crate::models::LyricElement;

/// Parse original lyrics (YRC preferred, LRC fallback) with optional
/// romaji LRC data to produce ruby-annotated LyricElements.
///
/// Returns plain text elements if romaji is empty; otherwise returns
/// ruby-annotated elements by aligning romaji to original lines.
pub fn parse_lyrics_with_ruby(
    yrc: Option<&str>,
    lrc: Option<&str>,
    romalrc: Option<&str>,
) -> Vec<LyricElement> {
    // Parse original into timed lines
    let orig_timed: Vec<(u64, String)> = if let Some(y) = yrc.filter(|s| !s.is_empty()) {
        parse_yrc_timed(y)
    } else if let Some(l) = lrc.filter(|s| !s.is_empty()) {
        parse_lrc_timed(l)
    } else {
        return vec![];
    };

    // No romaji → plain text output
    let roma = match romalrc.filter(|s| !s.is_empty()) {
        Some(r) => r,
        None => return timed_to_elements(&orig_timed),
    };

    let roma_timed = parse_lrc_timed(roma);
    if roma_timed.is_empty() {
        return timed_to_elements(&orig_timed);
    }

    // Align romaji to original by time window and produce ruby elements
    build_ruby_elements(&orig_timed, &roma_timed)
}

/// 将带时间戳的条目转换为纯文本 LyricElement。
/// 相邻行之间插入换行符，跳过空文本行。
/// Convert timed entries to plain-text LyricElements.
fn timed_to_elements(timed: &[(u64, String)]) -> Vec<LyricElement> {
    let mut elements = Vec::new();
    for (i, (_, text)) in timed.iter().enumerate() {
        if text.is_empty() {
            continue;
        }
        elements.push(LyricElement::new_text(text.clone()));
        if i + 1 < timed.len() {
            elements.push(LyricElement::new_linebreak());
        }
    }
    elements
}

/// 通过将 romaji 对齐到原始歌词行并执行 romaji → 平假名 → ruby 对齐管线来构建 ruby 注音元素。
/// Build ruby-annotated elements by aligning romaji to original lines
/// and running the romaji→hiragana→ruby_align pipeline.
fn build_ruby_elements(
    orig: &[(u64, String)],
    roma: &[(u64, String)],
) -> Vec<LyricElement> {
    let aligned_roma = align_romaji_by_time(orig, roma);
    let mut elements = Vec::new();

    for (i, (orig_text, roma_text)) in aligned_roma.iter().enumerate() {
        if orig_text.is_empty() {
            continue;
        }

        if let Some(roma_str) = roma_text {
            let hiragana = crate::romaji::romaji_to_hiragana_strict(roma_str);
            if !hiragana.is_empty() && hiragana != orig_text.as_str() {
                let line_elements =
                    crate::ruby_align::align_ruby_to_text(orig_text, &hiragana);
                elements.extend(line_elements);
            } else {
                elements.push(LyricElement::new_text(orig_text.clone()));
            }
        } else {
            elements.push(LyricElement::new_text(orig_text.clone()));
        }

        if i + 1 < aligned_roma.len() {
            elements.push(LyricElement::new_linebreak());
        }
    }

    elements
}

/// 计算 YRC 原始歌词与 romalrc 之间的系统时间偏移量。
///
/// 网易云音乐的 YRC 和 romalrc 时间戳通常来自不同的编码管线，可能存在数百毫秒的系统误差。
/// 该函数估算中位数偏移量，使 `align_romaji_by_time` 能够校正它，防止行级联对齐错误。
///
/// 正偏移表示 romaji 时间戳超前于原始歌词（romaji 起始时间早于对应 YRC 行）；负偏移表示落后。
/// Compute the systemic time offset between YRC original lyrics and romalrc.
///
/// NetEase YRC and romalrc timestamps often come from different encoding
/// pipelines and can have a systematic offset of several hundred milliseconds.
/// This function estimates the median offset so `align_romaji_by_time` can
/// correct it, preventing cascading line misalignment.
///
/// Positive offset means romaji timestamps are ahead of original (romaji
/// starts earlier than the corresponding YRC line); negative means behind.
fn compute_systemic_offset(
    orig: &[(u64, String)],
    roma: &[(u64, String)],
) -> i64 {
    if orig.len() < 2 || roma.is_empty() {
        return 0;
    }

    let mut offsets: Vec<i64> = Vec::new();
    let mut roma_idx = 0;

    for (i, (orig_start, _)) in orig.iter().enumerate() {
        let win_end = if i + 1 < orig.len() {
            orig[i + 1].0
        } else {
            u64::MAX
        };

        // Skip romaji that's more than 2s before this line
        while roma_idx < roma.len() && roma[roma_idx].0 + 2000 < *orig_start {
            roma_idx += 1;
        }

        // If there's a romaji line within this window, record the offset
        if roma_idx < roma.len() && roma[roma_idx].0 < win_end {
            let diff = roma[roma_idx].0 as i64 - *orig_start as i64;
            offsets.push(diff);
            // Advance past this romaj line for the next iteration
            roma_idx += 1;
        }
    }

    if offsets.is_empty() {
        return 0;
    }

    // Use median for robustness against outlier lines
    offsets.sort();
    offsets[offsets.len() / 2]
}

/// 通过时间窗口将 romaji 行对齐到原始歌词行（类似 Lyrico 的 `lyricsMerge`）。
///
/// 每个原始歌词行收集其时间窗口内（当前行起始到下一行起始）的 ALL romaji 行，然后用空格连接。
/// 这处理了网易云 `romalrc` 数据中单个 YRC 行的读音可能分散在多行 LRC 中的情况。
///
/// 对齐之前，检测并校正 YRC 与 romalrc 轨道之间的系统时间偏移（参见 `compute_systemic_offset`）。
/// Align romaji lines to original lines by time window (like Lyrico's `lyricsMerge`).
///
/// Each original line accumulates ALL romaji lines whose timestamps fall
/// within the original line's time window (current line start to next line
/// start), then joins them with spaces. This handles NetEase `romalrc` data
/// where a single YRC line's reading may be split across multiple LRC lines.
///
/// Before alignment, detects and corrects systemic time offset between
/// the YRC and romalrc tracks (see `compute_systemic_offset`).
fn align_romaji_by_time(
    orig: &[(u64, String)],
    roma: &[(u64, String)],
) -> Vec<(String, Option<String>)> {
    // Step 0: detect and correct systemic time offset
    let offset = compute_systemic_offset(orig, roma);
    let adjusted_roma: Vec<(u64, String)> = roma
        .iter()
        .map(|(t, s)| {
            if offset >= 0 {
                // Romaji is ahead: subtract offset to align
                (t.saturating_sub(offset as u64), s.clone())
            } else {
                // Romaji is behind: add offset to align
                (t.saturating_add((-offset) as u64), s.clone())
            }
        })
        .collect();

    let mut result = Vec::with_capacity(orig.len());
    let mut roma_idx = 0;

    for (i, (orig_start, orig_text)) in orig.iter().enumerate() {
        let win_end = if i + 1 < orig.len() {
            orig[i + 1].0
        } else {
            u64::MAX
        };

        let mut matched_texts: Vec<String> = Vec::new();

        while roma_idx < adjusted_roma.len() {
            let (roma_start, roma_text) = &adjusted_roma[roma_idx];

            // Too early — skip (with tolerance for residual jitter)
            if *roma_start + 500 < *orig_start {
                roma_idx += 1;
                continue;
            }

            // Past the window — stop
            if *roma_start >= win_end {
                break;
            }

            // Within window — accumulate ALL romaji lines (not just the first)
            matched_texts.push(roma_text.clone());
            roma_idx += 1;
        }

        let matched = if matched_texts.is_empty() {
            None
        } else {
            Some(matched_texts.join(" "))
        };

        result.push((orig_text.clone(), matched));
    }

    result
}

/// 将 LRC 文本解析为按时间戳排序的 `(timestamp_ms, text)` 列表。
/// 支持 `[mm:ss.xx]` 和 `[mm:ss.xxx]` 两种时间格式。
/// Parse LRC text into a sorted list of `(timestamp_ms, text)` pairs.
pub fn parse_lrc_timed(lrc: &str) -> Vec<(u64, String)> {
    let re = regex::Regex::new(r"\[(\d{2}):(\d{2})\.(\d{2,3})\]").unwrap();
    let mut entries: Vec<(u64, String)> = Vec::new();

    for line in lrc.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut timestamps: Vec<u64> = Vec::new();
        let mut last_end = 0;
        for caps in re.captures_iter(line) {
            let min: u64 = caps[1].parse().unwrap_or(0);
            let sec: u64 = caps[2].parse().unwrap_or(0);
            let ms_str = &caps[3];
            let ms: u64 = if ms_str.len() == 2 {
                ms_str.parse::<u64>().unwrap_or(0) * 10
            } else {
                ms_str.parse().unwrap_or(0)
            };
            timestamps.push(min * 60000 + sec * 1000 + ms);
            last_end = caps.get(0).map(|m| m.end()).unwrap_or(0);
        }
        let content = line[last_end..].trim().to_string();

        for ts in timestamps {
            entries.push((ts, content.clone()));
        }
    }

    entries.sort_by_key(|(ts, _)| *ts);
    entries
}

/// 将 LRC 字符串解析为纯文本 `LyricElement` 行序列。
/// Parse an LRC string into plain-text `LyricElement` lines.
pub fn parse_lrc_to_elements(lrc: &str) -> Vec<LyricElement> {
    let timed = parse_lrc_timed(lrc);
    timed_to_elements(&timed)
}

/// 将 YRC 解析为带时间戳的 `(start_ms, text)` 对。
/// 去除 YRC 单词时间标记 `(start,dur,flag)`，只保留单词文本。
/// 同时支持 `text(markers)` 和 `(markers)text` 两种格式。
/// Parse YRC into timed `(start_ms, text)` pairs.
/// YRC word timing markers `(start,dur,flag)` are stripped; only the
/// word text is kept. Handles both `text(markers)` and `(markers)text` formats.
pub fn parse_yrc_timed(yrc: &str) -> Vec<(u64, String)> {
    let line_re = regex::Regex::new(r"^\[(\d+),(\d+)\](.*)$").unwrap();
    // Match word timing and capture the following text (Lyrico format)
    let word_re = regex::Regex::new(r"\(\d+,\d+,\d+\)([^()]*)").unwrap();
    // Also match text preceding timing markers (e.g., 鮮(0,50,0))
    let prefix_word_re = regex::Regex::new(r"([^()]+)\(\d+,\d+,\d+\)").unwrap();
    let mut entries: Vec<(u64, String)> = Vec::new();

    for line in yrc.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(caps) = line_re.captures(line) {
            let start: u64 = caps[1].parse().unwrap_or(0);
            let content = caps.get(3).map(|m| m.as_str()).unwrap_or("");

            // Try text(timing) format first (e.g., 鮮(0,50,0))
            let mut text: String = prefix_word_re
                .captures_iter(content)
                .filter_map(|c| c.get(1))
                .map(|m| m.as_str())
                .collect();

            // If empty, try Lyrico format: (timing)text
            if text.is_empty() {
                text = word_re
                    .captures_iter(content)
                    .filter_map(|c| c.get(1))
                    .map(|m| m.as_str())
                    .collect();
            }

            // Also capture any trailing text after the last timing marker
            // (e.g., "fg" in "abc(0,100,0)de(101,200,0)fg")
            if !text.is_empty() {
                let trailing = regex::Regex::new(r"\)([^()]+)$")
                    .unwrap()
                    .captures(content)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str())
                    .unwrap_or("");
                text.push_str(trailing);
            }

            // Fallback: strip all timing markers and keep remaining chars
            if text.is_empty() {
                let stripped = regex::Regex::new(r"\(\d+,\d+,\d+\)")
                    .unwrap()
                    .replace_all(content, "");
                text = stripped.trim().to_string();
            }

            if !text.is_empty() {
                entries.push((start, text));
            }
        }
    }

    entries.sort_by_key(|(ts, _)| *ts);
    entries
}

/// 将 YRC 字符串解析为纯文本 `LyricElement` 行序列。
/// Parse a YRC string into plain-text `LyricElement` lines.
pub fn parse_yrc_to_elements(yrc: &str) -> Vec<LyricElement> {
    let timed = parse_yrc_timed(yrc);
    timed_to_elements(&timed)
}

/// 使用 romaji LRC 解析原始歌词（优先 YRC），通过将 romaji 令牌分发到每行中的汉字块来生成 ruby 注音。
///
/// 与 `parse_lyrics_with_ruby`（将整个平假名字符串传递给 `align_ruby_to_text`，在词边界处会错误对齐）不同，该函数：
/// 1. 将原始文本拆分为汉字块
/// 2. 将空格分隔的 romaji 令牌分发到这些块中
/// 3. 为每个汉字块分配其比例的读音
///
/// 假名块（送假名、助词等）作为纯文本渲染，不带 ruby 注音。
/// Parse original lyrics (YRC preferred) with romaji LRC to produce ruby
/// annotations by distributing romaji tokens among kanji blocks within each line.
///
/// Unlike `parse_lyrics_with_ruby` which passes the full hiragana string to
/// `align_ruby_to_text` (a heuristic that misaligns across word boundaries),
/// this function:
/// 1. Splits the original text into kanji blocks
/// 2. Distributes space-separated romaji tokens among those blocks
/// 3. Assigns each kanji block its proportional share of the reading
///
/// Kana blocks (okurigana, particles) are rendered as plain text without ruby.
pub fn parse_lyrics_with_ruby_tokenized(
    yrc: Option<&str>,
    lrc: Option<&str>,
    romalrc: Option<&str>,
) -> Vec<LyricElement> {
    // Parse original into timed lines
    let orig_timed: Vec<(u64, String)> = if let Some(y) = yrc.filter(|s| !s.is_empty()) {
        parse_yrc_timed(y)
    } else if let Some(l) = lrc.filter(|s| !s.is_empty()) {
        parse_lrc_timed(l)
    } else {
        return vec![];
    };

    // No romaji → plain text output
    let roma = match romalrc.filter(|s| !s.is_empty()) {
        Some(r) => r,
        None => return timed_to_elements(&orig_timed),
    };

    let roma_timed = parse_lrc_timed(roma);
    if roma_timed.is_empty() {
        return timed_to_elements(&orig_timed);
    }

    // Align romaji to original by time window (same as build_ruby_elements)
    let aligned = align_romaji_by_time(&orig_timed, &roma_timed);
    let mut elements = Vec::new();

    for (i, (orig_text, roma_text)) in aligned.iter().enumerate() {
        if orig_text.is_empty() {
            continue;
        }

        if let Some(roma_str) = roma_text {
            let tokens: Vec<&str> = roma_str.split_whitespace().collect();
            if tokens.is_empty() {
                elements.push(LyricElement::new_text(orig_text.clone()));
            } else {
                // Split original into kanji and kana blocks
                let blocks = split_kanji_kana_blocks(orig_text);
                // Count kanji blocks for token distribution
                let kanji_blocks: Vec<&Block> = blocks.iter().filter(|b| b.is_kanji).collect();
                let kanji_count = kanji_blocks.len();

                if kanji_count == 0 || tokens.len() < kanji_count {
                    // No kanji or insufficient tokens → plain text
                    elements.push(LyricElement::new_text(orig_text.clone()));
                } else {
                    // Distribute tokens proportionally among kanji blocks
                    let base = tokens.len() / kanji_count;
                    let extra = tokens.len() % kanji_count;

                    let mut token_idx = 0;
                    for block in &blocks {
                        if block.is_kanji {
                            let n = if kanji_blocks.len() > 1 {
                                // Distribute: first `extra` blocks get base+1, rest get base
                                let position = kanji_blocks.iter().position(|b| b.start == block.start).unwrap_or(0);
                                if position < extra { base + 1 } else { base }
                            } else {
                                // Single kanji block → all tokens
                                tokens.len() - token_idx
                            };
                            let reading: String = tokens[token_idx..token_idx + n].join(" ");
                            let hiragana = crate::romaji::romaji_to_hiragana_strict(&reading);
                            token_idx += n;

                            if !hiragana.is_empty() && hiragana != block.text {
                                elements.push(LyricElement::new_ruby(block.text.clone(), hiragana));
                            } else {
                                elements.push(LyricElement::new_text(block.text.clone()));
                            }
                        } else {
                            elements.push(LyricElement::new_text(block.text.clone()));
                        }
                    }

                    // Any remaining original text (shouldn't happen)
                    if token_idx < tokens.len() {
                        let rest: String = tokens[token_idx..].join(" ");
                        let hiragana = crate::romaji::romaji_to_hiragana_strict(&rest);
                        if !hiragana.is_empty() {
                            elements.push(LyricElement::new_text(hiragana));
                        }
                    }
                }
            }
        } else {
            elements.push(LyricElement::new_text(orig_text.clone()));
        }

        if i + 1 < aligned.len() {
            elements.push(LyricElement::new_linebreak());
        }
    }

    // Merge adjacent same-type elements
    crate::ruby_align::merge_adjacent(&elements)
}

/// 表示文本中的一个块：连续的汉字序列或连续的非汉字序列。
/// 包含文本内容、是否为汉字标记以及在原字符串中的起始位置。
#[derive(Debug)]
struct Block {
    /// 块的文本内容
    text: String,
    /// 是否为汉字块（假若为 false，则表示假名、标点或其他非汉字字符）
    is_kanji: bool,
    /// 块在原始字符串中的字节起始位置
    start: usize,
}

/// 将文本拆分为连续的汉字块和连续的非汉字块。
/// 用于将 romaji 令牌分发到每个汉字块以生成精确的 ruby 注音。
/// Split text into blocks of consecutive kanji and consecutive non-kanji.
fn split_kanji_kana_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut current_is_kanji = false;
    let mut start = 0;

    for (i, ch) in text.char_indices() {
        let ct = crate::ruby_align::classify_char(ch);
        let is_kanji = ct == crate::ruby_align::CharType::Kanji;

        if current.is_empty() {
            current.push(ch);
            current_is_kanji = is_kanji;
            start = i;
        } else if is_kanji == current_is_kanji {
            current.push(ch);
        } else {
            blocks.push(Block {
                text: current.clone(),
                is_kanji: current_is_kanji,
                start,
            });
            current = ch.to_string();
            current_is_kanji = is_kanji;
            start = i;
        }
    }

    if !current.is_empty() {
        blocks.push(Block {
            text: current,
            is_kanji: current_is_kanji,
            start,
        });
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_lrc() {
        let lrc = "[00:05.00]First line\n[00:10.00]Second line\n";
        let elements = parse_lrc_to_elements(lrc);
        let texts: Vec<&str> = elements
            .iter()
            .filter(|e| e.element_type == "text")
            .filter_map(|e| e.base.as_deref())
            .collect();
        assert_eq!(texts, vec!["First line", "Second line"]);
    }

    #[test]
    fn parses_yrc() {
        // YRC with text(timing) format: abc(0,100,0)de(101,200,0)fg
        // → extracted text: abcde + trailing fg = abcdefg
        let yrc = "[0,500]abc(0,100,0)de(101,200,0)fg\n[501,500]hij\n";
        let elements = parse_yrc_to_elements(yrc);
        let texts: Vec<&str> = elements
            .iter()
            .filter(|e| e.element_type == "text")
            .filter_map(|e| e.base.as_deref())
            .collect();
        assert_eq!(texts, vec!["abcdefg", "hij"]);
    }

    #[test]
    fn aligns_romaji_to_original() {
        let orig = vec![
            (0u64, "鮮やかなる色彩の".to_string()),
            (5000u64, "その意味など君はもう".to_string()),
        ];
        let roma = vec![
            (0u64, "a za ya ka na ru shi ki sa i no".to_string()),
            (5000u64, "so no i mi na do ki mi wa mo u".to_string()),
        ];
        let aligned = align_romaji_by_time(&orig, &roma);
        assert_eq!(aligned.len(), 2);
        assert_eq!(aligned[0].1.as_deref(), Some("a za ya ka na ru shi ki sa i no"));
        assert_eq!(aligned[1].1.as_deref(), Some("so no i mi na do ki mi wa mo u"));
    }

    #[test]
    fn parse_lyrics_with_ruby_produces_ruby_elements() {
        let yrc = Some("[0,500]鮮(0,50,0)や(51,50,0)か(102,50,0)な(153,50,0)る(204,50,0)\n");
        let romalrc = Some("[00:00.00]a za ya ka na ru\n");
        let elements = parse_lyrics_with_ruby(yrc, None, romalrc);
        let ruby_count = elements.iter().filter(|e| e.element_type == "ruby").count();
        assert!(ruby_count > 0, "should produce ruby elements from romaji, got {} elements", elements.len());
        let ruby = elements.iter().find(|e| e.base.as_deref() == Some("鮮"));
        assert!(ruby.is_some(), "should have ruby for 鮮");
        assert_eq!(ruby.unwrap().ruby.as_deref(), Some("あざ"));
    }

    #[test]
    fn empty_inputs_yield_empty() {
        let elements = parse_lyrics_with_ruby(None, None, None);
        assert!(elements.is_empty());
    }

    #[test]
    fn computes_zero_offset_when_aligned() {
        let orig = vec![
            (0u64, "line1".to_string()),
            (1000u64, "line2".to_string()),
        ];
        let roma = vec![
            (0u64, "a".to_string()),
            (1000u64, "b".to_string()),
        ];
        assert_eq!(compute_systemic_offset(&orig, &roma), 0);
    }

    #[test]
    fn offset_detects_romaji_ahead() {
        // Romaji timestamps are 500ms ahead of original (smaller timestamp)
        // diff = roma - orig = 500 - 1000 = -500
        let orig = vec![
            (1000u64, "line1".to_string()),
            (3000u64, "line2".to_string()),
        ];
        let roma = vec![
            (500u64, "ro".to_string()),
            (2500u64, "ma".to_string()),
        ];
        assert_eq!(compute_systemic_offset(&orig, &roma), -500);
    }

    #[test]
    fn offset_detects_romaji_behind() {
        // Romaji timestamps are 300ms behind original (larger timestamp)
        // diff = roma - orig = 1300 - 1000 = 300
        let orig = vec![
            (1000u64, "line1".to_string()),
            (3000u64, "line2".to_string()),
        ];
        let roma = vec![
            (1300u64, "ro".to_string()),
            (3300u64, "ma".to_string()),
        ];
        assert_eq!(compute_systemic_offset(&orig, &roma), 300);
    }

    #[test]
    fn offset_detects_mixed_scenario() {
        let orig = vec![
            (0u64, "line0".to_string()),
            (5000u64, "line1".to_string()),
            (10000u64, "line2".to_string()),
        ];
        let roma = vec![
            (300u64, "r0".to_string()),
            (4800u64, "r1".to_string()),
            (10300u64, "r2".to_string()),
        ];
        // diffs: 300-0=300, 4800-5000=-200, 10300-10000=300 → sorted [-200,300,300]
        assert_eq!(compute_systemic_offset(&orig, &roma), 300);
    }
}
