use crate::models::LyricElement;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharType {
    Kanji,
    Kana,
    Other,
}

#[inline]
fn classify_char(ch: char) -> CharType {
    if ('\u{4e00}'..='\u{9fff}').contains(&ch) || ('\u{3400}'..='\u{4dbf}').contains(&ch) {
        CharType::Kanji
    } else if ('\u{3040}'..='\u{30ff}').contains(&ch) {
        CharType::Kana
    } else {
        CharType::Other
    }
}

/// Normalize katakana to hiragana so that kana matching works even when
/// the original text uses katakana (e.g. "パージ") while the reading is
/// in hiragana ("ぱあじ"). Small kana (ぁっゃ etc.) are intentionally
/// preserved — they match against small kana in the hiragana reading
/// and avoid over-normalization that would break sokuon/anchor searches.
#[inline]
fn to_hiragana(ch: char) -> char {
    if ('\u{30A1}'..='\u{30F6}').contains(&ch) {
        char::from_u32(ch as u32 - 96).unwrap_or(ch)
    } else {
        ch
    }
}

/// When the full kana anchor is not found in the hiragana reading
/// (e.g. because the romaji data omits sokuon markers — "to te" instead
/// of "to tte"), fall back to matching the last character of the anchor
/// as a consumption boundary. If even that fails, use the legacy
/// `kanji_len * 2` heuristic.
fn fallback_consume(remaining: &[char], kana_chars: &[char], kanji_len: usize) -> usize {
    if let Some(&last_char) = kana_chars.last() {
        if let Some(fallback_pos) = remaining.iter().position(|c| to_hiragana(*c) == last_char) {
            return fallback_pos;
        }
    }
    (kanji_len * 2).min(remaining.len())
}

struct RawBlock {
    text: String,
    char_type: CharType,
}

/// Align original Japanese text with hiragana reading to produce `Vec<LyricElement>`.
///
/// The algorithm:
/// 1. Classifies characters into Kanji, Kana, or Other blocks
/// 2. For each kanji block, uses the next kana block as an anchor to find reading boundaries
/// 3. Kana blocks are verified against and consume matching hiragana
/// 4. Adjacent same-type elements are merged (e.g., consecutive ruby elements),
///    unless separated by whitespace in the original text.
///
/// If no kana anchor is found (all-kanji text or mismatched input), falls back to
/// a heuristic of 2 kana characters per kanji.
pub fn align_ruby_to_text(original: &str, hiragana: &str) -> Vec<LyricElement> {
    let original_chars: Vec<char> = original.chars().collect();
    let hiragana_chars: Vec<char> = hiragana.chars().collect();
    let mut hira_idx: usize = 0;

    // Step 1: Classify into blocks
    let mut raw_blocks: Vec<RawBlock> = Vec::new();
    let mut i = 0;
    while i < original_chars.len() {
        let ct = classify_char(original_chars[i]);
        if ct == CharType::Kanji || ct == CharType::Kana {
            let mut text = String::new();
            while i < original_chars.len() && classify_char(original_chars[i]) == ct {
                text.push(original_chars[i]);
                i += 1;
            }
            raw_blocks.push(RawBlock {
                text,
                char_type: ct,
            });
        } else {
            raw_blocks.push(RawBlock {
                text: original_chars[i].to_string(),
                char_type: CharType::Other,
            });
            i += 1;
        }
    }

    // Step 2: Assign readings
    let mut elements: Vec<LyricElement> = Vec::new();

    for (block_idx, block) in raw_blocks.iter().enumerate() {
        match block.char_type {
            CharType::Kanji => {
                // Skip hiragana space markers (boundaries from original text)
                while hira_idx < hiragana_chars.len()
                    && hiragana_chars[hira_idx].is_whitespace()
                {
                    hira_idx += 1;
                }

                let kanji_len = block.text.chars().count();

                // Find where this kanji block's reading ends by searching
                // for the next kana block in the remaining hiragana.
                // Don't search past non-Kanji/non-Kana blocks (spaces, punctuation).
                let mut consume = if hira_idx < hiragana_chars.len() {
                    let following: Vec<&RawBlock> = raw_blocks[block_idx + 1..]
                        .iter()
                        .take_while(|b| {
                            b.char_type == CharType::Kanji
                                || b.char_type == CharType::Kana
                        })
                        .collect();
                    let hit_boundary = following.len() < raw_blocks[block_idx + 1..].len();

                    let next_kana = following
                        .iter()
                        .find(|b| b.char_type == CharType::Kana)
                        .map(|b| b.text.as_str());

                    if let Some(kana_text) = next_kana {
                        let remaining = &hiragana_chars[hira_idx..];
                        let kana_chars: Vec<char> =
                            kana_text.chars().map(to_hiragana).collect();
                        let pos = remaining
                            .windows(kana_chars.len())
                            .position(|w| w.iter().zip(&kana_chars).all(|(a, b)| to_hiragana(*a) == *b));
                        match pos {
                            Some(p) => p,
                            None => {
                                // Full anchor not found (e.g. romaji data lacks
                                // sokuon markers — "to te" instead of "to tte").
                                // Fall back to the last character of the anchor
                                // kana block as a boundary hint.
                                fallback_consume(remaining, &kana_chars, kanji_len)
                            }
                        }
                    } else if hit_boundary {
                        // No kana before the space/punctuation. Look past the
                        // boundary for the next kana and distribute the reading
                        // proportionally among all kanji blocks in the group.
                        let next_kana_all = raw_blocks[block_idx + 1..]
                            .iter()
                            .find(|b| b.char_type == CharType::Kana)
                            .map(|b| b.text.as_str());

                        if let Some(kana_text) = next_kana_all {
                            let all_kanji_before_kana: usize = raw_blocks[block_idx..]
                                .iter()
                                .take_while(|b| b.char_type != CharType::Kana)
                                .filter(|b| b.char_type == CharType::Kanji)
                                .map(|b| b.text.chars().count())
                                .sum();

                            let remaining = &hiragana_chars[hira_idx..];
                            let kana_chars: Vec<char> =
                                kana_text.chars().map(to_hiragana).collect();
                            let total_pos = remaining
                                .windows(kana_chars.len())
                                .position(|w| w.iter().zip(&kana_chars).all(|(a, b)| to_hiragana(*a) == *b))
                                .unwrap_or_else(|| fallback_consume(remaining, &kana_chars, kanji_len));

                            if all_kanji_before_kana > 0 {
                                (kanji_len * total_pos) / all_kanji_before_kana
                            } else {
                                (kanji_len * 2).min(remaining.len())
                            }
                        } else {
                            let remaining = &hiragana_chars[hira_idx..];
                            (kanji_len * 2).min(remaining.len())
                        }
                    } else {
                        let remaining = &hiragana_chars[hira_idx..];
                        (kanji_len * 2).min(remaining.len())
                    }
                } else {
                    0
                };

                if consume > 0 && kanji_len > 0 {
                    // Cap consumption at the first space marker in the remaining
                    // hiragana (spaces are inserted by insert_spaces_into_hiragana
                    // to signal boundaries from the original text).
                    let remaining = &hiragana_chars[hira_idx..];
                    if let Some(space_pos) = remaining.iter().position(|c| c.is_whitespace()) {
                        consume = consume.min(space_pos);
                    }
                    if kanji_len == 1 {
                        // Single kanji: existing behavior
                        let reading: String = hiragana_chars[hira_idx..hira_idx + consume]
                            .iter()
                            .collect();
                        elements.push(LyricElement::new_ruby(block.text.clone(), reading));
                        hira_idx += consume;
                    } else {
                        // Multi-kanji: proportionally distribute the reading
                        // among individual kanji characters. Extra chars are
                        // assigned to the rightmost characters.
                        let base = consume / kanji_len;
                        let extra = consume % kanji_len;
                        let rightmost_start = kanji_len - extra;

                        for (i, ch) in block.text.chars().enumerate() {
                            let reading_len = if i < rightmost_start {
                                base
                            } else {
                                base + 1
                            };

                            let reading: String =
                                hiragana_chars[hira_idx..hira_idx + reading_len]
                                    .iter()
                                    .collect();
                            elements.push(LyricElement::new_ruby(ch.to_string(), reading));
                            hira_idx += reading_len;
                        }
                    }
                } else {
                    elements.push(LyricElement::new_text(block.text.clone()));
                }
            }
            CharType::Kana => {
                let text_chars: Vec<char> = block.text.chars().collect();
                let mut matched = 0;
                while hira_idx < hiragana_chars.len() && matched < text_chars.len() {
                    let orig_ch = text_chars[matched];
                    if orig_ch == '\u{30FC}' {
                        // Long vowel mark (ー): romaji-derived hiragana uses
                        // actual vowel characters. Consume one hiragana char
                        // without comparing — the counts still align.
                        hira_idx += 1;
                        matched += 1;
                        continue;
                    }
                    if to_hiragana(hiragana_chars[hira_idx]) == to_hiragana(orig_ch) {
                        hira_idx += 1;
                        matched += 1;
                    } else {
                        break;
                    }
                }
                elements.push(LyricElement::new_text(block.text.clone()));
            }
            CharType::Other => {
                elements.push(LyricElement::new_text(block.text.clone()));
            }
        }
    }

    // Step 3: Merge adjacent same-type elements
    merge_adjacent(&elements)
}

fn merge_adjacent(elements: &[LyricElement]) -> Vec<LyricElement> {
    if elements.is_empty() {
        return vec![];
    }
    let mut merged: Vec<LyricElement> = Vec::new();
    let mut current_type: Option<String> = None;
    let mut current_base = String::new();
    let mut current_ruby = String::new();

    for elem in elements {
        let etype = &elem.element_type;
        let elem_base = elem.base.as_deref().unwrap_or("");
        let elem_ruby = elem.ruby.as_deref().unwrap_or("");

        let same_type = etype == "ruby" || etype == "text";
        let whitespace_prevents_merge = same_type
            && etype == "ruby"
            && (current_base.ends_with(char::is_whitespace)
                || elem_base.starts_with(char::is_whitespace));

        match &current_type {
            Some(t) if t == etype && etype != "linebreak" && !whitespace_prevents_merge => {
                current_base.push_str(elem_base);
                if !elem_ruby.is_empty() {
                    current_ruby.push_str(elem_ruby);
                }
            }
            _ => {
                // Flush previous
                if let Some(t) = current_type.take() {
                    match t.as_str() {
                        "ruby" => merged.push(LyricElement::new_ruby(
                            std::mem::take(&mut current_base),
                            std::mem::take(&mut current_ruby),
                        )),
                        "text" => {
                            merged.push(LyricElement::new_text(std::mem::take(&mut current_base)))
                        }
                        _ => {}
                    }
                }
                current_type = Some(etype.clone());
                current_base = elem_base.to_string();
                current_ruby = elem_ruby.to_string();
            }
        }
    }

    if let Some(t) = current_type {
        match t.as_str() {
            "ruby" => merged.push(LyricElement::new_ruby(current_base, current_ruby)),
            "text" => merged.push(LyricElement::new_text(current_base)),
            _ => {}
        }
    }

    merged
}

/// Insert whitespace markers into hiragana based on space positions in the
/// original text. Each space in the original signals a boundary where
/// the algorithm should split ruby blocks. Without these markers, the
/// kana-anchor search may find anchors past spaces and misdistribute readings.
pub fn insert_spaces_into_hiragana(original: &str, hiragana: &str) -> String {
    let hira_chars: Vec<char> = hiragana.chars().collect();
    let mut insert_positions: Vec<usize> = Vec::new();
    let mut kanji_count = 0;
    let mut kana_count = 0;
    let mut cumulative = 0;

    let total_kanji = original
        .chars()
        .filter(|c| matches!(classify_char(*c), CharType::Kanji))
        .count();
    let kanji_hira_ratio = if total_kanji > 0 {
        let total_kana_in_original = original
            .chars()
            .filter(|c| matches!(classify_char(*c), CharType::Kana))
            .count();
        let hira_for_kanji = hira_chars.len().saturating_sub(total_kana_in_original);
        hira_for_kanji as f64 / total_kanji as f64
    } else {
        2.0
    };

    for ch in original.chars() {
        match classify_char(ch) {
            CharType::Kanji => kanji_count += 1,
            CharType::Kana => kana_count += 1,
            CharType::Other if ch.is_whitespace() => {
                let estimated_hira =
                    (kanji_count as f64 * kanji_hira_ratio).floor() as usize + kana_count;
                cumulative += estimated_hira;
                let pos = cumulative.min(hira_chars.len());
                insert_positions.push(pos);
                kanji_count = 0;
                kana_count = 0;
            }
            _ => {}
        }
    }

    if insert_positions.is_empty() {
        return hiragana.to_string();
    }

    let mut result = String::with_capacity(hiragana.len() + insert_positions.len());
    let mut last_pos = 0;
    for pos in insert_positions {
        if pos > last_pos {
            result.extend(hira_chars[last_pos..pos].iter());
        }
        result.push(' ');
        last_pos = pos;
    }
    if last_pos < hira_chars.len() {
        result.extend(hira_chars[last_pos..].iter());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_simple_kanji_kana_mixed() {
        let elements = align_ruby_to_text("鮮やかなる色彩の", "あざやかなるしきさいの");
        assert_eq!(elements.len(), 4);
        assert_eq!(elements[0].element_type, "ruby");
        assert_eq!(elements[0].base.as_deref(), Some("鮮"));
        assert_eq!(elements[0].ruby.as_deref(), Some("あざ"));
        assert_eq!(elements[1].element_type, "text");
        assert_eq!(elements[1].base.as_deref(), Some("やかなる"));
        assert_eq!(elements[2].element_type, "ruby");
        assert_eq!(elements[2].base.as_deref(), Some("色彩"));
        assert_eq!(elements[2].ruby.as_deref(), Some("しきさい"));
        assert_eq!(elements[3].element_type, "text");
        assert_eq!(elements[3].base.as_deref(), Some("の"));
    }

    #[test]
    fn all_kana_no_ruby() {
        let elements = align_ruby_to_text("あいうえお", "あいうえお");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, "text");
    }

    #[test]
    fn all_kanji() {
        let elements = align_ruby_to_text("色彩", "しきさい");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, "ruby");
        assert_eq!(elements[0].base.as_deref(), Some("色彩"));
        assert_eq!(elements[0].ruby.as_deref(), Some("しきさい"));
    }

    #[test]
    fn empty_reading_falls_back_to_text() {
        let elements = align_ruby_to_text("漢字", "");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, "text");
    }

    #[test]
    fn merges_adjacent_ruby() {
        let elements = align_ruby_to_text("鮮明色彩", "あざあきしきさい");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, "ruby");
        assert_eq!(elements[0].base.as_deref(), Some("鮮明色彩"));
        assert_eq!(elements[0].ruby.as_deref(), Some("あざあきしきさい"));
    }

    #[test]
    fn space_separates_ruby_blocks() {
        // When the original text contains a space between two kanji blocks,
        // the ruby elements should NOT be merged across the space.
        // "鮮明 色彩" has a space between the two compounds.
        let elements = align_ruby_to_text("鮮明 色彩", "あざあきしきさい");
        assert_eq!(elements.len(), 3);
        assert_eq!(elements[0].element_type, "ruby");
        assert_eq!(elements[0].base.as_deref(), Some("鮮明"));
        assert_eq!(elements[0].ruby.as_deref(), Some("あざあき"));
        assert_eq!(elements[1].element_type, "text");
        assert_eq!(elements[1].base.as_deref(), Some(" "));
        assert_eq!(elements[2].element_type, "ruby");
        assert_eq!(elements[2].base.as_deref(), Some("色彩"));
        assert_eq!(elements[2].ruby.as_deref(), Some("しきさい"));
    }

    #[test]
    fn punctuation_splits_blocks() {
        let elements = align_ruby_to_text("歌、舞", "うたまい");
        assert_eq!(elements.len(), 3);
        assert_eq!(elements[0].element_type, "ruby");
        assert_eq!(elements[0].base.as_deref(), Some("歌"));
        assert_eq!(elements[1].element_type, "text");
        assert_eq!(elements[1].base.as_deref(), Some("、"));
        assert_eq!(elements[2].element_type, "ruby");
        assert_eq!(elements[2].base.as_deref(), Some("舞"));
    }

    #[test]
    fn distributes_reading_across_consecutive_kanji() {
        // Regression: "意味初" (3 consecutive kanji) should correctly
        // distribute the reading as 意=い, 味=み, 初=はじ internally,
        // then merge into ruby("意味初", "いみはじ").
        // The key fix: 初 keeps its はじ instead of it being consumed
        // by the previous block.
        let elements = align_ruby_to_text("温もりの意味初めて知れば", "ぬくもりのいみはじめてしれば");
        // 温(ruby) もりの(text) 意味初(ruby, merged) めて(text) 知(ruby) れば(text)
        assert_eq!(elements.len(), 6);
        assert_eq!(elements[0].base.as_deref(), Some("温"));
        assert_eq!(elements[0].ruby.as_deref(), Some("ぬく"));
        assert_eq!(elements[1].base.as_deref(), Some("もりの"));
        assert_eq!(elements[2].base.as_deref(), Some("意味初"));
        assert_eq!(elements[2].ruby.as_deref(), Some("いみはじ"));
        assert_eq!(elements[3].base.as_deref(), Some("めて"));
        assert_eq!(elements[4].base.as_deref(), Some("知"));
        assert_eq!(elements[4].ruby.as_deref(), Some("し"));
        assert_eq!(elements[5].base.as_deref(), Some("れば"));
    }

    #[test]
    fn non_japanese_preserved() {
        let elements = align_ruby_to_text("hello world", "hello world");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, "text");
    }

    #[test]
    fn katakana_original_matches_hiragana_reading() {
        let elements =
            align_ruby_to_text("パージして拓いた", "ぱあじしてひらいた");
        // 拓(ruby="ひら") いた(text)
        assert!(elements.len() >= 2, "should produce at least ruby+text");
        let ruby = elements.iter().find(|e| e.element_type == "ruby").unwrap();
        assert_eq!(ruby.base.as_deref(), Some("拓"));
        assert_eq!(
            ruby.ruby.as_deref(),
            Some("ひら"),
            "拓 should get ひら, not the preceding kana"
        );
    }

    #[test]
    fn space_boundary_with_kanji_group_distributes_proportionally() {
        // "意味" and "初" are consecutive kanji blocks separated by a space.
        // Total reading for the group (意味+初) is 4 kana (いみはじ).
        // 意味(2 kanji) gets floor(2/3*4)=2, 初(1 kanji) gets 4-2=2.
        let elements =
            align_ruby_to_text("温もりの意味 初めて知れば", "ぬくもりのいみはじめてしれば");
        let ruby_elements: Vec<_> = elements.iter().filter(|e| e.element_type == "ruby").collect();
        assert!(ruby_elements.iter().any(|e| e.base.as_deref() == Some("意味")),
            "意味 should be a ruby element");
        assert!(ruby_elements.iter().any(|e| e.base.as_deref() == Some("初")),
            "初 should be a ruby element");
        // 初 should have はじ (not be text)
        let hatsu = ruby_elements.iter().find(|e| e.base.as_deref() == Some("初")).unwrap();
        assert_eq!(hatsu.ruby.as_deref(), Some("はじ"), "初 should get はじ");
    }

    #[test]
    fn ruby_kanji_not_overconsuming_into_trailing_kana() {
        // Regression: "貴方の" where 貴方 is followed by の katakana particle.
        // The reading for 貴方 should be あなた (2 chars),
        // NOT あなたの (3 chars). The のアイダ are consecutive kana so they
        // form a single text block — that's fine, the key fix is avoiding
        // ruby overconsumption when katakana anchor search fails.
        let elements =
            align_ruby_to_text("\"白\"は私と貴方のアイダ", "しろはわたしとあなたのあいだ");
        let ruby_elements: Vec<_> = elements.iter().filter(|e| e.element_type == "ruby").collect();

        let anata = ruby_elements.iter().find(|e| e.base.as_deref() == Some("貴方"));
        assert!(anata.is_some(), "貴方 should be a ruby element");
        assert_eq!(
            anata.unwrap().ruby.as_deref(),
            Some("あなた"),
            "貴方 should get あなた, NOT あなたの — the の is a separate particle"
        );

        // の should NOT appear as a duplicate — it belongs in the trailing text
        let last_text = elements.last().unwrap();
        assert_eq!(last_text.element_type, "text");
        assert!(last_text.base.as_deref().unwrap().contains("の"),
            "の should be in the trailing text, not duplicated in ruby");
    }

    // ── Bug reproduction tests ──

    #[test]
    fn bug3_hakushi_particle_not_in_ruby_with_space() {
        // Bug 3 repro: "白紙のキャンバス" with a preceding space
        // Expected: 白紙=はくし, NOT はくしの
        let elements = align_ruby_to_text(
            "此処に立つ僕は 白紙のキャンバス",
            "ここにたつぼくははくしのきゃんばす",
        );
        let hakushi = elements.iter().find(|e| e.base.as_deref() == Some("白紙"));
        assert!(hakushi.is_some(), "白紙 should be a ruby element");
        assert_eq!(
            hakushi.unwrap().ruby.as_deref(),
            Some("はくし"),
            "白紙 should be はくし, not はくしの"
        );
    }

    #[test]
    fn bug3_hakushi_particle_not_in_ruby_without_space() {
        // Same but without the space (no boundary marker)
        let elements = align_ruby_to_text(
            "此処に立つ僕は白紙のキャンバス",
            "ここにたつぼくははくしのきゃんばす",
        );
        let hakushi = elements.iter().find(|e| e.base.as_deref() == Some("白紙"));
        assert!(hakushi.is_some(), "白紙 should be a ruby element");
        assert_eq!(
            hakushi.unwrap().ruby.as_deref(),
            Some("はくし"),
            "白紙 should be はくし, not はくしの"
        );
    }

    #[test]
    fn bug5_okurigana_not_leaked_into_ruby_single_kanji() {
        // Bug 5 repro: "取って" where hiragana is "とて" (missing sokuon)
        // Fixed: the fallback should consume only up to the last matching
        // kana char (て), giving 取=と, NOT 取=とて
        let elements = align_ruby_to_text("取って", "とて");
        let toru = elements.iter().find(|e| e.base.as_deref() == Some("取"));
        assert!(toru.is_some(), "取 should be a ruby element");
        assert_eq!(
            toru.unwrap().ruby.as_deref(),
            Some("と"),
            "取 should get と, not とて — okurigana should not leak into ruby"
        );
        let has_tte_text = elements.iter().any(|e| {
            e.element_type == "text"
                && e.base.as_deref().unwrap_or("").contains("って")
        });
        assert!(has_tte_text, "って should appear as text after 取");
    }

    #[test]
    fn bug5_okurigana_anchor_search_handles_mismatch() {
        // Broader test: when the kana anchor search fails because
        // hiragana doesn't contain the expected kana (e.g., って vs とて),
        // the fallback should not blindly consume all remaining hiragana.
        let elements = align_ruby_to_text("取って", "とて");
        let toru = elements.iter().find(|e| e.base.as_deref() == Some("取"));
        assert!(toru.is_some(), "取 should be a ruby element");
        assert_eq!(
            toru.unwrap().ruby.as_deref(),
            Some("と"),
            "取 should get と after fallback fix"
        );
        let has_tte_text = elements.iter().any(|e| {
            e.element_type == "text"
                && e.base.as_deref().unwrap_or("").contains("って")
        });
        assert!(has_tte_text, "って should appear as text after 取, not be consumed");
    }

    #[test]
    fn romaji_to_hiragana_preserves_long_vowels() {
        // Verify that romaji conversion is not the source of truncation
        // for bugs 1 (救済=きゅうさい) and 2 (感情=かんじょう)
        let h = crate::romaji::romaji_to_hiragana("kyu u sa i");
        assert_eq!(h, "きゅうさい", "救済 should convert correctly from romaji");
        let h = crate::romaji::romaji_to_hiragana("ka n jo u");
        assert_eq!(h, "かんじょう", "感情 should convert correctly from romaji");
    }

    #[test]
    fn insert_spaces_into_hiragana_preserves_boundary() {
        // Verify insert_spaces_into_hiragana works correctly
        let result = insert_spaces_into_hiragana(
            "此処に立つ僕は 白紙のキャンバス",
            "ここにたつぼくははくしのきゃんばす",
        );
        // Should have a space marker inserted
        assert!(result.contains(' '), "should insert a space marker");
    }
}