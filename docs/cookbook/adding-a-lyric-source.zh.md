# 添加歌词数据源

[English](adding-a-lyric-source.md) | 中文

添加新的歌词或封面提供方时使用本流程。数据源必须保持共享的 `LyricElement` 和响应契约。

## 1. 定义数据源契约

在 `cli/src/searcher.rs` 中增加数据源偏好及其可接受的设置值。定义 URL 和缓存键使用的数据源标识符。记录该数据源提供纯歌词、带时间歌词、Ruby 注音、封面还是元数据。

## 2. 实现获取

网络访问保持在 CLI crate 中。在现有提供方代码附近增加请求、解码、解析和错误处理。遵守数据源限速，不要把 HTTP 错误缓存成空结果。

## 3. 接入应用

如果数据源需要命令级路由、缓存行为或响应规范化，更新 `src-tauri/src/lib.rs`。如果 JSON 契约或用户选择发生变化，更新 `src/ts/types.ts`、设置、数据源控件和结果处理。

## 4. 测试流水线

增加聚焦的解析或对齐测试；数据源有稳定样本时增加 fixture。运行：

```sh
cargo test --manifest-path cli/Cargo.toml
pnpm run build:ts
```

## 5. 更新文档

数据源改变边界或流程时，更新所属子系统页面、用户指南和 `docs/architecture.md`。数据源引入持久决策、缓存规则或兼容性义务时，新增或更新 implemented Agent Note。
