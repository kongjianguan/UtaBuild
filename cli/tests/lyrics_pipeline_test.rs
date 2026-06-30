use utabuild_cli::output::LyricsOutput;
use utabuild_cli::qrc_parser;

/// Helper: run pipeline on QRC fixtures, compare JSON output against expected fixture.
fn run_fixture_test(slug: &str, title: &str, artist: &str, url: &str) {
    let (orig, roma, expected) = match slug {
        "kishikaisen" => (
            include_str!("fixtures/kishikaisen_original.qrc"),
            include_str!("fixtures/kishikaisen_romaji.qrc"),
            include_str!("fixtures/kishikaisen_expected.json"),
        ),
        "utakoe_kizuna" => (
            include_str!("fixtures/utakoe_kizuna/original.qrc"),
            include_str!("fixtures/utakoe_kizuna/romaji.qrc"),
            include_str!("fixtures/utakoe_kizuna/expected.json"),
        ),
        "haruhikage" => (
            include_str!("fixtures/haruhikage/original.qrc"),
            include_str!("fixtures/haruhikage/romaji.qrc"),
            include_str!("fixtures/haruhikage/expected.json"),
        ),
        "aliez" => (
            include_str!("fixtures/aliez/original.qrc"),
            include_str!("fixtures/aliez/romaji.qrc"),
            include_str!("fixtures/aliez/expected.json"),
        ),
        "shinzou" => (
            include_str!("fixtures/shinzou/original.qrc"),
            include_str!("fixtures/shinzou/romaji.qrc"),
            include_str!("fixtures/shinzou/expected.json"),
        ),
        _ => panic!("unknown fixture: {slug}"),
    };

    let elements = qrc_parser::process_qrc_pipeline(orig, roma)
        .unwrap_or_else(|| panic!("pipeline failed for {slug}"));

    let output = LyricsOutput::success(
        title.to_string(),
        artist.to_string(),
        url.to_string(),
        &elements,
    );

    let actual: serde_json::Value =
        serde_json::from_str(&output.to_json().unwrap()).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(expected).unwrap();

    if actual != expected {
        let diff = json_diff(&expected, &actual, "");
        panic!(
            "Output differs for '{slug}':\n{diff}\n\n\
             Regenerate with:\n  cargo run --release -- search --title <title> ... > cli/tests/fixtures/{slug}/expected.json"
        );
    }
}

#[test]
fn kishikaisen_matches_expected() {
    run_fixture_test("kishikaisen", "起死開戦", "millsage", "qq_music:003kP6E71r8DAn");
}

#[test]
fn utakoe_kizuna_matches_expected() {
    run_fixture_test("utakoe_kizuna", "詩超絆", "MyGO!!!!!", "qq_music");
}

#[test]
fn haruhikage_matches_expected() {
    run_fixture_test("haruhikage", "春日影", "MyGO!!!!!", "qq_music");
}

#[test]
fn aliez_matches_expected() {
    run_fixture_test("aliez", "aLIEz", "SawanoHiroyuki", "qq_music");
}

#[test]
fn shinzou_matches_expected() {
    run_fixture_test("shinzou", "心臓を捧げよ!", "Linked Horizon", "qq_music");
}

fn json_diff(expected: &serde_json::Value, actual: &serde_json::Value, path: &str) -> String {
    match (expected, actual) {
        (serde_json::Value::Number(a), serde_json::Value::Number(b)) if a != b => {
            format!("  {path}: expected {a}, got {b}")
        }
        (serde_json::Value::String(a), serde_json::Value::String(b)) if a != b => {
            format!("  {path}: expected \"{a}\", got \"{b}\"")
        }
        (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) if a != b => {
            format!("  {path}: expected {a}, got {b}")
        }
        (serde_json::Value::Null, serde_json::Value::Null) => String::new(),
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            let mut diffs = Vec::new();
            for i in 0..a.len().max(b.len()) {
                let p = format!("{path}[{i}]");
                match (a.get(i), b.get(i)) {
                    (Some(ae), Some(be)) => {
                        let d = json_diff(ae, be, &p);
                        if !d.is_empty() {
                            diffs.push(d);
                        }
                    }
                    (Some(_), None) => diffs.push(format!("  {p}: expected value, got nothing")),
                    (None, Some(_)) => diffs.push(format!("  {p}: expected nothing, got value")),
                    (None, None) => {}
                }
            }
            diffs.join("\n")
        }
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            let mut diffs = Vec::new();
            let all_keys: std::collections::BTreeSet<&String> =
                a.keys().chain(b.keys()).collect();
            for k in all_keys {
                let p = if path.is_empty() {
                    format!(".{k}")
                } else {
                    format!("{path}.{k}")
                };
                match (a.get(k), b.get(k)) {
                    (Some(ae), Some(be)) => {
                        let d = json_diff(ae, be, &p);
                        if !d.is_empty() {
                            diffs.push(d);
                        }
                    }
                    (Some(_), None) => diffs.push(format!("  {path}: expected value, got nothing")),
                    (None, Some(_)) => diffs.push(format!("  {path}: expected nothing, got value")),
                    (None, None) => {}
                }
            }
            diffs.join("\n")
        }
        (a, b) if a == b => String::new(),
        _ => format!("  {path}: expected {expected}, got {actual}"),
    }
}

#[test]
fn pipeline_invalid_xml_returns_none() {
    assert!(qrc_parser::process_qrc_pipeline("not xml", "<QrcInfos/>").is_none());
    assert!(qrc_parser::process_qrc_pipeline("<QrcInfos/>", "not xml").is_none());
}

#[test]
fn pipeline_empty_romaji_produces_plain_text() {
    let orig = r#"<?xml version="1.0"?>
<QrcInfos>
<LyricInfo LyricCount="1">
<Lyric_1 LyricType="1" LyricContent="
[0,405]起(0,50)死(51,50)開(101,101)戦(203,101)
"/>
</LyricInfo>
</QrcInfos>"#;

    let roma = r#"<?xml version="1.0"?>
<QrcInfos>
<LyricInfo LyricCount="1">
<Lyric_1 LyricType="1" LyricContent="
[0,405]
"/>
</LyricInfo>
</QrcInfos>"#;

    let elements = qrc_parser::process_qrc_pipeline(orig, roma)
        .expect("pipeline should succeed with empty romaji");

    assert!(!elements.is_empty(), "should still produce text elements");
    let ruby_count = elements.iter().filter(|e| e.element_type == "ruby").count();
    assert_eq!(ruby_count, 0, "no romaji means no ruby");
    let text: String = elements.iter()
        .filter_map(|e| e.base.as_deref())
        .collect();
    assert_eq!(text, "起死開戦", "plain text should match original");
}

#[test]
fn pipeline_non_overlapping_romaji_produces_plain_text() {
    let orig = r#"<?xml version="1.0"?>
<QrcInfos>
<LyricInfo LyricCount="1">
<Lyric_1 LyricType="1" LyricContent="
[0,100]起(0,50)死(51,50)
"/>
</LyricInfo>
</QrcInfos>"#;

    let roma = r#"<?xml version="1.0"?>
<QrcInfos>
<LyricInfo LyricCount="1">
<Lyric_1 LyricType="1" LyricContent="
[9999,100]o(0,50)ki(51,50)
"/>
</LyricInfo>
</QrcInfos>"#;

    let elements = qrc_parser::process_qrc_pipeline(orig, roma)
        .expect("pipeline should succeed");

    let ruby_count = elements.iter().filter(|e| e.element_type == "ruby").count();
    assert_eq!(ruby_count, 0, "non-overlapping romaji should not produce ruby");
}
