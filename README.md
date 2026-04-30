# UtaBuild

跨平台歌词搜索与显示工具，复刻 utaten.com 的歌词搜索与振假名（Ruby）显示体验。  
同时也是一个 **LSPosed 模块**，可为 Salt Player 提供日语歌词振假名渲染支持。

---

## 功能特点

- **歌词搜索** — 从 utaten.com 等来源搜索日语歌词
- **Ruby/振假名渲染** — 在汉字的顶部标注读音假名
- **跨平台桌面应用** — Windows / Linux / macOS（Tauri v2）
- **Android APK** — 移动端歌词浏览
- **LSPosed 集成** — 为 Salt Player 注入日语歌词优化显示（可选）
- **CLI 工具** — 命令行歌词搜索与解析

---

## 环境要求

| 组件 | 版本 |
|------|------|
| Rust | 1.77+ |
| Tauri CLI v2 | `cargo install tauri-cli --version "^2"` |
| Node.js（可选，用于前端工具链） | 18+ |

### Android 额外要求

| 组件 | 说明 |
|------|------|
| **JDK** | 17+（推荐 Eclipse Temurin 或 Oracle JDK 17） |
| **Android Studio** | 2024.1+（用于 SDK/NDK 管理） |
| **Android SDK** | API 34+（通过 Android Studio SDK Manager 安装） |
| **Android NDK** | 27+（通过 SDK Manager 安装，用于 Rust → .so 交叉编译） |
| **Gradle** | 使用项目自带的 Gradle Wrapper（`gradlew`）|

> **Windows 用户注意**：Android 构建必须在 **Windows 原生环境**（而非 WSL）中执行。确保环境变量 `ANDROID_HOME` 或 `ANDROID_SDK_ROOT` 已正确设置。

---

## 安装

```bash
# 克隆项目
git clone <repo-url>
cd utabuild-tauri

# 安装 Rust（如果还没有）
rustup default stable

# 安装 Tauri CLI
cargo install tauri-cli --version "^2"

# 对有 Android 构建需求的用户：
# 通过 Android Studio > SDK Manager 安装 Android SDK 34+ 和 NDK 27+
```

---

## 开发

### 桌面端开发

```bash
cargo tauri dev
```

这将以开发模式启动桌面应用。前端位于 `src/` 目录（纯 HTML/CSS/JS），修改后刷新即可生效。

### Android 开发（连接真机或模拟器）

```bash
# 1. 首次构建前需初始化 Android 项目
cargo tauri android init

# 2. （如需要 LSPosed 集成）注入 LSPosed 模块代码
scripts/integrate-lsposed-into-tauri-android.sh

# 3. 在已连接的 Android 设备上运行开发版本
cargo tauri android dev
```

### CLI 工具开发

```bash
cd cli
cargo run -- --help        # 查看 CLI 帮助
cargo run -- search "歌名"  # 搜索歌词
```

---

## 构建

### Windows 桌面版

```bash
cargo tauri build
# 输出路径: src-tauri/target/release/bundle/
# 生成 .msi 或 .exe 安装包（取决于系统配置）
```

### macOS / Linux 桌面版

```bash
cargo tauri build
# macOS: src-tauri/target/release/bundle/dmg/
# Linux: src-tauri/target/release/bundle/deb/ 或 AppImage
```

### Android APK

```bash
# 1. 如果尚未初始化
cargo tauri android init

# 2. （可选）注入 LSPosed 模块代码（与 Salt Player 集成）
scripts/integrate-lsposed-into-tauri-android.sh

# 3. 构建发布版 APK
cargo tauri android build --target aarch64 --apk
# 输出路径: src-tauri/gen/android/app/build/outputs/apk/universal/release/
# 生成文件: app-universal-release-unsigned.apk
# 
# 注意：产出的 APK 是未签名的，如需安装需使用 Android Studio 签名，
#       或使用 `apksigner` / `jarsigner` 手动签名。
```

> **多架构构建**：如需同时构建多种 CPU 架构，省略 `--target` 参数：
> ```bash
> cargo tauri android build --apk
> ```
> 这将会构建 arm64-v8a、armeabi-v7a、x86、x86_64 四种架构的 APK。

### CLI 独立构建

CLI 库和可执行文件可以脱离 Tauri 独立构建：

```bash
# 构建 CLI 二进制
cargo build --manifest-path cli/Cargo.toml --release
# 产物: cli/target/release/utabuild-cli (或 .exe)

# 或者通过 Tauri 项目间接构建（会包含 CLI 库）
cargo build --manifest-path src-tauri/Cargo.toml --release
```

---

## LSPosed / Salt Player 集成

UtaBuild 的 Android APK 同时也是一个 **LSPosed API 101 模块**。在正常启动时是一个普通应用；如果在 LSPosed Manager 中启用并为 Salt Player 激活，会在 Salt Player 中注入歌词优化。

完整集成流程见 [docs/LSPOSED_INTEGRATION.md](docs/LSPOSED_INTEGRATION.md)。

### 快速启用

```bash
cargo tauri android init
scripts/integrate-lsposed-into-tauri-android.sh
cargo tauri android build --target aarch64 --apk
```

安装生成的 APK，然后在 **LSPosed Manager** 中启用 `UtaBuild` 模块，作用域设为 Salt Player（`com.salt.music`）。

---

## 测试

```bash
# CLI 库单元测试
cargo test --manifest-path cli/Cargo.toml

# Tauri 后端单元测试
cargo test --manifest-path src-tauri/Cargo.toml

# 前端 Ruby 渲染快速验证
# 在浏览器中直接打开 src/test-ruby.html
```

---

## 代码质量

```bash
# 格式化 Rust 代码
cargo fmt --manifest-path src-tauri/Cargo.toml --all

# Lint 检查（在提交前运行）
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

---

## 项目结构

```
utabuild-tauri/
├── src/                    # 前端（HTML / CSS / JS）
│   ├── js/app.js           # 主应用逻辑
│   ├── css/style.css       # 样式
│   └── test-ruby.html      # Ruby 渲染测试页
├── src-tauri/              # Tauri v2 Rust 后端
│   ├── src/                # Rust 源码（IPC 命令、App 入口）
│   ├── android-lsposed/    # LSPosed 模块覆盖层（Java）
│   ├── gen/android/        # 自动生成的 Android 项目（不提交）
│   └── tauri.conf.json     # Tauri 配置
├── cli/                    # 可复用的 Rust 歌词库 + CLI 工具
│   ├── src/                # 搜索、解析、缓存、历史
│   └── tests/              # 单元测试（主要测试集中在此）
├── lsposed-module/         # LSPosed 模块开发脚手架（仅用于快速编译检查）
├── scripts/                # 辅助脚本
│   ├── integrate-lsposed-into-tauri-android.sh  # LSPosed 注入脚本
│   └── sync-to-windows.sh  # Windows 镜像同步脚本
└── docs/
    └── LSPOSED_INTEGRATION.md   # LSPosed 集成文档
```

---

## 许可证

本项目基于 [LICENSE](LICENSE) 文件中指定的许可证发布。
