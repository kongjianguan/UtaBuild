# Generate a song teaching video

English | [中文](teaching-video.zh.md)

## Request a video

Open UtaBuild in Codex and request a song and layout, for example “生成 spiral 教学视频，横屏 16:9” or “生成 spiral 教学视频，竖屏 9:16”. The [project Skill](../../.agents/skills/generate-teaching-video/SKILL.md) prepares the recording and lyric sources, writes and reviews the teaching content, renders the complete MP4, and verifies the exported file. Include the artist when the song name and request context cannot identify the recording.

The exported picture includes progressively highlighted Japanese lyrics and romaji, Ruby readings, current and neighboring Chinese sentence translations, vocabulary cards, expression explanations, and a playback progress bar over blurred cover artwork. Landscape uses three vocabulary columns; portrait uses two columns and adjusts the lyric and explanation areas to the available width.

## Prepare the environment

The command workflow requires Rust, Node.js, pnpm, Python 3.10 or later, `curl`, `yt-dlp`, FFmpeg, and a CJK font. The Python dependencies include faster-whisper for automatic lyric calibration. Install the repository's JavaScript and Python dependencies from the repository root:

```sh
pnpm install
python3 -m venv tools/teaching-video/.venv
tools/teaching-video/.venv/bin/python -m pip install -r tools/teaching-video/requirements.txt
```

On Windows, the created environment's interpreter is `tools/teaching-video/.venv/Scripts/python.exe`. The `pnpm video:calibrate`, `pnpm video:render`, and `pnpm video:verify` entry points select the project environment's interpreter for the current platform. Choose a font file that contains the Japanese and Chinese glyphs used by the song and teaching content. The [subsystem reference](../subsystems/teaching-video.md) describes source requirements, validation, and failure behavior.

## Prepare a song

Choose a new output directory or an existing empty directory under `work/`:

```sh
pnpm video:prepare --title spiral --artist LONGMAN --output work/spiral
```

Preparation automatically selects a complete official recording by matching its title, artist, album, and duration with the selected song. It saves the song metadata, original lyric response, decoded source lyrics and Ruby, source project, unique teaching input, audio, and cover artwork under the output directory. It then measures known lyrics against the recording in three song regions and produces the calibrated `project.json` only after those measurements support a common offset. Inspect the retained source evidence, recording identity, and `project.calibration.json` before authoring the learning content.

The first calibration downloads the default `small` acoustic model into `work/whisper-models/`; subsequent calibrations reuse that cache. `project-source.json` preserves the source intervals. `project.json` stores the accepted `lyric_offset_ms` and calibration-report path, while `project.measurements.json` preserves the measured positions. The audio keeps its complete original timeline. The [calibration reference](../subsystems/teaching-video.md#automatic-lyric-calibration) explains acceptance and failure behavior.

To calibrate a retained source project independently, choose an unused output path:

```sh
pnpm video:calibrate \
  --project work/spiral/project-source.json \
  --output work/spiral/project-calibrated.json
```

Use the resulting project for the editor and renderer. The [command declaration](../../tools/teaching-video/calibrate.py) owns supported model options. A calibration failure requires checking the recording and retained measurement evidence before rendering.

## Review the teaching content

The working Agent uses `teaching-input.json` and the source Ruby to write `work/spiral/teaching.json`. Every unique sung line receives a Chinese translation, contextual vocabulary information, and expression explanations. Repeated lines reuse the reviewed entry. Vocabulary cards identify the lyric surface, reading, lemma, part of speech, conjugation, Chinese meaning, and a supported JLPT level when available.

Read the complete teaching file against the original lyrics and their context. Confirm meanings and grammatical explanations as well as the source readings. The renderer validates the [teaching contract](../subsystems/teaching-video.md); language accuracy requires this content review.

## Edit the video visually

Open the local editor with the prepared inputs and an independent editing directory:

```sh
pnpm video:edit \
  --project work/spiral/project.json \
  --teaching work/spiral/teaching.json \
  --font /absolute/path/to/CJK-font.ttf \
  --workspace work/spiral-editor
```

Open `http://127.0.0.1:49603` in your browser. Select a lyric scene or seek through the recording. Double-click any visible text to edit it directly in the preview; Enter applies the change, Shift+Enter inserts a paragraph in ordinary text, and Escape cancels the edit. The properties panel also edits lyrics, Ruby, romaji, translations, vocabulary details, expression explanations, headings, song metadata, pagination, and the playback-time label. Lyric and reading replacements retain their source intervals; review their wording and pronunciation before exporting.

Select components in the preview or component list. Shift-click adds or removes a selection; dragging a rectangle from blank preview space selects enclosed components. Fully enclosed regions select their contents together. Original words select their related Ruby by default; disable the Ruby-link option to select them independently. Double-clicking a text field selects that field for direct editing. To move the current scene's three lyric rows together, select Original, Romaji, and Current translation with the Current scene range, then click Select components.

Choose All scenes and Romaji to select that row throughout the song. For a subset, check the desired scenes in the left panel and choose Marked scenes. Several component types can be selected together. The selection summary shows component and scene counts; the preview displays selected components in the current picture. Changing the viewed scene retains the selection.

Drag selected components or the Move selected handle, enter coordinates for one component or displacement values for multiple components, or use the arrow keys. Shift+arrow moves ten pixels. A selected parent and child receive the displacement once. Hide removes selected components from the rendered video while retaining their content and positions in the component list; Show makes them visible again. Delete removes them. Each batch operation can be undone together. Restore deleted restores deleted components, and Restore automatic position removes selected components' manual offsets.

Automatic layout is disabled by default, so other components retain their positions after editing, hiding, or deletion. Layout once arranges all visible components in the scene range selected above, clears that range's manual offsets in the current orientation, and retains the automatic-layout switch. Undo restores the previous arrangement and offsets. Enable Automatic layout when content and visibility changes should recalculate cards and pages continuously; disable it to retain the current arrangement. The layout controls describe their full scene scope independently of component-type selection.

Song metadata, background, time label, and progress-bar changes apply throughout the recording. Other components belong to their selected lyric occurrences. Text, visibility, and deletion changes apply to both video orientations; positions and arrangements are saved separately for landscape and portrait. The highlight color is extracted automatically from the song cover. Pure preview hides selection controls while preserving the rendered picture.

Valid edits save automatically to `edit.json` in the editing directory. Save project also writes that file explicitly. Reopening with the same inputs and editing directory restores the saved document. Export complete video creates a new export directory, captures the edit document, encodes the complete MP4, and runs full-song verification. Download links appear after the export passes; failures appear in the interface with the responsible error. Keep the editor process running while editing or exporting.

The command renderer can use the same saved composition by adding `--edits work/spiral-editor/edit.json` to the render command below. [Editor E2E](../../tools/teaching-video/editor-e2e.mjs) exercises real browser editing, saving, reloading, responsive previews, and complete exports; run it against an editor started with a new editing directory and retain its evidence directory.

[Selection E2E](../../tools/teaching-video/editor-selection-e2e.mjs) exercises multi-selection, cross-scene movement, hiding, deletion, one-shot layout, orientation independence, saving, reopening, and complete exports. Run `node tools/teaching-video/editor-selection-e2e.mjs --url http://127.0.0.1:49603 --evidence-dir work/editor-verification/selection-evidence` against a new editing directory. The output includes the saved edit document, actual preview images and geometry, an interaction report, and full-song verification reports.

## Render the complete video

```sh
pnpm video:render \
  --project work/spiral/project.json \
  --teaching work/spiral/teaching.json \
  --layout landscape \
  --font /absolute/path/to/CJK-font.ttf \
  --output work/spiral/spiral-landscape.mp4
```

Use `--layout portrait` and a separate output filename for the vertical composition. The renderer's [command declaration](../../tools/teaching-video/render.py) owns additional supported options. Export without a bounded time range and retain the default zero audio offset to include the complete recording.

## Verify and receive the output

```sh
pnpm video:verify \
  --manifest work/spiral/spiral-landscape.render.json \
  --require-full-song \
  --evidence-dir work/spiral/spiral-landscape-evidence
```

`--require-full-song` requires the export to start at the beginning, contain the complete song's frame count, and use a zero audio offset. Verification checks the calibration report and its three-region measurements, decodes the full MP4, checks video and audio metadata, compares every teaching page and selected calibrated highlight moments with the encoded frames, and compares the exported audio with the source recording. The evidence directory contains `verification.json` and the sampled frame files. Confirm `status: passed` and `song_scope: full`, and inspect the accepted offset and regional residuals in `lyric_alignment`. The Agent returns links to the complete video and evidence artifact after these checks succeed.
