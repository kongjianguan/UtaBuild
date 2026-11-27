# Agent Note: Restore UtaTen search with a same-origin Referer

Status: implemented

[English](2026-11-28-utaten-search-referer.md) | 中文

## Problem

UtaTen 对缺少 `Referer` 请求头的搜索请求返回 HTTP 200 和空正文。CLI 把这个空页面解析成零条搜索结果，因此每次 UtaTen 搜索都没有匹配项，同时不报告错误。

## Decision

[`searcher.rs`](../../../../cli/src/searcher.rs) 中的 UtaTen HTTP 客户端默认发送 `Referer: https://utaten.com/`。该请求头同样应用于 UtaTen 歌词页面请求，与浏览器在同站导航时发送的来源一致。

搜索响应解码把空正文视为请求错误，错误消息为 `搜索请求返回空响应`。空响应不会写入缓存，非空搜索结果继续使用现有缓存路径。

## Alternatives considered

**只在搜索请求上发送该请求头。** 共享客户端还会获取 UtaTen 歌词页面，同站来源在这些页面同样有效。默认请求头避免出现第二套请求路径和不同的浏览器行为。

**当 `title` 参数返回空正文时回退到 `artist_name`。** 这种回退无法恢复仅按标题的搜索，并且会掩盖上游请求契约。

**继续把空正文解析为 `not_found`。** 这会保留导致本次回归难以定位的静默失败。

## Consequences

UtaTen 标题搜索、艺术家搜索、分页和歌词获取在使用 HTTP 代理与不使用代理时都能正常工作。上游未来如果再次返回空页面，会以 `status: error` 呈现，并且不会进入搜索缓存。

## Testing

CLI 全 target 测试通过，包含 102 个单元测试和 8 个歌词流水线测试。真实搜索中，`紅蓮華` 配合艺术家 `LiSA` 返回 1 条 UtaTen 结果，`夜に駆ける` 返回 8 条，`ヨルシカ` 第 2 页返回 40 条。选择第一条 `紅蓮華` 结果后获取到完整的带注音歌词。不带 `Referer` 的原始请求返回 200 和 0 字节；同一请求携带 `Referer: https://utaten.com/` 后返回完整的搜索结果页面。
