use crate::models::LyricElement;

/// 字符类型枚举，用于区分日文字符类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharType {
    /// 汉字（CJK统一表意文字区）
    Kanji,
    /// 假名（平假名和片假名）
    Kana,
    /// 其他字符（标点、空格、拉丁字母等）
    Other,
}

/// 将字符分类为汉字、假名或其他类型
///
/// 汉字的判断基于 CJK 统一表意文字区（U+4E00~U+9FFF）和
/// CJK 扩展区（U+3400~U+4DBF）。
/// 假名的判断基于平假名和片假名区（U+3040~U+30FF）。
#[inline]
pub(crate) fn classify_char(ch: char) -> CharType {
    if ('\u{4e00}'..='\u{9fff}').contains(&ch) || ('\u{3400}'..='\u{4dbf}').contains(&ch) {
        CharType::Kanji
    } else if ('\u{3040}'..='\u{30ff}').contains(&ch) {
        CharType::Kana
    } else {
        CharType::Other
    }
}

/// 将片假名规范化为平假名，使得即使原文使用片假名（如"パージ"）
/// 而注音读法是平假名（"ぱあじ"）时，假名匹配仍然能正常工作。
/// 小假名（ぁっゃ等）被有意保留 —— 它们与平假名读法中的小假名匹配，
/// 避免过度规范化导致促音/锚点搜索失败。
///
/// 片假名范围之外的字符，如长音符号 ー（U+30FC）和
/// 反复记号 ヽ（U+30FD）、ヾ（U+30FE），按原样返回，
/// 因为它们是附加符号而非假名。
#[inline]
fn to_hiragana(ch: char) -> char {
    if ('\u{30A1}'..='\u{30F6}').contains(&ch)
        && ch != '\u{30FC}'
        && ch != '\u{30FD}'
        && ch != '\u{30FE}'
    {
        char::from_u32(ch as u32 - 96).unwrap_or(ch)
    } else {
        ch
    }
}

/// 比较两个假名字符是否匹配，处理助词音变和长音符号的等价关系。具体来说：
/// - `は`（ha）作为主题助词时，罗马音读作 `わ`（wa）
/// - `へ`（he）作为方向助词时，罗马音读作 `え`（e）
/// - `ー`（长音符号）在片假名中表示元音延长；
///   在由罗马音派生的平假名中，对应的是短元音
/// NetEase 的 romalrc 总是使用实际发音（wa/e/元音），
/// 而原始 YRC 文本保留书写形式（は/へ/ー）。
#[inline]
fn chars_match_kana(a: char, b: char) -> bool {
    let a = to_hiragana(a);
    let b = to_hiragana(b);
    a == b
        || (a == 'は' && b == 'わ') || (a == 'わ' && b == 'は')
        || (a == 'へ' && b == 'え') || (a == 'え' && b == 'へ')
        || (a == 'ー' && is_vowel_kana(b))
        || (b == 'ー' && is_vowel_kana(a))
}

/// 判断字符是否为日语元音假名（あいうえお及对应的片假名）
fn is_vowel_kana(c: char) -> bool {
    matches!(c, 'あ' | 'い' | 'う' | 'え' | 'お' | 'ア' | 'イ' | 'ウ' | 'エ' | 'オ')
}

/// 当完整的假名锚点在平假名读法中找不到时
///（例如，罗马音数据省略了促音标记 —— "to te" 而不是 "to tte"），
/// 回退到匹配锚点的最后一个字符作为消耗边界。
/// 如果仍然失败，则使用传统的 `kanji_len * 2` 启发式规则。
fn fallback_consume(remaining: &[char], kana_chars: &[char], kanji_len: usize) -> usize {
    if let Some(&last_char) = kana_chars.last() {
        if let Some(fallback_pos) = remaining.iter().position(|c| chars_match_kana(*c, last_char)) {
            return fallback_pos;
        }
    }
    let heuristic = (kanji_len * 2).min(remaining.len());
    heuristic
}

/// 原始文本块，包含文本内容和对应的字符类型
struct RawBlock {
    /// 文本内容
    text: String,
    /// 字符类型（汉字/假名/其他）
    char_type: CharType,
}

/// 将日文原文与平假名读音对齐，生成 `Vec<LyricElement>`。
///
/// 算法步骤：
/// 1. 将字符分类为汉字、假名或其他类型的块
/// 2. 对于每个汉字块，使用下一个假名块作为锚点来查找读音边界
/// 3. 验证假名块并消耗匹配的平假名
/// 4. 合并相邻的同类型元素（如连续的 ruby 元素），
///    除非它们在原文中被空格分隔。
///
/// 如果找不到假名锚点（全汉字文本或输入不匹配），
/// 回退到每个汉字对应 2 个假名字符的启发式规则。
pub fn align_ruby_to_text(original: &str, hiragana: &str) -> Vec<LyricElement> {
    let original_chars: Vec<char> = original.chars().collect();
    let hiragana_chars: Vec<char> = hiragana.chars().collect();
    let mut hira_idx: usize = 0;

    // 第 1 步：将字符分类为块
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

    // 第 2 步：为每个块分配读音
    let mut elements: Vec<LyricElement> = Vec::new();

    for (block_idx, block) in raw_blocks.iter().enumerate() {
        match block.char_type {
            CharType::Kanji => {
                // 跳过平假名中的空格标记（来自原文的边界标记）
                while hira_idx < hiragana_chars.len()
                    && hiragana_chars[hira_idx].is_whitespace()
                {
                    hira_idx += 1;
                }

                let kanji_len = block.text.chars().count();

                // 通过在剩余平假名中搜索下一个假名块
                // 来查找当前汉字块的读音结束位置。
                // 不要搜索越过非汉字/非假名块（空格、标点）。
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
                            .position(|w| w.iter().zip(&kana_chars).all(|(a, b)| chars_match_kana(*a, *b)));
                        match pos {
                            Some(p) => p,
                            None => {
                                // 完整的锚点未找到（例如罗马音数据缺少
                                // 促音标记 —— "to te" 而不是 "to tte"）。
                                // 回退到使用锚点假名块的
                                // 最后一个字符作为边界提示。
                                fallback_consume(remaining, &kana_chars, kanji_len)
                            }
                        }
                    } else if hit_boundary {
                        // 在空格/标点之前没有假名。越过
                        // 边界查找下一个假名，并将读音
                        // 按比例分配给组中所有汉字块。
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
                                .position(|w| w.iter().zip(&kana_chars).all(|(a, b)| chars_match_kana(*a, *b)))
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
                    // 将消耗上限限制在剩余平假名中第一个空格标记处
                    //（insert_spaces_into_hiragana 插入空格
                    // 以标记原文中的边界）。
                    let remaining = &hiragana_chars[hira_idx..];
                    if let Some(space_pos) = remaining.iter().position(|c| c.is_whitespace()) {
                        consume = consume.min(space_pos);
                    }
                    if kanji_len == 1 {
                        // 单汉字：保持现有行为
                        let reading: String = hiragana_chars[hira_idx..hira_idx + consume]
                            .iter()
                            .collect();
                        elements.push(LyricElement::new_ruby(block.text.clone(), reading));
                        hira_idx += consume;
                    } else {
                        // 多汉字：按比例将读音分配给各个汉字。
                        // 多余的字符分配给最右边的汉字。
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
                // 假名块：逐字符匹配并消耗平假名读音
                let text_chars: Vec<char> = block.text.chars().collect();
                let mut matched = 0;
                while hira_idx < hiragana_chars.len() && matched < text_chars.len() {
                    let orig_ch = text_chars[matched];
                    if orig_ch == '\u{30FC}' {
                        // 长音符号（ー）：由罗马音派生的平假名使用
                        // 实际的元音字符。消耗一个平假名字符
                        // 而不进行比较 —— 计数仍然对齐。
                        hira_idx += 1;
                        matched += 1;
                        continue;
                    }
                    if chars_match_kana(orig_ch, hiragana_chars[hira_idx]) {
                        hira_idx += 1;
                        matched += 1;
                    } else {
                        break;
                    }
                }
                elements.push(LyricElement::new_text(block.text.clone()));
            }
            CharType::Other => {
                // 其他字符（标点、空格等）：直接作为文本输出
                elements.push(LyricElement::new_text(block.text.clone()));
            }
        }
    }

    // 第 3 步：合并相邻的同类型元素
    merge_adjacent(&elements)
}

/// 合并相邻的同类型 LyricElement。
///
/// 同类型的相邻元素（如连续的 ruby 元素或连续的 text 元素）
/// 会被合并为一个元素。但如果元素之间在原文中有空格分隔，
/// 则跳过合并以保留原文的边界信息。
pub(crate) fn merge_adjacent(elements: &[LyricElement]) -> Vec<LyricElement> {
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
                // 刷新上一个合并结果
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

    // 处理最后一个未处理的合并项
    if let Some(t) = current_type {
        match t.as_str() {
            "ruby" => merged.push(LyricElement::new_ruby(current_base, current_ruby)),
            "text" => merged.push(LyricElement::new_text(current_base)),
            _ => {}
        }
    }

    merged
}

/// 后处理 LyricElement，修复常见的编码和规范化问题。
///
/// 当前处理：
/// - ruby 文本中残留的 ASCII 辅音字母（如代表促音っ的 's'）
///   这捕获了上游数据管道中っ（U+3063）丢失正确编码的问题。
pub fn sanitize_ruby_elements(elements: Vec<LyricElement>) -> Vec<LyricElement> {
    elements.into_iter().map(|elem| {
        if elem.element_type == "ruby" {
            if let Some(ref ruby_text) = elem.ruby {
                // 检查 ruby 文本是否包含 ASCII 字母
                if ruby_text.bytes().any(|b| b.is_ascii_alphabetic()) {
                    // 将残留的辅音替换为促音（っ）
                    let sanitized: String = ruby_text.chars().map(|c| {
                        match c {
                            's' | 't' | 'k' | 'p' | 'b' | 'd' | 'g' | 'j' | 'c' => 'っ',
                            _ => c,
                        }
                    }).collect();
                    if sanitized != *ruby_text {
                        return LyricElement::new_ruby(
                            elem.base.unwrap_or_default(),
                            sanitized,
                        );
                    }
                }
            }
        }
        elem
    }).collect()
}

/// 根据原文中的空格位置，在平假名中插入空格标记。
///
/// 原文中的每个空格标志着算法应拆分 ruby 块的边界。
/// 如果没有这些标记，假名锚点搜索可能会越过空格找到锚点，
/// 导致读音分配错误。
pub fn insert_spaces_into_hiragana(original: &str, hiragana: &str) -> String {
    let hira_chars: Vec<char> = hiragana.chars().collect();
    let mut insert_positions: Vec<usize> = Vec::new();
    let mut kanji_count = 0;
    let mut kana_count = 0;
    let mut cumulative = 0;

    // 计算汉字与假名的比例，用于估计每个汉字块对应的平假名数量
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

    // 遍历原文，在空格处计算插入位置
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

    // 如果没有需要插入空格的位置，直接返回原字符串
    if insert_positions.is_empty() {
        return hiragana.to_string();
    }

    // 在计算出的位置插入空格标记
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
