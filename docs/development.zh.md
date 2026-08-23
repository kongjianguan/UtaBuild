# UtaBuild 开发

[English](development.md) | 中文

UtaBuild 使用 Rust 编写 CLI 和 Tauri 后端，使用 TypeScript 与 SCSS 编写前端，使用 Java 编写 Android LSPosed bridge。除非命令明确进入子项目，否则从仓库根目录执行工作。

## 环境要求

安装 Rust 1.70 或更高版本、Node.js 18 或更高版本、Tauri CLI v2 以及前端依赖。Android 开发还需要 JDK 17+、Android Studio 2024.1+、Android SDK API 34+、Android NDK 27+ 和生成的 Gradle wrapper。

仓库包含 `pnpm-lock.yaml`；前端依赖和脚本统一使用 pnpm。

## 日常命令

```sh
pnpm install
pnpm run build:ts
pnpm run build:scss
pnpm run lint
cargo test --manifest-path cli/Cargo.toml
```

在 `cargo tauri dev` 或 `cargo tauri build` 前运行 `pnpm run build`；Tauri 配置也会通过 `beforeDevCommand` 和 `beforeBuildCommand` 调用前端构建。

## 桌面端和 Android

```sh
cargo tauri dev
cargo tauri build
cargo tauri android init
cargo tauri android dev
cargo tauri android build --target aarch64 --apk
```

Android 初始化会创建或更新 `src-tauri/gen/android/`。除非平台变更要求提交，否则将生成的 Android 输出视为构建产物。

## CLI fixture

`cli/tests/lyrics_pipeline_test.rs` 中的 CLI 集成测试读取 QRC fixture，并将完整的 `LyricsOutput` JSON 与 expected 文件对比。明确修改 fixture 后，将 expected JSON 重新生成到该 fixture 现有的 expected 文件路径，再人工审查完整输出：

```sh
cargo run --manifest-path cli/Cargo.toml --release -- search \
  --title "<title>" --artist "<artist>" \
  --output <fixture-expected-json-path>
```

审查 fixture 后重新运行聚焦测试：

```sh
cargo test --manifest-path cli/Cargo.toml --test lyrics_pipeline_test
```

## 变更路由

共享歌词行为先在 `cli/` 中修改。只有应用边界需要新的编排或序列化时，才修改 Tauri 命令。JSON 契约变化时更新前端类型和视图。Salt Player 传输或渲染契约变化时更新 Android bridge。

每个非平凡的架构、行为、测试或流程变更都要新增或更新 Agent Note。报告完成前运行文档校验器和受影响范围内最小的代码检查。
