# UtaBuild

跨平台歌词搜索与显示工具，复刻 utaten.com 的歌词搜索与振假名（Ruby）显示体验。

## 支持平台

- Windows (x86_64)
- Android (aarch64)

## 环境要求

- Rust 1.77+
- Tauri v2 CLI: `cargo install tauri-cli --version "^2"`

## 安装

```bash
# 克隆项目
git clone <repo-url>
cd utabuild-tauri

# 安装Rust (如果还没有)
rustup default stable

# 安装Tauri CLI
cargo install tauri-cli --version "^2"
```

## 开发

```bash
cargo tauri dev
```

## 构建

```bash
# Windows
cargo tauri build
# 输出: src-tauri/target/release/bundle/

# Android (需要Android Studio)
# 1. git pull 在Windows端
# 2. 用Android Studio打开项目
# 3. Build > Build Bundle(s) / APK(s)
```

## 文档

- [架构详解](./docs/ARCHITECTURE.md)
- [Ruby渲染方案](./docs/RUBY_RENDERING.md)
