# UtaBuild Repository Instructions

## Documentation library

- 任何新增、修改、移动、翻译、归档、审计或生成文档的工作，都必须遵守 `project-doc-library` skill。
- 同时遵守最近的 `AGENTS.md`。文档归属和写作规则见 [`docs/AGENTS.md`](docs/AGENTS.md)，Agent Note 生命周期见 [`.agents/notes/AGENTS.md`](.agents/notes/AGENTS.md)，双语配对规则见 [`docs/i18n/README.md`](docs/i18n/README.md)。
- 文档变更必须维护正确的 owning document、双语 counterpart 和 `.i18n.yaml` sidecar；完成后运行 project-doc-library verifier、prose lint 和 `git diff --check`。sidecar 只能在两种语言都完成语义复核后刷新。

## Core lyric pipeline test

- UtaBuild 的核心测试是 [`cli/tests/lyrics_pipeline_test.rs`](cli/tests/lyrics_pipeline_test.rs)。它将同一首歌的 QRC fixture 经过歌词流水线后的完整 JSON，与对应的 expected JSON 做结构比较。
- 影响歌词搜索、解析、Ruby 对齐、输出结构或相关缓存行为的代码变更，必须在变更前后针对同一首或受影响的歌曲运行该测试，并检查完整 JSON 的实际效果。输出应保持一致，或经过人工确认后变得更正确、更完整；不能只以“测试通过”为依据。
- `cli/tests/fixtures/` 中每首歌曲的 expected JSON 是核心行为的完整测试基准，必须保存正确或最优的完整 JSON。禁止删除、截断、简略字段或元素，禁止用最小 JSON 让测试通过。
- 不得删除现有歌曲的 fixture。新增歌曲时，补齐原始 QRC、Ruby/romaji QRC、完整 expected JSON，并在 [`cli/tests/lyrics_pipeline_test.rs`](cli/tests/lyrics_pipeline_test.rs) 注册对应测试。
- 只有在确认代码变更确实改善或修正输出后，才可以更新 expected JSON；更新后必须人工检查完整内容，并重新运行核心测试。
