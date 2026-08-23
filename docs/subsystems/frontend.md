# Frontend

English | [中文](frontend.zh.md)

The frontend is a static TypeScript, HTML, and SCSS application under `src/`. Tauri serves the built `src/` directory as the frontend distribution; browser work can use the mock invocation path in `src/ts/tauri.ts`.

## Responsibilities

`app.ts` initializes controls and coordinates the first-level views: search, saved songs, and settings. `dom.ts` owns view routing, DOM access, loading states, toasts, scroll restoration, and shared control state. `search.ts` owns search submission, source tabs, pagination, result selection, and lyric retrieval. `songs.ts` owns saved-lyric browsing and metadata refresh.

`ruby.ts` converts `LyricElement[]` into DOM nodes. It keeps Ruby annotations in `<ruby>`/`<rt>` elements and emits plain text for non-Ruby elements. `settings.ts`, `cache.ts`, `lsp.ts`, and `export.ts` own their respective settings, cache, bridge-log, and export interactions.

## Backend boundary

Use `invoke` through `src/ts/tauri.ts`. The frontend passes serializable options such as `title`, `artist`, `page`, `useCache`, `lyricSource`, and `artworkSource`, then consumes the typed response shapes in `types.ts`. Do not fetch lyric providers directly from the browser layer.

## State and views

The router distinguishes first-level search, saved-song, and settings views from nested results, lyrics, About, LSP settings, and LSP log views. First-level saved-song and settings views have no back action; nested views expose a top-left back action. View transitions save and restore scroll position.

The frontend uses a Miuix-inspired visual system implemented in SCSS rather than importing a platform component library. Shared tokens define rounded surfaces, grouped preference rows, 40px controls, 48px search fields, primary and secondary button roles, and press feedback. Small windows use a floating bottom `NavigationBar`; windows at least 960px wide use a left `NavigationRail`. Search, results, saved songs, settings, logs, dialogs, and lyrics share the same surface and control tokens, while MyGO keeps its palette and starfield.

The document and body are locked against page scrolling. Only elements explicitly marked with `data-view-scroll` or the LSP log's horizontal-and-vertical log surface may scroll. The search form collapses to a title field plus action on narrow screens and expands to title, artist, and action fields when focused. The viewport guard disables pinch zoom and modifier-based browser zoom gestures.

Search state keeps per-source responses and a merged result list; settings persist locally and synchronize LSP settings with the backend when Tauri is available.

## Verification

Run `pnpm run build:ts`, `pnpm run lint`, and `pnpm run build:scss` for frontend changes. Use the static browser path at `http://127.0.0.1:4173/` to check narrow and wide layouts, first-level versus nested navigation, internal scrolling, search expansion, and theme rendering; it does not prove Tauri command behavior.
