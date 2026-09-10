# Agent Note: Match NetEase historical kana spellings during Ruby alignment

Status: implemented

[English](2026-09-02-netease-historical-kana-ruby-boundary.md) | 中文

## Problem

NetEase `romalrc` 可能把原文中的 `づ` 或 `ぢ` 按发音写成 `ず` 或 `じ`。此前 Ruby 对齐器在搜索送假名边界时把这些异写形式视为不同字符。边界搜索失败后会回退为消耗当前行剩余的大部分读音，因此 `気づけなかったんだ` 可能把 `きずけなかったん` 整段挂到 `気` 上。

## Decision

[`ruby_align.rs`](../../../../cli/src/ruby_align.rs) 中的假名匹配器在比较时将 `ぢ` 归一化为 `じ`，将 `づ` 归一化为 `ず`，并保留已有的助词和长音等价规则。锚点搜索、回退边界检测和假名消耗都使用这个匹配器，因此整个对齐算法使用同一规则。

生成的 Ruby 仍然来自 NetEase `romalrc` 转换出的发音；本修复只改变边界识别。因此 `気づけ` 会生成 Ruby `きず`，与项目保留 NetEase 发音拼写、而不是改写为原文正字法的现有约定一致。

## Alternatives considered

**在 Ruby 输出中把 `ず` 和 `じ` 改回 `づ` 和 `ぢ`。** 这会把原文正字法混入本来由 `romalrc` 生成的读音字符串，并且需要比当前边界缺陷更广泛的源文本重写。

**将 NetEase 切换到 tokenized LRC/YRC 路径。** 该路径按汉字块分发 token，无法可靠处理这个输入形态中的送假名。切换共享的 NetEase 路径会扩大行为变化，却没有直接修复字符匹配不变量。

## Consequences

原文和罗马音使用这些历史假名异写形式的歌词不再触发过度消耗回退。现有歌词和公开的 `LyricElement` 结构保持不变，只有受影响的 Ruby 边界会改为挂到正确的原文上。

已经持久化的 Ruby 或 Salt bridge 缓存不会由这次 parser 修改自动重写。受影响的歌曲需要重新获取歌词或清理缓存，旧的错误条目才会被替换。

## Testing

[`lrc_parser.rs`](../../../../cli/src/lrc_parser.rs) 包含使用 `spiral` 对应歌词行和 NetEase romaji 的回归测试。CLI 全 target 测试、完整 QRC 流水线 fixture 测试以及 `src-tauri` 编译均通过。
