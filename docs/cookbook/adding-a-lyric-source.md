# Adding a Lyric Source

English | [中文](adding-a-lyric-source.zh.md)

Use this procedure when adding a new lyrics or artwork provider. The source must preserve the shared `LyricElement` and response contracts.

## 1. Define the source contract

Add the source preference and its accepted setting values in `cli/src/searcher.rs`. Define the source identifier used in URLs and cache keys. Record whether the source returns plain lyrics, timed lyrics, Ruby annotations, artwork, or metadata.

## 2. Implement retrieval

Keep network access in the CLI crate. Add source-specific request, decoding, parsing, and error handling beside the existing provider code. Enforce the source's rate limit and do not cache an HTTP error as an empty result.

## 3. Route the application

Update `src-tauri/src/lib.rs` when the source needs command-level routing, cache behavior, or response normalization. Update `src/ts/types.ts`, settings, source controls, and result handling when the JSON contract or user choice changes.

## 4. Test the pipeline

Add focused parser or alignment tests and a fixture when the source has stable sample data. Run:

```sh
cargo test --manifest-path cli/Cargo.toml
pnpm run build:ts
```

## 5. Update documentation

Update the owning subsystem page, user guide, and `docs/architecture.md` when the source changes a boundary or flow. Add or update an implemented Agent Note when the source introduces a durable decision, cache rule, or compatibility obligation.
