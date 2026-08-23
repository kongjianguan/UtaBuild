# UtaBuild User Guide

English | [中文](index.zh.md)

UtaBuild searches lyrics, displays Japanese Ruby readings, and stores selected lyrics for later use. The desktop and Android application share the same Rust lyric pipeline.

## Search and display

Enter a title and optional artist, choose a source when the default result is not suitable, and select a result. The lyrics view renders plain text, Ruby readings, line breaks, album metadata, and cover artwork when the source provides them.

## Saved lyrics

The Songs view lists locally saved lyric entries. It can reload a saved entry, refresh artwork metadata, export HTML, and delete the entry. Cache controls are available in Settings.

## Salt Player integration

The Android LSPosed integration can receive a song launch request from Salt Player, resolve structured lyrics through the UtaBuild bridge, and inject Ruby readings into the Salt Player rendering path. Enable the module and the relevant settings only on a device where the bridge is installed.

## CLI

The CLI supports search, direct URL retrieval, history management, JSON output, and HTML output:

```sh
cargo run --manifest-path cli/Cargo.toml -- search --title "曲名" --artist "歌手"
cargo run --manifest-path cli/Cargo.toml -- history list
```
