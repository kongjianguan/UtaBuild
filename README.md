# UtaBuild

跨平台歌词搜索与显示工具，提供日语 Ruby（振假名）显示和歌词保存功能。桌面端、Android 应用与 CLI 共用 Rust 歌词流水线。

## 功能

- 从 UtaTen、QQ Music 和 NetEase Cloud Music 搜索或获取歌词
- 渲染 Ruby 注音、换行、歌曲元数据和封面
- 提供 Windows、Linux、macOS 桌面应用和 Android 应用
- 提供 CLI，支持搜索、URL 获取、历史记录、JSON 和 HTML 输出
- 通过 Android LSPosed 集成为 Salt Player 注入 Ruby 歌词显示

## 快速开始

### 桌面端

环境要求：Rust 1.70+、Node.js 18+、Tauri CLI v2。安装前端依赖后运行：

```sh
pnpm install
cargo install tauri-cli --version "^2"
cargo tauri dev
```

完整的环境要求、构建命令和 Android 流程见[开发指南](docs/development.zh.md)。

### Android

```sh
cargo tauri android init
cargo tauri android dev
```

### CLI

```sh
cargo run --manifest-path cli/Cargo.toml -- search --title "曲名" --artist "歌手"
cargo run --manifest-path cli/Cargo.toml -- history list
```

更多 CLI 用法见[用户指南](docs/user/index.zh.md)。

## 核心测试

歌词流水线的核心测试会将 QRC fixture 的完整输出 JSON 与 expected JSON 比较：

```sh
cargo test --manifest-path cli/Cargo.toml --test lyrics_pipeline_test
```

fixture 的生成和维护要求见[开发指南](docs/development.zh.md#cli-fixture)。

## 文档

- [用户指南](docs/user/index.zh.md)
- [开发指南](docs/development.zh.md)
- [架构](docs/architecture.zh.md)
- [子系统说明](docs/subsystems/README.zh.md)

## 许可证

[GPLv3](LICENSE)
