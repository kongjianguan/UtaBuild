# UtaBuild

跨平台歌词搜索与显示工具，复刻 utaten.com 的歌词搜索与振假名（Ruby）显示体验。

## 环境要求 Enviroment requirement

- Rust 1.77+
- Tauri v2 CLI: `cargo install tauri-cli --version "^2"`

## 安装 Install

```bash
# 克隆项目 clone project
git clone <repo-url>
cd utabuild-tauri

# 安装Rust (如果还没有)
rustup default stable

# 安装Tauri CLI
cargo install tauri-cli --version "^2"
```

## 开发 Debug

```bash
cargo tauri dev
```

## 构建 Build

```bash
# Windows
cargo tauri build
# 输出: src-tauri/target/release/bundle/

# Android (need Android Studio)
# 1. git pull 
# 2. Open with Android Studio
# 3. Build > Build Bundle(s) / APK(s)
```
