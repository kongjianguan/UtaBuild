# Agent Note: Restore UtaTen search with a same-origin Referer

Status: implemented

English | [中文](2026-11-28-utaten-search-referer.zh.md)

## Problem

UtaTen returns HTTP 200 with an empty response body for search requests that lack a `Referer` header. The CLI parsed that empty page as zero search results, so every UtaTen search produced no matches without reporting an error.

## Decision

The UtaTen HTTP client in [`searcher.rs`](../../../../cli/src/searcher.rs) sends `Referer: https://utaten.com/` as a default header. The header also applies to UtaTen lyric page requests, matching the referer a browser sends during same-site navigation.

Search response decoding treats an empty body as a request error with the message `搜索请求返回空响应`. Empty responses are not cached, and non-empty search results continue through the existing cache path.

## Alternatives considered

**Send the header only on search requests.** The shared client also retrieves UtaTen lyric pages, and a same-origin referer is valid there. A default header avoids a second request path with different browser-like behavior.

**Fall back to `artist_name` when the `title` parameter returns an empty body.** This cannot recover title-only searches and hides the upstream request contract.

**Keep parsing empty bodies as `not_found`.** This preserves the silent failure that made the regression difficult to identify.

## Consequences

UtaTen title searches, artist searches, pagination, and lyric retrieval work with and without an HTTP proxy. A future upstream response change that returns an empty page now surfaces as `status: error` and remains outside the search cache.

## Testing

The CLI all-target suite passes with 102 unit tests and 8 lyrics pipeline tests. Live searches returned 1 UtaTen result for `紅蓮華` with artist `LiSA`, 8 results for `夜に駆ける`, and 40 results for `ヨルシカ` page 2. Selecting the first `紅蓮華` result returned complete ruby lyrics. A raw request without `Referer` returned 200 with 0 bytes, while the same request with `Referer: https://utaten.com/` returned a populated search page.
