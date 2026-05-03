use crate::models::LyricElement;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharType {
    Kanji,
    Kana,
    Other,
}

fn classify_char(ch: char) -> CharType {
    if ('\u{4e00}'..='\u{9fff}').contains(&ch)
        || ('\u{3400}'..='\u{4dbf}').contains(&ch)
    {
        CharType::Kanji
    } else if ('\u{3040}'..='\u{30ff}').contains(&ch) {
        CharType::Kana
    } else {
        CharType::Other
    }
}

struct RawBlock {
    text: String,
    char_type: CharType,
}

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
            raw_blocks.push(RawBlock { text, char_type: ct });
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

    for block in &raw_blocks {
        match block.char_type {
            CharType::Kanji => {
                let kanji_len = block.text.chars().count();
                let max_consume = (kanji_len * 2).min(hiragana_chars.len().saturating_sub(hira_idx));
                if max_consume > 0 && hira_idx < hiragana_chars.len() {
                    let reading: String = hiragana_chars[hira_idx..hira_idx + max_consume]
                        .iter()
                        .collect();
                    elements.push(LyricElement::new_ruby(block.text.clone(), reading));
                    hira_idx += max_consume;
                } else {
                    elements.push(LyricElement::new_text(block.text.clone()));
                }
            }
            CharType::Kana => {
                let text_chars: Vec<char> = block.text.chars().collect();
                let mut matched = 0;
                while hira_idx < hiragana_chars.len() && matched < text_chars.len() {
                    if hiragana_chars[hira_idx] == text_chars[matched] {
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
        match &current_type {
            Some(t) if t == etype && etype != "linebreak" => {
                if let Some(b) = &elem.base {
                    current_base.push_str(b);
                }
                if let Some(r) = &elem.ruby {
                    current_ruby.push_str(r);
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
                        "text" => merged.push(LyricElement::new_text(
                            std::mem::take(&mut current_base),
                        )),
                        _ => {}
                    }
                }
                current_type = Some(etype.clone());
                current_base = elem.base.clone().unwrap_or_default();
                current_ruby = elem.ruby.clone().unwrap_or_default();
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
    fn non_japanese_preserved() {
        let elements = align_ruby_to_text("hello world", "hello world");
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].element_type, "text");
    }
}
