# Agent Note: Restore QQ Music search with the current song search type

Status: implemented

[English](2026-11-28-qq-music-search-type.md) | 中文

## Problem

QQ Music 的 `DoSearchForQQMusicLite` 接口改变了 `search_type: 0` 的含义。所有查询都会返回顶层 `code: 0`、`req_0.code: 2001` 和空的 `item_song` 数组。CLI 把空数组当作 `not_found`，因此 QQ Music 返回零条结果，同时不报告上游错误。封面查询和 QRC 歌曲查询也使用同一套请求结构。

## Decision

[`searcher.rs`](../../../../cli/src/searcher.rs) 中的常量 `QQ_MUSIC_SONG_SEARCH_TYPE` 取值为 `7`，这个值返回单曲。三个 QQ Music Lite 搜索请求都使用该值：封面查询、QRC 歌曲查询和面向用户的搜索。

`search_qq_music` 在读取 `item_song` 之前校验 `req_0.code`。非零请求码会返回 `status: error` 和上游请求码，因此未来的枚举变化不会再表现为空结果集。

## Alternatives considered

**切换到旧版 `client_search_cp` 接口。** 该接口仍然返回歌曲，但响应使用 `songname`、`songmid` 和 `albumname`，需要第二套解析器和不同的分页契约。Lite 接口在使用当前单曲类型后仍然可用。

**把空的 `item_song` 数组当作 `not_found`。** 这会掩盖上游契约变化，也是本次静默失败的来源。

**使用 `search_type` 的 1、2 或 3。** 实际响应分别填充歌手、专辑或歌单数组，`item_song` 保持为空。

## Consequences

QQ Music 恢复每页八首歌曲，并且分页结果可用。封面查询和 QRC 歌词查询使用修正后的单曲类型。非零 `req_0.code` 现在会作为错误呈现，不会变成空结果列表。

## Testing

CLI 全 target 测试通过，包含 102 个单元测试和 8 个歌词流水线测试。真实搜索中，`紅蓮華` 配合艺术家 `LiSA` 返回 8 条 QQ Music 结果，艺术家 `LiSA` 返回 8 条，`晴天` 配合艺术家 `周杰伦` 返回 8 条。选择 QQ Music 的 `紅蓮華` 结果后返回 24 行歌词和 94 个元素。第 1、2、3 页使用 `search_type: 7` 时都返回 8 首歌曲；`search_type: 0` 返回 `req_0.code: 2001`。
