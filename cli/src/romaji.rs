use std::collections::HashMap;

fn romaji_map() -> &'static HashMap<String, String> {
    use std::sync::OnceLock;
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    MAP.get_or_init(build_romaji_map)
}

pub fn romaji_to_hiragana(input: &str) -> String {
    romaji_to_hiragana_impl(input, false)
}

/// Convert romaji to hiragana, skipping characters that cannot be mapped
/// (digits, punctuation, leaked timing data) instead of passing them through.
pub fn romaji_to_hiragana_strict(input: &str) -> String {
    romaji_to_hiragana_impl(input, true)
}

/// Characters that could represent sokuon (促音) when appearing alone.
/// In romaji, a lone 's', 't', 'k', 'p', 'b', 'd', 'g', 'j', 'c'
/// before another consonant syllable indicates gemination (っ).
fn is_sokuon_candidate(c: char) -> bool {
    matches!(c, 's' | 't' | 'k' | 'p' | 'b' | 'd' | 'g' | 'j' | 'c')
}

fn romaji_to_hiragana_impl(input: &str, strict: bool) -> String {
    let map = romaji_map();
    let mut result = String::with_capacity(input.len());

    for word in input.split_whitespace() {
        let mut i = 0;
        let chars: Vec<char> = word.chars().collect();
        while i < chars.len() {
            // Try 4-char match first (for sokuon like "sshi", "tchi", "ttsu")
            if i + 3 < chars.len() {
                let four: String = chars[i..i + 4].iter().collect();
                if let Some(h) = map.get(&four) {
                    result.push_str(h);
                    i += 4;
                    continue;
                }
            }
            // Try 3-char match (youon like "kya", "shu", "cho")
            if i + 2 < chars.len() {
                let three: String = chars[i..i + 3].iter().collect();
                if let Some(h) = map.get(&three) {
                    result.push_str(h);
                    i += 3;
                    continue;
                }
            }
            // Try 2-char match (basic syllables + dakuon + sokuon)
            if i + 1 < chars.len() {
                let two: String = chars[i..i + 2].iter().collect();
                if let Some(h) = map.get(&two) {
                    result.push_str(h);
                    i += 2;
                    continue;
                }
            }
            // Single char
            let one: String = chars[i..=i].iter().collect();
            if let Some(h) = map.get(&one) {
                result.push_str(h);
                i += 1;
            } else if strict && is_sokuon_candidate(chars[i]) {
                // Stray consonant → sokuon (っ).
                // This handles broken romaji like "i s sho" (should be "i ssho")
                // where the 's' is a standalone sokuon marker.
                result.push('っ');
                i += 1;
            } else if !strict {
                result.push(chars[i]);
                i += 1;
            } else {
                i += 1; // skip unmappable in strict mode
            }
        }
    }
    result
}

fn build_romaji_map() -> HashMap<String, String> {
    let pairs: &[(&str, &str)] = &[
        ("a", "あ"),
        ("i", "い"),
        ("u", "う"),
        ("e", "え"),
        ("o", "お"),
        ("ka", "か"),
        ("ki", "き"),
        ("ku", "く"),
        ("ke", "け"),
        ("ko", "こ"),
        ("sa", "さ"),
        ("shi", "し"),
        ("su", "す"),
        ("se", "せ"),
        ("so", "そ"),
        ("ta", "た"),
        ("chi", "ち"),
        ("tsu", "つ"),
        ("te", "て"),
        ("to", "と"),
        ("na", "な"),
        ("ni", "に"),
        ("nu", "ぬ"),
        ("ne", "ね"),
        ("no", "の"),
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
        ("ma", "ま"),
        ("mi", "み"),
        ("mu", "む"),
        ("me", "め"),
        ("mo", "も"),
        ("ya", "や"),
        ("yu", "ゆ"),
        ("yo", "よ"),
        ("ra", "ら"),
        ("ri", "り"),
        ("ru", "る"),
        ("re", "れ"),
        ("ro", "ろ"),
        ("wa", "わ"),
        ("wo", "を"),
        ("n", "ん"),
        ("ga", "が"),
        ("gi", "ぎ"),
        ("gu", "ぐ"),
        ("ge", "げ"),
        ("go", "ご"),
        ("za", "ざ"),
        ("ji", "じ"),
        ("zu", "ず"),
        ("ze", "ぜ"),
        ("zo", "ぞ"),
        ("da", "だ"),
        ("di", "ぢ"),
        ("du", "づ"),
        ("de", "で"),
        ("do", "ど"),
        ("ba", "ば"),
        ("bi", "び"),
        ("bu", "ぶ"),
        ("be", "べ"),
        ("bo", "ぼ"),
        ("pa", "ぱ"),
        ("pi", "ぴ"),
        ("pu", "ぷ"),
        ("pe", "ぺ"),
        ("po", "ぽ"),
        ("kya", "きゃ"),
        ("kyu", "きゅ"),
        ("kyo", "きょ"),
        ("sha", "しゃ"),
        ("shu", "しゅ"),
        ("sho", "しょ"),
        ("cha", "ちゃ"),
        ("chu", "ちゅ"),
        ("cho", "ちょ"),
        ("nya", "にゃ"),
        ("nyu", "にゅ"),
        ("nyo", "にょ"),
        ("hya", "ひゃ"),
        ("hyu", "ひゅ"),
        ("hyo", "ひょ"),
        ("mya", "みゃ"),
        ("myu", "みゅ"),
        ("myo", "みょ"),
        ("rya", "りゃ"),
        ("ryu", "りゅ"),
        ("ryo", "りょ"),
        ("gya", "ぎゃ"),
        ("gyu", "ぎゅ"),
        ("gyo", "ぎょ"),
        ("ja", "じゃ"),
        ("ju", "じゅ"),
        ("jo", "じょ"),
        ("bya", "びゃ"),
        ("byu", "びゅ"),
        ("byo", "びょ"),
        ("pya", "ぴゃ"),
        ("pyu", "ぴゅ"),
        ("pyo", "ぴょ"),
        ("kka", "っか"),
        ("kki", "っき"),
        ("kku", "っく"),
        ("kke", "っけ"),
        ("kko", "っこ"),
        ("ssa", "っさ"),
        ("sshi", "っし"),
        ("ssu", "っす"),
        ("sse", "っせ"),
        ("sso", "っそ"),
        ("tta", "った"),
        ("tchi", "っち"),
        ("ttsu", "っつ"),
        ("tte", "って"),
        ("tto", "っと"),
        ("ppa", "っぱ"),
        ("ppi", "っぴ"),
        ("ppu", "っぷ"),
        ("ppe", "っぺ"),
        ("ppo", "っぽ"),
        ("dda", "っだ"),
        ("ddi", "っぢ"),
        ("ddu", "っづ"),
        ("dde", "っで"),
        ("ddo", "っど"),
        ("gga", "っが"),
        ("ggi", "っぎ"),
        ("ggu", "っぐ"),
        ("gge", "っげ"),
        ("ggo", "っご"),
        ("bba", "っば"),
        ("bbi", "っび"),
        ("bbu", "っぶ"),
        ("bbe", "っべ"),
        ("bbo", "っぼ"),
        ("ffa", "っふぁ"),
        ("ffi", "っふぃ"),
        ("ffe", "っふぇ"),
        ("ffo", "っふぉ"),
        ("ffya", "っふゃ"),
        ("ffyu", "っふゅ"),
        ("ffyo", "っふょ"),
        ("aa", "あー"),
        ("ii", "いー"),
        ("uu", "うー"),
        ("ee", "えー"),
        ("oo", "おー"),
        // Sokuon + youon (missing entries for common combinations)
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

    #[test]
    fn converts_basic_syllables() {
        assert_eq!(romaji_to_hiragana("a"), "あ");
        assert_eq!(romaji_to_hiragana("ka"), "か");
        assert_eq!(romaji_to_hiragana("ki"), "き");
        assert_eq!(romaji_to_hiragana("ku"), "く");
        assert_eq!(romaji_to_hiragana("ke"), "け");
        assert_eq!(romaji_to_hiragana("ko"), "こ");
    }

    #[test]
    fn converts_dakuon() {
        assert_eq!(romaji_to_hiragana("ga"), "が");
        assert_eq!(romaji_to_hiragana("ji"), "じ");
        assert_eq!(romaji_to_hiragana("zu"), "ず");
        assert_eq!(romaji_to_hiragana("de"), "で");
        assert_eq!(romaji_to_hiragana("bo"), "ぼ");
    }

    #[test]
    fn converts_youon() {
        assert_eq!(romaji_to_hiragana("kya"), "きゃ");
        assert_eq!(romaji_to_hiragana("shu"), "しゅ");
        assert_eq!(romaji_to_hiragana("cho"), "ちょ");
        assert_eq!(romaji_to_hiragana("nya"), "にゃ");
        assert_eq!(romaji_to_hiragana("ryo"), "りょ");
    }

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

    #[test]
    fn preserves_non_japanese() {
        assert_eq!(romaji_to_hiragana("123"), "123");
        assert_eq!(romaji_to_hiragana("!"), "!");
    }

    #[test]
    fn converts_space_separated_words() {
        assert_eq!(romaji_to_hiragana("a za ya ka"), "あざやか");
        assert_eq!(romaji_to_hiragana("na ru"), "なる");
    }

    #[test]
    fn handles_particle_wa() {
        assert_eq!(romaji_to_hiragana("wa"), "わ");
    }

    #[test]
    fn handles_empty_input() {
        assert_eq!(romaji_to_hiragana(""), "");
    }

    #[test]
    fn handles_mixed_content() {
        assert_eq!(romaji_to_hiragana("shi ki sa i"), "しきさい");
    }
}
