# UtaBuild 子系统

[English](README.md) | 中文

本目录负责稳定的子系统契约。先阅读 [architecture.md](../architecture.zh.md) 了解组合关系和依赖方向，再使用下面的页面查找术语、边界和聚焦验证方式。

## 子系统地图

| 子系统 | 负责内容 | 主要源码 |
|---|---|---|
| [CLI 与歌词流水线](cli.zh.md) | 搜索源、歌词解析、Ruby 对齐、缓存、历史记录以及 JSON/HTML 输出 | `cli/src/` |
| [前端](frontend.zh.md) | TypeScript 视图、状态、Tauri 调用、Ruby DOM 渲染、设置和导出控件 | `src/ts/`、`src/index.html` |
| [Tauri 后端](tauri-backend.zh.md) | 桌面/移动端命令、共享 `AppState`、缓存编排、持久化、日志和 HTML 导出 | `src-tauri/src/lib.rs` |
| [Android LSPosed bridge](android-lsposed.zh.md) | Salt Player hook、结构化歌词传输、Ruby 对齐和 Canvas 渲染 | `src-tauri/android-lsposed/` |

## 归属规则

类型、命令 payload、数据源名称、缓存键和输出字段都属于源码事实。此目录只描述契约，并在可能发生漂移的细节处链接到声明位置。不要在这里建立第二份完整符号目录。

## 验证

歌词流水线运行 `cargo test --manifest-path cli/Cargo.toml`，前端运行 `pnpm run build:ts`。Android 和 Tauri 变更在环境提供对应平台工具链时，运行平台专属构建或测试命令。
