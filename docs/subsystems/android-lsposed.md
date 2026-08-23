# Android LSPosed Bridge

English | [中文](android-lsposed.zh.md)

The Android LSPosed source tree integrates UtaBuild with Salt Player. It runs in the target player process, so it uses a low-invasive bridge instead of calling the Tauri/Rust command surface directly.

## Data flow

`UtaBuildSaltModule` hooks Salt application and activity lifecycle, song-open/update events, the lyric-line accessor, and `Canvas.drawText`. `UtaBuildLyricProvider` resolves structured lyrics in this order: the UtaBuild ContentProvider, a local Downloads fixture, then the loopback HTTP endpoint.

The provider returns `StructuredLyrics`, which contains lines and character-range `RubyAnnotation` values. `RubyAlignmentEngine` aligns those annotations with the lyric text observed in Salt. `RubyCanvasInjector` and `SaltRubyRenderer` draw the reading above the original character while preserving the host renderer for ordinary text.

## ContentProvider contract

`UtaBuildLyricContentProvider` exposes the `fyi.kongjianguan.utabuild.lyrics` authority. The `lyrics` query returns structured JSON; `pending` accepts a Salt launch request; `logs` accepts bridge log writes and returns logs; `settings` returns synchronized LSP settings. The provider is read-only for lyric queries and uses mirrored application files as its transport storage.

## Failure behavior

Bridge lookup failures are non-fatal. The provider falls back to the next source, and the Salt hook keeps the host lyrics when no usable Ruby data is available. File names and log fields are sanitized before crossing the bridge.

## Verification

Verify Java compilation through the Android build when the generated project and SDK are available. For algorithm changes, keep `RubyAlignmentEngine` and structured JSON fixtures focused on character ranges, kana normalization, duplicate suppression, and plain-text fallback.
