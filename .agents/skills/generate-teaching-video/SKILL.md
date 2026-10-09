---
name: generate-teaching-video
description: Generate a complete Japanese-song teaching MP4 from a song name and requested horizontal or vertical layout, with timed lyrics, Ruby, romaji, Chinese translations, vocabulary, and expression explanations.
---

# Generate a teaching video

Use this workflow for requests such as “生成 spiral 教学视频，横屏 16:9” or “制作纵屏日语歌词教学视频”. Read the [user procedure](../../../docs/user/teaching-video.zh.md) and [teaching-video contract](../../../docs/subsystems/teaching-video.zh.md) before execution. Runtime declarations and schemas own accepted fields and command options.

## Resolve the request and prepare sources

Resolve the title and requested `landscape` or `portrait` layout. Preserve any supplied artist, rendering options, and explicit song identifier. Use the named layout; choose `landscape` when the request leaves the layout unspecified. A title-only request starts with the real title search. If the response contains several matching artists, inspect the saved search response and the user's song context to identify the intended recording. Request the artist only when that evidence cannot identify the song; do not choose a singer merely because their result appears first.

Inspect existing prepared inputs before preparing the song again. Reuse a project when its current song metadata, source evidence, recording, complete timing, readings, and accepted calibration match the request; reuse its teaching file after checking the current content against those inputs. Require `lyric_offset_ms`, `calibration_path`, and valid calibration evidence bound to the current audio and original lyric data. A new layout or accent can use the same reviewed source and teaching files. Keep inputs referenced by existing render manifests intact.

When new source preparation is needed, run `pnpm video:prepare --title <title> --output work/<song>` from the repository root in an unused output directory. Add `--artist <artist>` when the user supplied the artist or trusted song context identifies them. Use the actual supported song-identifier option when supplied. The command automatically selects and checks a complete official recording against the song's title, artist, album, and duration, then calibrates known lyrics against the recording before publishing the project. Keep all intermediate files and the downloaded model cache in the ignored `work/` directory. Inspect `song.json`, `source.json`, `project-source.json`, `project.json`, `project.calibration.json`, `project.measurements.json`, `teaching-input.json`, and the saved audio-selection evidence before writing teaching content. Confirm complete original lyrics, all source Ruby spans, the complete Japanese romaji text and its timing, the full recording, and accepted measurements across all three song regions. Original QRC intervals remain intact; the shared renderer applies the accepted overall lyric offset.

For an independently retained source project, run `pnpm video:calibrate --project <source-project> --output <unused-project-path>` and use its calibrated output. The calibration command declaration owns model options and the timing module owns acceptance criteria. Completed acoustic measurements remain available after statistical rejection. Resolve source or recording failures and require calibration success before rendering.

## Author and review teaching content

The working Agent writes `teaching.json` using file editing tools. Cover every unique sung line in `teaching-input.json`; repetitions reuse the same reviewed entry. Preserve each line's exact `text`. Write a natural Chinese translation and vocabulary and expression explanations appropriate to that line's meaning and grammar. Consult authoritative dictionary or grammar sources for uncertain readings, senses, conjugations, and constructions.

Use the source Ruby for contextual readings, including deliberate sung readings that differ from a dictionary reading. For each vocabulary item, distinguish the exact lyric surface from its lemma and explain the actual conjugation when applicable. Mark a JLPT level only when a consulted source supports it; otherwise use `null`. Explain the real expressions and grammatical constructions present in the line. Do not insert generic dictionary placeholders, fabricated definitions, or a mechanical phrase repeated across entries. Full coverage means all sung lines receive actual reviewed teaching content.

Read the complete resulting JSON and compare each entry with the original line, source readings, and adjacent context. Verify surface substrings, reading consistency, sense selection, translation accuracy, conjugations, and full entry coverage. The renderer's schema and coverage checks do not establish language accuracy.

## Render and verify the deliverable

Use `pnpm video:render`, the renderer's current options, and an installed CJK font containing all required glyphs. The pnpm entry point uses `tools/teaching-video/.venv` for the current platform. Select an unused output filename and render the complete recording in the requested layout with the default zero audio offset. Full-song exports require `--audio-offset-ms 0` so the source recording's beginning and end remain present. Short `--start-ms`/`--end-ms` renders are useful for layout checks; they do not satisfy a request for the complete song. The renderer combines the selected recording with progressive original and romaji highlighting, Ruby, Chinese translations, vocabulary cards, expression explanations, and playback progress.

Run `pnpm video:verify --manifest <output>.render.json --require-full-song` and inspect the evidence directory's `verification.json`. Require `status: passed` and `song_scope: full`. Confirm the full duration, video dimensions, audio stream, complete decoder success, lyric and romaji coverage, frame evidence for every teaching page, and the accepted offset and regional residuals in `lyric_alignment`. The verifier checks calibration hashes and repeats the anchor analysis using the captured report. Resolve failures in the owning source or teaching file and rerun the affected export and verification. Return a clickable link to the complete MP4 and its verification artifact; never describe a preview render or a started encoder as completed output.
