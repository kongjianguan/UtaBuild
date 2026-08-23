# Tauri Backend

English | [中文](tauri-backend.zh.md)

The Tauri backend in `src-tauri/src/lib.rs` is the application boundary around the shared `utabuild-cli` crate. It owns command registration, application state, platform persistence, bridge files, and response serialization.

## State and commands

`AppState` contains a mutex-protected `UtaTenSearcher` and the LSP logging flag. The registered command groups are lyric search and retrieval, saved lyric management, cache management, Salt launch/bind operations, LSP settings and logs, and HTML export.

`search_lyrics` checks the search cache and routes UtaTen, QQ Music, or NetEase queries. `get_lyrics` handles source-specific lyric retrieval and cache keys before returning `ruby_annotations`. `search_and_get` combines search and first-result selection for the one-shot frontend path.

## Persistence

The backend persists search responses and annotated lyrics through the CLI cache modules. Saved lyric summaries include title, artist, album, cover URL, source URL, timestamp, and annotation count. Deletion invalidates both disk and in-memory lyric state.

LSP settings and logs are written under the application data directory. Salt launch requests and structured lyric bridge files use sanitized title-based paths so the Android provider can read them without receiving Rust command calls.

## Response boundary

Tauri commands return `serde_json::Value` or simple booleans/strings because the frontend invokes them dynamically. Keep field names synchronized with `src/ts/types.ts` and the Android `StructuredLyrics.fromUtaBuildJson` parser when changing `ruby_annotations`, `lines`, metadata, or source identifiers.

## Verification

Run `cargo check --manifest-path src-tauri/Cargo.toml` when the local Tauri toolchain is available. Run the CLI tests for shared behavior and `pnpm run build:ts` when command payloads change. Android bridge changes require the generated Android build or a focused Java test where available.
