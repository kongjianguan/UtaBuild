# UtaBuild Development

English | [中文](development.zh.md)

UtaBuild uses Rust for the CLI and Tauri backend, TypeScript and SCSS for the frontend, and Java for the Android LSPosed bridge. Work from the repository root unless a command names a subproject.

## Prerequisites

Install Rust 1.70 or newer, Node.js 18 or newer, Tauri CLI v2, and the frontend dependencies. Android work additionally requires JDK 17+, Android Studio 2024.1+, Android SDK API 34+, Android NDK 27+, and the generated Gradle wrapper.

The repository contains `pnpm-lock.yaml`; use pnpm for frontend dependency and script commands.

## Daily commands

```sh
pnpm install
pnpm run build:ts
pnpm run build:scss
pnpm run lint
cargo test --manifest-path cli/Cargo.toml
```

Run `pnpm run build` before `cargo tauri dev` or `cargo tauri build`; the Tauri configuration invokes the frontend build through `beforeDevCommand` and `beforeBuildCommand` as well.

## Desktop and Android

```sh
cargo tauri dev
cargo tauri build
cargo tauri android init
cargo tauri android dev
cargo tauri android build --target aarch64 --apk
```

Android initialization creates or updates `src-tauri/gen/android/`. Treat generated Android output as build-owned unless a platform change requires a checked-in change.

## CLI fixtures

The CLI integration test in `cli/tests/lyrics_pipeline_test.rs` reads QRC fixtures and compares the complete `LyricsOutput` JSON with the expected file. After an intentional fixture change, regenerate the expected JSON at that fixture's existing expected-file path, then review the complete output manually:

```sh
cargo run --manifest-path cli/Cargo.toml --release -- search \
  --title "<title>" --artist "<artist>" \
  --output <fixture-expected-json-path>
```

Run the focused test again after reviewing the fixture:

```sh
cargo test --manifest-path cli/Cargo.toml --test lyrics_pipeline_test
```

## Change routing

Change shared lyric behavior in `cli/` first. Update the Tauri command only when the application boundary needs new orchestration or serialization. Update frontend types and views when the JSON contract changes. Update the Android bridge when the Salt Player transport or rendering contract changes.

Every non-trivial architecture, behavior, testing, or process change adds or updates an Agent Note. Run the documentation verifier and the narrowest affected code checks before reporting completion.
