# UtaBuild 用户指南

[English](index.md) | 中文

UtaBuild 可以搜索歌词、显示日语 Ruby 读音，并保存已选择的歌词供后续使用。桌面端和 Android 应用共享同一套 Rust 歌词流水线。

## 搜索与显示

输入标题和可选歌手；默认结果不合适时选择数据源，然后选择一条结果。歌词视图会渲染纯文本、Ruby 读音、换行、专辑元数据，以及数据源提供的封面。

## 已保存歌词

歌曲视图列出本地保存的歌词条目，可以重新加载、刷新封面元数据、导出 HTML 或删除条目。缓存控制位于设置页面。

## Salt Player 集成

Android LSPosed 集成可以接收来自 Salt Player 的歌曲启动请求，通过 UtaBuild bridge 获取结构化歌词，并将 Ruby 读音注入 Salt Player 的渲染路径。只有在设备已经安装 bridge 时，才启用模块和相关设置。

## CLI

CLI 支持搜索、直接获取 URL 歌词、历史记录管理、JSON 输出和 HTML 输出：

```sh
cargo run --manifest-path cli/Cargo.toml -- search --title "曲名" --artist "歌手"
cargo run --manifest-path cli/Cargo.toml -- history list
```
