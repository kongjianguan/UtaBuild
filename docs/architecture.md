# UtaBuild Architecture

English | [中文](architecture.zh.md)

UtaBuild is a cross-platform lyrics application with a reusable Rust lyric pipeline. The Tauri application owns the desktop and mobile command surface, the TypeScript frontend owns presentation and interaction, and the Android LSPosed source tree owns Salt Player integration.

## Composition

```text
src/                         TypeScript, HTML, and SCSS frontend
└── invoke() ────────────────┐
                             ▼
src-tauri/src/lib.rs         Tauri commands and application state
└── utabuild-cli             ▼
cli/src/                     search, parse, align, cache, history, output

src-tauri/android-lsposed/   Salt Player hooks and structured-lyrics bridge
```

The `cli` crate is the shared domain layer. `src-tauri` imports it through a local path dependency, so search, parsing, Ruby alignment, cache models, and output models are not duplicated in the frontend.

## Runtime flow

The normal search flow is:

1. The frontend collects a title, optional artist, source preference, and cache preference.
2. `invoke('search_lyrics')` calls the Tauri command.
3. The Tauri command checks the search cache and delegates to `UtaTenSearcher` for UtaTen, QQ Music, or NetEase.
4. The frontend presents `SearchResponse.results` and selects a result.
5. `invoke('get_lyrics')` fetches or loads structured `ruby_annotations`, persists the saved entry, and may write the Salt bridge cache.
6. `ruby.ts` renders text, Ruby annotations, and line breaks into the lyrics view.

`search_and_get` is the one-shot path. It returns a cached successful result or selects the first result before saving the same structured output.

## Core boundaries

### Frontend

`src/ts/` owns DOM state, view routing, settings, search result interaction, saved songs, log views, export controls, and Ruby rendering. It does not implement network retrieval or lyric parsing. `src/ts/tauri.ts` is the invocation boundary and also supplies a mock path for non-Tauri browser work.

### Tauri backend

`src-tauri/src/lib.rs` owns the `AppState` searcher, Tauri command registration, cache orchestration, saved-lyrics persistence, LSP settings and logs, Salt launch requests, and HTML export. The backend converts Rust results to JSON values consumed by the frontend.

### CLI domain layer

`cli/src/searcher.rs` owns source access and source preference routing. `lrc_parser.rs` and `qrc_parser.rs` convert timed lyric formats into `LyricElement` values. `ruby_align.rs` and the tokenized LRC/YRC path produce `ruby` elements. `cache.rs`, `cache_manager.rs`, and `commands/history.rs` own local persistence and history. `output.rs` and `output_html.rs` own JSON and HTML contracts.

### Android LSPosed bridge

`src-tauri/android-lsposed/` contains Java code that runs inside Salt Player. `UtaBuildSaltModule` observes the Salt lifecycle and lyric drawing path. `UtaBuildLyricProvider` reads structured lyrics from the UtaBuild ContentProvider, local files, or the loopback bridge. `RubyAlignmentEngine` maps UtaBuild annotations onto the lyric text observed by Salt, and `RubyCanvasInjector`/`SaltRubyRenderer` render the reading above the original characters.

## Data contracts

The shared element contract is a list of values with `type: text | ruby | linebreak`. Text uses `base`; Ruby uses `base` and `ruby`. The JSON output wraps those elements in response-specific fields such as `found_title`, `found_artist`, `lyrics_url`, and `ruby_annotations`.

Cache keys are source-aware. QQ Music and NetEase use source-specific identifiers when available; UtaTen uses the lyric URL. Deleting a saved entry also invalidates the in-memory lyric cache so a later cache hit cannot recreate the deleted file.

## Where new behavior goes

- Add source retrieval, parsing, alignment, or shared output behavior to `cli/src/`.
- Add desktop/mobile command orchestration or persistence to `src-tauri/src/lib.rs`.
- Add interaction, view state, or rendering to `src/ts/`.
- Add Salt Player hooks or bridge behavior to `src-tauri/android-lsposed/`.
- Update the owning subsystem page and an Agent Note when the change creates a durable boundary or changes a shared contract.

Do not put source-specific network logic in TypeScript. Do not duplicate Rust domain behavior in Tauri commands when the `cli` crate can own it.
