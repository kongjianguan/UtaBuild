# CLI and Lyric Pipeline

English | [中文](cli.zh.md)

The `cli` crate is UtaBuild's shared lyric domain layer. It is compiled as `utabuild-cli` and imported by the Tauri backend through a local path dependency.

## Responsibilities

- Search UtaTen with pagination, rate limiting, response decoding, and HTML extraction.
- Search and fetch lyrics from QQ Music and NetEase through source-specific adapters.
- Parse LRC, YRC, and QRC input into normalized lyric elements.
- Align romaji or source timing with kanji and kana to produce Ruby annotations.
- Persist search and lyric caches, manage history, and serialize JSON or HTML output.

## Core contracts

`LyricElement` is the shared render model. `text` carries `base`; `ruby` carries `base` and `ruby`; `linebreak` carries neither. `LyricsSearchResponse` represents search and selection state, while `LyricsOutput` represents structured lyric output.

`UtaTenSearcher` owns source preference routing and provider coordination. `ArtworkSourcePreference` and `LyricSourcePreference` normalize user settings to the supported source set: UtaTen, QQ Music, NetEase, or automatic selection.

## Parsing and alignment

`lrc_parser.rs` handles timed LRC/YRC lines and can align a romaji track by time. `qrc_parser.rs` parses QQ Music's XML-like word timing format and performs character-level overlap alignment. `ruby_align.rs` provides the general text-to-reading alignment and sanitization path. Missing, empty, or non-overlapping readings fall back to plain text.

## Persistence and output

`CacheManager` owns in-memory search and lyric caches. The file cache and history modules persist JSON under the platform data or cache directory. `output.rs` defines JSON fields consumed by the CLI, Tauri backend, and Android bridge; `output_html.rs` escapes titles, artists, URLs, and lyric text before rendering HTML.

## Verification

Run `cargo test --manifest-path cli/Cargo.toml`. The integration test in `cli/tests/lyrics_pipeline_test.rs` compares QRC fixture output with expected JSON and covers empty or non-overlapping romaji fallback.
