# Agent Note: Generate complete song teaching videos

Status: implemented

English | [中文](2026-10-03-song-teaching-video.zh.md)

## Problem

Naming a song and a presentation form should produce a complete teaching video with the recording, progressive original and romaji highlighting, Ruby, Chinese translations, contextual vocabulary, expression explanations, and playback progress. The existing lyric display and JSON output do not carry the complete learning content or perform video encoding.

Accurate QRC intervals can have a different time origin from the selected recording. Source preparation must establish the recording-specific lyric offset before video export.

## Decision

The workflow separates deterministic source preparation and rendering from language-content authorship. The Rust domain layer retrieves and decrypts authoritative lyric and reading sources. The source command preserves recording identity and timing in a project. The working Agent generates and reviews the complete teaching content through a discoverable [project Skill](../../../skills/generate-teaching-video/SKILL.md). Pillow composes landscape or portrait frames, and FFmpeg encodes the selected complete recording.

Automatic calibration uses known-lyric acoustic matching at the beginning, middle, and end of the recording to establish one common lyric offset. Text token probability screens measurements, and residuals and regional agreement establish their statistical consistency. Preserving the QRC intervals retains their accurate relative timing; applying the accepted offset in the shared renderer gives previews, scene seeking, highlighting, and teaching pagination one recording-specific timeline. The selected audio retains its complete duration.

Teaching entries use normalized unique sung text as their key. This gives repeated lyrics one reviewed semantic owner while each occurrence retains its own source timing. Source Ruby remains authoritative for contextual and deliberately sung readings. The renderer validates fields, source coverage, glyph availability, and layout capacity before encoding; semantic accuracy remains the language review's responsibility.

Shared row baselines preserve the font's natural glyph positions. White text and its highlight share raster masks so color progression preserves those positions.

The local post-production editor uses the same Pillow composition as video export. An independent edit document records display text, visibility, deleted components, and per-orientation placement; prepared source timing and reviewed teaching inputs retain their identity. The [component-selection decision](2026-10-09-video-component-selection.md) owns selection, batch operations, and explicit layout policy. Cover quantization supplies the highlight hue with readable contrast. Sharing this composition gives the preview and exported MP4 one geometry authority.

Video card materials are composited with Pillow and NumPy at the final component positions. This makes their refraction, tint, highlights, and shadows part of the encoded picture, while preserving the shared text geometry. Computing the material when a teaching page is composed keeps optical processing outside the per-frame highlight loop.

The [user tutorial](../../../../docs/user/teaching-video.md) owns the reusable execution procedure, and the [subsystem reference](../../../../docs/subsystems/teaching-video.md) owns the runtime contract. The Skill routes natural-language requests through those maintained interfaces and returns the complete MP4 together with verifiable evidence.

## Alternatives considered

**A dedicated runtime language service** introduces a model-service boundary into a natural-language workflow the working Agent already performs. Agent-authored teaching content keeps contextual translation and review within the active request, with a complete content artifact for the deterministic renderer.

**Dictionary-only teaching generation** can supply lexical entries but does not establish a sentence's contextual translation or explain the constructions present in it. The workflow combines authoritative readings and consulted reference material with reviewed language content.

## Consequences

Online sources can have different recording versions or incomplete timing and reading data. Source identity, duration checks, complete Ruby mapping, and source-text matching reject these mismatches. Ambiguous song names require enough metadata to identify the requested recording. Automatic calibration stops on insufficient qualifying measurements or inconsistent offsets across the three song regions.

Language accuracy requires full contextual review of translations, readings, lemmas, conjugations, expressions, and supported JLPT labels. The working Agent owns that review for every unique sung line. Fonts and layout capacity are validated against the actual complete content before a full export.

Complete exports preserve the source recording's beginning and end with a zero audio offset. Prepared inputs can be reused after their current source, calibration, and teaching review. The accepted calibration report binds measured anchors to the audio and original lyric data and preserves model provenance. Loading repeats the anchor analysis and source-hash checks. New preparation uses an empty directory, and rendering requires the inputs and calibration report to remain unchanged throughout encoding. The captured input hashes bind the verification evidence to the complete exported file.

## Testing

The complete source projects for `spiral / LONGMAN` and `春日影 / MyGO!!!!!` contain 53 and 45 sung lines, preserving all 84 and 89 source Ruby spans respectively. The second song retains backing-vocal text, whitespace, and zero-duration whitespace intervals in the rendered project.

[`calibration-e2e.py`](../../../../tools/teaching-video/calibration-e2e.py) calibrates the real `spiral` recording with the default model and verifies the complete source intervals and input hashes remain intact. Its accepted offset is `-60 ms`, with 66 accepted anchors among 90 that pass the token probability threshold. Adding `1,250 ms` of leading silence produces an offset of `1,193 ms`, measuring a `1,253 ms` change with `3 ms` error. Both orientation checks preserve each word interval and apply the same offset. Replacing the audio causes the renderer to reject the original calibration report. `work/calibration-e2e-20261009/verification.json` records these checks; its full-song media report covers `231,100 ms`, `6,933` frames, and `115` sampled frames, with source-audio correlation `0.9984967661935833`.

The `node tools/teaching-video/python.mjs calibrate` wrapper also calibrates `春日影` successfully. `work/calibration-haruhikage-final/project.calibration.json` records an offset of `-71 ms`, 72 accepted anchors among 88 that pass the token probability threshold, and passing statistics for all three regions. These reports verify regional statistical consistency and source identity through hashes.

The real editor HTTP checks load calibrated session times, request a preview, save the edit document, and export and verify an eight-second clip. `work/calibration-editor-e2e-20261009/verification.json` records the `-60 ms` lyric offset, zero audio offset, unchanged source times, saved edit hash, and passed media export.

The headless Chrome DOM checks exercise Next lyric, the middle and final lyric selections, and Previous lyric. Each preview request uses the selected source start plus `-60 ms` and the editor's `1 ms` scene-switch increment. The preview clock, DOM clock, selected lyric, and audio position agree after each action. `work/calibration-browser-e2e-20261009/verification.json` records all four actions, preserved source data, and no page errors.

Repeat the calibration and full-song media checks with a retained source project and a new evidence directory:

```sh
tools/teaching-video/.venv/bin/python tools/teaching-video/calibration-e2e.py \
  --project work/spiral/project.json \
  --teaching work/spiral/teaching.json \
  --font /absolute/path/to/CJK-font.ttf \
  --evidence-dir work/calibration-verification
```

The retained editor scripts start their own real service and write separate evidence directories:

```sh
node work/lyric-calibration-editor-e2e.mjs \
  --project work/calibration-e2e-20261009/project.json \
  --teaching work/spiral/teaching.json \
  --font /absolute/path/to/CJK-font.ttf \
  --evidence work/calibration-editor-verification
node work/lyric-calibration-browser-e2e.mjs \
  --project work/calibration-e2e-20261009/project.json \
  --teaching work/spiral/teaching.json \
  --font /absolute/path/to/CJK-font.ttf \
  --evidence work/calibration-browser-verification
```

The edited `spiral` landscape and portrait media reports record full decoding, stream checks, teaching-page comparisons, and source-audio comparisons. Their reports contain 115 and 114 sampled encoded frames, covering 59 and 58 teaching pages. Independent font-raster and encoded-frame checks cover 1,070 and 1,056 original-character samples; the maximum measured vertical offset is zero in both reports. The source-audio correlation is 0.998497.

The browser editing and complete media checks can be repeated from the repository root using a new editing directory:

```sh
pnpm video:edit --project work/calibration-e2e-20261009/project.json --teaching work/spiral/teaching.json --font /absolute/path/to/CJK-font.ttf --workspace work/editor-verification --port 49604
pnpm video:editor-e2e --url http://127.0.0.1:49604 --evidence-dir work/editor-verification/evidence
```

The editor retains an immutable edit snapshot, MP4, render manifest, logs, and sampled verification frames for each export. The E2E evidence directory contains the interaction report, actual preview images and component geometry, and the full-song verification reports. It checks direct text editing, timed words and Ruby, teaching text, dragging, group movement, deletion with automatic reflow, undo, redo, independent orientation positions, saving, reloading, and responsive preview ratios.

[`editor-glass-material-e2e.py`](../../../../tools/teaching-video/editor-glass-material-e2e.py) checks real landscape and portrait previews through the editing service. The checks measure card-background variation, edge highlights, and at least 4.5:1 contrast for white, muted, and accent text. Moving and deleting cards must preserve material placement and automatic layout; deleting every component must leave the empty background. The script saves actual preview images and a repeatable `glass-material-e2e.json` report.

Both songs pass landscape and portrait editor rendering and cover-color contrast checks. HTTP and media E2E checks reject blank text, missing glyphs, picture overflow, multiline timed fragments, and visible text in zero-duration whitespace intervals. `春日影` clips verify deletion of every component and positioning an original glyph at the picture edge; the latter covers 112 glyph samples with zero vertical offset. The clips have a source-audio correlation of 0.998384. Source project, teaching, cover, and audio hashes remain unchanged.

Shared lyric changes require the complete fixture checks; teaching-video changes require real source preparation, complete content review, full exports, and their media evidence. Documentation verification, prose lint, and whitespace checks cover the maintained bilingual reference and tutorial.
