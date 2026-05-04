use utabuild_cli::qm_decrypt;
use utabuild_cli::qrc_parser;
use utabuild_cli::romaji;
use utabuild_cli::ruby_align;

#[test]
fn full_pipeline_e2e_with_sample_qrc() {
    // Simulate QRC XML for "起死開戦"
    let qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[ti:起死開戦]
[ar:millsage]
[0,500]起(0,50)死(51,50)開(102,50)戦(153,50)
[501,500]鮮(0,50)や(51,50)か(102,50)な(153,50)る(204,50)
"/>
  </LyricInfo>
</QrcInfos>"#;

    let original_lines = qrc_parser::parse_qrc(qrc).expect("Should parse QRC");
    assert_eq!(original_lines.len(), 2, "Two lyric lines");
    assert_eq!(
        original_lines[0].words.len(),
        4,
        "First line: 4 words (起死開戦)"
    );
    assert_eq!(
        original_lines[1].words.len(),
        5,
        "Second line: 5 words (鮮やかなる)"
    );
}

#[test]
fn qrc_parser_with_romaji_alignment() {
    let orig_qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[0,500]起(0,50)死(51,50)開(102,50)戦(153,50)
"/>
  </LyricInfo>
</QrcInfos>"#;

    let roma_qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[0,500]o(0,50)ki(51,50)shi(102,50)ka(153,50)i(204,50)se(255,50)n(306,50)
"/>
  </LyricInfo>
</QrcInfos>"#;

    let original = qrc_parser::parse_qrc(orig_qrc).unwrap();
    let romaji_lines = qrc_parser::parse_qrc(roma_qrc).unwrap();
    let aligned = qrc_parser::align_romaji_to_original(&original, &romaji_lines);

    assert_eq!(aligned.len(), 1, "One aligned line");
    let (orig_words, roma_words) = &aligned[0];
    assert!(roma_words.is_some(), "Romaji should be found");
    assert_eq!(orig_words.len(), 4, "4 original words");
}

#[test]
fn romaji_to_hiragana_full_words() {
    // "起死開戦" romaji: o ki shi ka i se n
    let hiragana = romaji::romaji_to_hiragana("o ki shi ka i se n");
    assert_eq!(hiragana, "おきしかいせん", "起死開戦 hiragana");

    // "鮮やかなる" romaji: a za ya ka na ru
    let hiragana = romaji::romaji_to_hiragana("a za ya ka na ru");
    assert_eq!(hiragana, "あざやかなる", "鮮やかなる hiragana");

    // "色彩の" romaji: shi ki sa i no
    let hiragana = romaji::romaji_to_hiragana("shi ki sa i no");
    assert_eq!(hiragana, "しきさいの", "色彩の hiragana");
}

#[test]
fn ruby_alignment_produces_correct_elements() {
    let hiragana = "あざやかなるしきさいの";
    let elements = ruby_align::align_ruby_to_text("鮮やかなる色彩の", hiragana);

    // Should be: ruby("鮮","あざ") + text("やかなる") + ruby("色彩","しきさい") + text("の")
    assert_eq!(elements.len(), 4, "Should produce 4 elements");

    // Verify first ruby element
    assert_eq!(elements[0].element_type, "ruby");
    assert_eq!(elements[0].base.as_deref(), Some("鮮"));
    assert_eq!(elements[0].ruby.as_deref(), Some("あざ"));

    // Verify text elements
    assert_eq!(elements[1].element_type, "text");
    assert_eq!(elements[1].base.as_deref(), Some("やかなる"));

    // Verify second ruby
    assert_eq!(elements[2].element_type, "ruby");
    assert_eq!(elements[2].base.as_deref(), Some("色彩"));
    assert_eq!(elements[2].ruby.as_deref(), Some("しきさい"));

    // Verify trailing text
    assert_eq!(elements[3].element_type, "text");
    assert_eq!(elements[3].base.as_deref(), Some("の"));
}

#[test]
fn qm_decrypt_handles_invalid_input_gracefully() {
    // Non-hex input
    assert!(qm_decrypt::decrypt_qm_lyrics("ZZZZZZZZZZZZZZZZ").is_none());
    // Odd-length hex
    assert!(qm_decrypt::decrypt_qm_lyrics("ABC").is_none());
    // Very short valid hex (1 byte, not 8-aligned for 3DES)
    assert!(qm_decrypt::decrypt_qm_lyrics("00").is_none());
}

#[test]
fn full_end_to_end_mock_pipeline() {
    // Simulate the entire pipeline: parse QRC → align → romaji→hiragana → ruby align
    let orig_qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[ti:起死開戦]
[ar:millsage]
[0,1000]起(0,50)死(51,50)開(102,50)戦(153,50)
"/>
  </LyricInfo>
</QrcInfos>"#;

    let roma_qrc = r#"<?xml version="1.0" encoding="utf-8"?>
<QrcInfos>
  <LyricInfo LyricCount="1">
    <Lyric_1 LyricType="1" LyricContent="
[0,1000]o(0,50)ki(51,50)shi(102,50)ka(153,50)i(204,50)se(255,50)n(306,50)
"/>
  </LyricInfo>
</QrcInfos>"#;

    // Step 1: Parse QRC
    let original = qrc_parser::parse_qrc(orig_qrc).unwrap();
    let romaji_lines = qrc_parser::parse_qrc(roma_qrc).unwrap();
    assert_eq!(original.len(), 1);
    assert_eq!(romaji_lines.len(), 1);

    // Step 2: Align
    let aligned = qrc_parser::align_romaji_to_original(&original, &romaji_lines);
    assert_eq!(aligned.len(), 1);

    // Step 3: Convert romaji → hiragana and produce LyricElements
    let (orig_words, roma_words) = &aligned[0];
    let orig_text: String = orig_words.iter().map(|w| w.text.as_str()).collect();
    let roma_text: String = roma_words
        .as_ref()
        .unwrap()
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let hiragana = romaji::romaji_to_hiragana(&roma_text);
    let elements = ruby_align::align_ruby_to_text(&orig_text, &hiragana);

    // Should produce ruby elements for all characters since everything is kanji
    assert!(!elements.is_empty(), "Should produce ruby elements");
    assert_eq!(orig_text, "起死開戦", "Original text preserved");
    assert_eq!(hiragana, "おきしかいせん", "Correct hiragana conversion");

    // Check we have at least one ruby element
    let ruby_count = elements.iter().filter(|e| e.element_type == "ruby").count();
    assert!(ruby_count > 0, "Should have ruby annotations");
}
