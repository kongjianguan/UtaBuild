use std::collections::HashMap;

/// 获取罗马音到平假名的懒静态映射表。
/// 使用 OnceLock 实现单次初始化，仅在首次调用时构建映射。
fn romaji_map() -> &'static HashMap<String, String> {
    use std::sync::OnceLock;
    /// 存储罗马音→平假名映射的全局静态变量
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    MAP.get_or_init(build_romaji_map)
}

/// 将罗马音字符串转换为平假名，无法映射的字符（数字、标点等）原样保留。
pub fn romaji_to_hiragana(input: &str) -> String {
    romaji_to_hiragana_impl(input, false)
}

/// Convert romaji to hiragana, skipping characters that cannot be mapped
/// (digits, punctuation, leaked timing data) instead of passing them through.
/// 将罗马音转换为平假名，严格模式：跳过无法映射的字符（数字、标点、泄露的时间数据等），而非原样保留。
pub fn romaji_to_hiragana_strict(input: &str) -> String {
    romaji_to_hiragana_impl(input, true)
}

/// Characters that could represent sokuon (促音) when appearing alone.
/// In romaji, a lone 's', 't', 'k', 'p', 'b', 'd', 'g', 'j', 'c'
/// before another consonant syllable indicates gemination (っ).
/// 判断字符是否为促音候选字符。单独出现的 's'/'t'/'k' 等辅音
/// 在另一个辅音音节之前表示促音（っ）。
fn is_sokuon_candidate(c: char) -> bool {
    matches!(c, 's' | 't' | 'k' | 'p' | 'b' | 'd' | 'g' | 'j' | 'c')
}

/// 罗马音到平假名转换的核心实现。
/// 按空格拆分单词，对每个单词从左到右贪婪匹配最长的可用映射（4→3→2→1 字符）。
/// 在非严格模式下，无法映射的字符原样保留；严格模式下则跳过。
fn romaji_to_hiragana_impl(input: &str, strict: bool) -> String {
    let map = romaji_map();
    let mut result = String::with_capacity(input.len());

    for word in input.split_whitespace() {
        let mut i = 0;
        let chars: Vec<char> = word.chars().collect();
        while i < chars.len() {
            // Try 4-char match first (for sokuon like "sshi", "tchi", "ttsu")
            // 优先尝试 4 字符匹配（处理促音 + 拗音组合，如 "sshi"、"tchi"、"ttsu"）
            if i + 3 < chars.len() {
                let four: String = chars[i..i + 4].iter().collect();
                if let Some(h) = map.get(&four) {
                    result.push_str(h);
                    i += 4;
                    continue;
                }
            }
            // Try 3-char match (youon like "kya", "shu", "cho")
            // 尝试 3 字符匹配（处理拗音，如 "kya"、"shu"、"cho"）
            if i + 2 < chars.len() {
                let three: String = chars[i..i + 3].iter().collect();
                if let Some(h) = map.get(&three) {
                    result.push_str(h);
                    i += 3;
                    continue;
                }
            }
            // Try 2-char match (basic syllables + dakuon + sokuon)
            // 尝试 2 字符匹配（基本音节、浊音、促音）
            if i + 1 < chars.len() {
                let two: String = chars[i..i + 2].iter().collect();
                if let Some(h) = map.get(&two) {
                    result.push_str(h);
                    i += 2;
                    continue;
                }
            }
            // Single char
            // 单字符匹配
            let one: String = chars[i..=i].iter().collect();
            if let Some(h) = map.get(&one) {
                result.push_str(h);
                i += 1;
            } else if strict && is_sokuon_candidate(chars[i]) {
                // Stray consonant → sokuon (っ).
                // This handles broken romaji like "i s sho" (should be "i ssho")
                // where the 's' is a standalone sokuon marker.
                // 单独的辅音字母在严格模式下视为促音（っ）。
                // 用于处理不规范的罗马音如 "i s sho"（应为 "i ssho"），
                // 其中 's' 是独立的促音标记。
                result.push('っ');
                i += 1;
            } else if !strict {
                result.push(chars[i]);
                i += 1;
            } else {
                i += 1; // skip unmappable in strict mode
                        // 严格模式下跳过无法映射的字符
            }
        }
    }
    result
}

/// 构建完整的罗马音→平假名映射表。
/// 包含基本音节、浊音、半浊音、拗音、促音及促音+拗音组合，
/// 以及长音标记（重复元音 → あー等）。
fn build_romaji_map() -> HashMap<String, String> {
    let pairs: &[(&str, &str)] = &[
        // 基本元音
        ("a", "あ"),
        ("i", "い"),
        ("u", "う"),
        ("e", "え"),
        ("o", "お"),
        // か行
        ("ka", "か"),
        ("ki", "き"),
        ("ku", "く"),
        ("ke", "け"),
        ("ko", "こ"),
        // さ行
        ("sa", "さ"),
        ("shi", "し"),
        ("su", "す"),
        ("se", "せ"),
        ("so", "そ"),
        // た行
        ("ta", "た"),
        ("chi", "ち"),
        ("tsu", "つ"),
        ("te", "て"),
        ("to", "と"),
        // な行
        ("na", "な"),
        ("ni", "に"),
        ("nu", "ぬ"),
        ("ne", "ね"),
        ("no", "の"),
        // は行
        ("ha", "は"),
        ("hi", "ひ"),
        ("fu", "ふ"),
        ("fa", "ふぁ"),
        ("fi", "ふぃ"),
        ("fe", "ふぇ"),
        ("fo", "ふぉ"),
        ("fya", "ふゃ"),
        ("fyu", "ふゅ"),
        ("fyo", "ふょ"),
        ("he", "へ"),
        ("ho", "ほ"),
        // ま行
        ("ma", "ま"),
        ("mi", "み"),
        ("mu", "む"),
        ("me", "め"),
        ("mo", "も"),
        // や行
        ("ya", "や"),
        ("yu", "ゆ"),
        ("yo", "よ"),
        // ら行
        ("ra", "ら"),
        ("ri", "り"),
        ("ru", "る"),
        ("re", "れ"),
        ("ro", "ろ"),
        // わ行
        ("wa", "わ"),
        ("wo", "を"),
        // 拨音
        ("n", "ん"),
        // 浊音 が行
        ("ga", "が"),
        ("gi", "ぎ"),
        ("gu", "ぐ"),
        ("ge", "げ"),
        ("go", "ご"),
        // 浊音 ざ行
        ("za", "ざ"),
        ("ji", "じ"),
        ("zu", "ず"),
        ("ze", "ぜ"),
        ("zo", "ぞ"),
        // 浊音 だ行
        ("da", "だ"),
        ("di", "ぢ"),
        ("du", "づ"),
        ("de", "で"),
        ("do", "ど"),
        // 浊音 ば行
        ("ba", "ば"),
        ("bi", "び"),
        ("bu", "ぶ"),
        ("be", "べ"),
        ("bo", "ぼ"),
        // 半浊音 ぱ行
        ("pa", "ぱ"),
        ("pi", "ぴ"),
        ("pu", "ぷ"),
        ("pe", "ぺ"),
        ("po", "ぽ"),
        // 拗音（きゃ行）
        ("kya", "きゃ"),
        ("kyu", "きゅ"),
        ("kyo", "きょ"),
        // 拗音（しゃ行）
        ("sha", "しゃ"),
        ("shu", "しゅ"),
        ("sho", "しょ"),
        // 拗音（ちゃ行）
        ("cha", "ちゃ"),
        ("chu", "ちゅ"),
        ("cho", "ちょ"),
        // 拗音（にゃ行）
        ("nya", "にゃ"),
        ("nyu", "にゅ"),
        ("nyo", "にょ"),
        // 拗音（ひゃ行）
        ("hya", "ひゃ"),
        ("hyu", "ひゅ"),
        ("hyo", "ひょ"),
        // 拗音（みゃ行）
        ("mya", "みゃ"),
        ("myu", "みゅ"),
        ("myo", "みょ"),
        // 拗音（りゃ行）
        ("rya", "りゃ"),
        ("ryu", "りゅ"),
        ("ryo", "りょ"),
        // 拗音 浊音（ぎゃ行）
        ("gya", "ぎゃ"),
        ("gyu", "ぎゅ"),
        ("gyo", "ぎょ"),
        // 拗音 浊音（じゃ行）
        ("ja", "じゃ"),
        ("ju", "じゅ"),
        ("jo", "じょ"),
        // 拗音 浊音（びゃ行）
        ("bya", "びゃ"),
        ("byu", "びゅ"),
        ("byo", "びょ"),
        // 拗音 半浊音（ぴゃ行）
        ("pya", "ぴゃ"),
        ("pyu", "ぴゅ"),
        ("pyo", "ぴょ"),
        // 促音 + か行
        ("kka", "っか"),
        ("kki", "っき"),
        ("kku", "っく"),
        ("kke", "っけ"),
        ("kko", "っこ"),
        // 促音 + さ行
        ("ssa", "っさ"),
        ("sshi", "っし"),
        ("ssu", "っす"),
        ("sse", "っせ"),
        ("sso", "っそ"),
        // 促音 + た行
        ("tta", "った"),
        ("tchi", "っち"),
        ("ttsu", "っつ"),
        ("tte", "って"),
        ("tto", "っと"),
        // 促音 + ぱ行
        ("ppa", "っぱ"),
        ("ppi", "っぴ"),
        ("ppu", "っぷ"),
        ("ppe", "っぺ"),
        ("ppo", "っぽ"),
        // 促音 + だ行
        ("dda", "っだ"),
        ("ddi", "っぢ"),
        ("ddu", "っづ"),
        ("dde", "っで"),
        ("ddo", "っど"),
        // 促音 + が行
        ("gga", "っが"),
        ("ggi", "っぎ"),
        ("ggu", "っぐ"),
        ("gge", "っげ"),
        ("ggo", "っご"),
        // 促音 + ば行
        ("bba", "っば"),
        ("bbi", "っび"),
        ("bbu", "っぶ"),
        ("bbe", "っべ"),
        ("bbo", "っぼ"),
        // 促音 + ふ行
        ("ffa", "っふぁ"),
        ("ffi", "っふぃ"),
        ("ffe", "っふぇ"),
        ("ffo", "っふぉ"),
        ("ffya", "っふゃ"),
        ("ffyu", "っふゅ"),
        ("ffyo", "っふょ"),
        // 长音（重复元音 → 元音+长音符号）
        ("aa", "あー"),
        ("ii", "いー"),
        ("uu", "うー"),
        ("ee", "えー"),
        ("oo", "おー"),
        // Sokuon + youon (missing entries for common combinations)
        // 促音 + 拗音（常用组合补全）
        ("kkya", "っきゃ"),
        ("kkyu", "っきゅ"),
        ("kkyo", "っきょ"),
        ("ssha", "っしゃ"),
        ("sshu", "っしゅ"),
        ("ssho", "っしょ"),
        ("tcha", "っちゃ"),
        ("tchu", "っちゅ"),
        ("tcho", "っちょ"),
        ("ppya", "っぴゃ"),
        ("ppyu", "っぴゅ"),
        ("ppyo", "っぴょ"),
        ("bbya", "っびゃ"),
        ("bbyu", "っびゅ"),
        ("bbyo", "っびょ"),
        ("ggya", "っぎゃ"),
        ("ggyu", "っぎゅ"),
        ("ggyo", "っぎょ"),
        ("jja",  "っじゃ"),
        ("jju",  "っじゅ"),
        ("jjo",  "っじょ"),
    ];
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试基本音节的转换
    #[test]
    fn converts_basic_syllables() {
        assert_eq!(romaji_to_hiragana("a"), "あ");
        assert_eq!(romaji_to_hiragana("ka"), "か");
        assert_eq!(romaji_to_hiragana("ki"), "き");
        assert_eq!(romaji_to_hiragana("ku"), "く");
        assert_eq!(romaji_to_hiragana("ke"), "け");
        assert_eq!(romaji_to_hiragana("ko"), "こ");
    }

    /// 测试浊音的转换（が行、ざ行、だ行、ば行）
    #[test]
    fn converts_dakuon() {
        assert_eq!(romaji_to_hiragana("ga"), "が");
        assert_eq!(romaji_to_hiragana("ji"), "じ");
        assert_eq!(romaji_to_hiragana("zu"), "ず");
        assert_eq!(romaji_to_hiragana("de"), "で");
        assert_eq!(romaji_to_hiragana("bo"), "ぼ");
    }

    /// 测试拗音的转换（きゃ、しゅ、ちょ等）
    #[test]
    fn converts_youon() {
        assert_eq!(romaji_to_hiragana("kya"), "きゃ");
        assert_eq!(romaji_to_hiragana("shu"), "しゅ");
        assert_eq!(romaji_to_hiragana("cho"), "ちょ");
        assert_eq!(romaji_to_hiragana("nya"), "にゃ");
        assert_eq!(romaji_to_hiragana("ryo"), "りょ");
    }

    /// 测试促音的转换（っか、った、っさ等）
    #[test]
    fn converts_sokuon() {
        assert_eq!(romaji_to_hiragana("kka"), "っか");
        assert_eq!(romaji_to_hiragana("tta"), "った");
        assert_eq!(romaji_to_hiragana("ssa"), "っさ");
        assert_eq!(romaji_to_hiragana("ppi"), "っぴ");
        assert_eq!(romaji_to_hiragana("sshi"), "っし");
        assert_eq!(romaji_to_hiragana("tchi"), "っち");
        assert_eq!(romaji_to_hiragana("ttsu"), "っつ");
    }

    /// 测试非日语字符（数字、标点）的保留
    #[test]
    fn preserves_non_japanese() {
        assert_eq!(romaji_to_hiragana("123"), "123");
        assert_eq!(romaji_to_hiragana("!"), "!");
    }

    /// 测试空格分隔单词的转换
    #[test]
    fn converts_space_separated_words() {
        assert_eq!(romaji_to_hiragana("a za ya ka"), "あざやか");
        assert_eq!(romaji_to_hiragana("na ru"), "なる");
    }

    /// 测试助词 \"wa\" 的转换
    #[test]
    fn handles_particle_wa() {
        assert_eq!(romaji_to_hiragana("wa"), "わ");
    }

    /// 测试空输入的处理
    #[test]
    fn handles_empty_input() {
        assert_eq!(romaji_to_hiragana(""), "");
    }

    /// 测试混合内容的转换
    #[test]
    fn handles_mixed_content() {
        assert_eq!(romaji_to_hiragana("shi ki sa i"), "しきさい");
    }
}
