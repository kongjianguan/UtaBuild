# Tauri 后端

[English](tauri-backend.md) | 中文

`src-tauri/src/lib.rs` 中的 Tauri 后端是共享 `utabuild-cli` crate 外侧的应用边界，负责命令注册、应用状态、平台持久化、bridge 文件和响应序列化。

## 状态与命令

`AppState` 包含受 mutex 保护的 `UtaTenSearcher` 和 LSP 日志开关。注册的命令分为歌词搜索与获取、已保存歌词管理、缓存管理、Salt 启动/绑定、LSP 设置与日志以及 HTML 导出。

`search_lyrics` 检查搜索缓存，并路由 UtaTen、QQ Music 或 NetEase 查询。`get_lyrics` 处理数据源专属歌词获取和缓存键，然后返回 `ruby_annotations`。`search_and_get` 为前端的一步路径组合搜索和第一条结果选择。

## 持久化

后端通过 CLI 缓存模块持久化搜索响应和带注音歌词。已保存歌词摘要包含标题、歌手、专辑、封面 URL、源 URL、时间戳和注音数量。删除操作同时失效磁盘和内存中的歌词状态。

LSP 设置和日志写入应用数据目录。Salt 启动请求和结构化歌词 bridge 文件使用经过清理的标题路径，使 Android provider 可以读取它们，而不需要直接调用 Rust 命令。

## 响应边界

由于前端动态调用命令，Tauri 命令返回 `serde_json::Value` 或简单布尔值/字符串。修改 `ruby_annotations`、`lines`、元数据或数据源标识符时，保持 `src/ts/types.ts` 与 Android `StructuredLyrics.fromUtaBuildJson` 解析器中的字段同步。

## 验证

本地 Tauri 工具链可用时，运行 `cargo check --manifest-path src-tauri/Cargo.toml`。共享行为运行 CLI 测试，命令 payload 变化时运行 `pnpm run build:ts`。Android bridge 变更需要运行生成的 Android 构建，或在可用时运行聚焦的 Java 测试。
