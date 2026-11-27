# Agent Note: Restore QQ Music search with the current song search type

Status: implemented

English | [中文](2026-11-28-qq-music-search-type.zh.md)

## Problem

QQ Music's `DoSearchForQQMusicLite` endpoint changed the meaning of `search_type: 0`. It returns top-level `code: 0` together with `req_0.code: 2001` and an empty `item_song` array for every query. The CLI treated the empty array as `not_found`, so QQ Music produced zero results without reporting an upstream failure. The same request shape also fed artwork lookup and QRC song lookup.

## Decision

The constant `QQ_MUSIC_SONG_SEARCH_TYPE` in [`searcher.rs`](../../../../cli/src/searcher.rs) is `7`, which returns individual songs. All three QQ Music Lite search requests use this value: artwork lookup, QRC song lookup, and the user-facing search.

`search_qq_music` validates `req_0.code` before reading `item_song`. A non-zero request code returns `status: error` with the upstream code, so a future enum change cannot appear as an empty result set.

## Alternatives considered

**Switch to the legacy `client_search_cp` endpoint.** The endpoint still returns songs, but its response uses `songname`, `songmid`, and `albumname`, which would require a second parser and a different pagination contract. The Lite endpoint remains functional with the current song type.

**Treat the empty `item_song` array as `not_found`.** This hides upstream contract changes and was the source of the silent failure.

**Use `search_type` values 1, 2, or 3.** Live responses populate singers, albums, or playlists respectively, leaving `item_song` empty.

## Consequences

QQ Music returns eight songs per page again, including paginated results. Artwork lookup and QRC lyrics lookup use the corrected song type. A non-zero `req_0.code` now appears as an error instead of an empty result list.

## Testing

The CLI all-target suite passes with 102 unit tests and 8 lyrics pipeline tests. Live searches returned 8 QQ Music results for `紅蓮華` with artist `LiSA`, 8 for artist `LiSA`, and 8 for `晴天` with artist `周杰伦`. Selecting the QQ Music `紅蓮華` result returned 24 lyric lines with 94 elements. Pages 1, 2, and 3 each returned 8 songs with `search_type: 7`; `search_type: 0` returned `req_0.code: 2001`.
