# UtaBuild Subsystems

English | [中文](README.zh.md)

This directory owns stable subsystem contracts. Read [architecture.md](../architecture.md) first for composition and dependency direction; use the pages below for vocabulary, boundaries, and focused verification.

## Subsystem map

| Subsystem | Owns | Primary sources |
|---|---|---|
| [CLI and lyric pipeline](cli.md) | Search sources, lyric parsing, Ruby alignment, caching, history, and JSON/HTML output | `cli/src/` |
| [Frontend](frontend.md) | TypeScript views, state, Tauri invocation, Ruby DOM rendering, settings, and export controls | `src/ts/`, `src/index.html` |
| [Tauri backend](tauri-backend.md) | Desktop/mobile commands, shared `AppState`, cache orchestration, persistence, logging, and HTML export | `src-tauri/src/lib.rs` |
| [Android LSPosed bridge](android-lsposed.md) | Salt Player hooks, structured lyric transport, Ruby alignment, and Canvas rendering | `src-tauri/android-lsposed/` |

## Ownership rule

Types, command payloads, source names, cache keys, and output fields are source-owned facts. Keep this directory focused on the contract and link to the declaration when a detail is likely to drift. Do not build a second exhaustive symbol catalog here.

## Verification

Run `cargo test --manifest-path cli/Cargo.toml` for the lyric pipeline and `pnpm run build:ts` for the frontend. Android and Tauri changes require their platform-specific build or test command when the environment provides it.
