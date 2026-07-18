# UtaBuild

跨平台歌词搜索与显示工具，复刻 utaten.com 的歌词搜索与振假名（Ruby）显示体验。  
同时也是一个 **LSPosed 模块**，可为 Salt Player 提供日语歌词振假名渲染支持。

---

## 功能特点

- **歌词搜索** — 从 utaten.com QQ 网易云等来源搜索歌词
- **Ruby/振假名渲染** — 在汉字的顶部标注读音假名
- **跨平台桌面应用** — Windows / Linux / macOS（Tauri v2）
- **Android APK** — 移动端歌词浏览
- **LSPosed 集成** — 为 Salt Player 注入日语歌词优化显示（暂未完成实现）
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

### TypeScript 编译

前端 TypeScript 源码位于 `src/ts/`，编译产物输出到 `src/js/`。

> **注意：** 仓库中已包含预构建的 `.js` 文件，仅修改 TypeScript 源码后才需要手动编译。

修改 TypeScript 源码后，需手动编译生成 `.js` 文件：

```bash
# npm
npx tsc

# bun
bunx tsc

# pnpm
pnpm exec tsc
```

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

### Android 镜像配置（中国大陆用户必读）

由于网络原因，中国大陆用户需要配置 Gradle 和 Cargo 镜像以加速下载。

#### Cargo 镜像

在 `%USERPROFILE%\.cargo\config.toml`（Windows）或 `~/.cargo/config.toml`（Linux/macOS）中添加：

```toml
[source.crates-io]
replace-with = 'rsproxy-sparse'

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[net]
git-fetch-with-cli = true
```

#### Gradle Wrapper 镜像

`cargo tauri android init` 生成 Android 项目后，修改 `src-tauri/gen/android/gradle/wrapper/gradle-wrapper.properties`：

```properties
distributionUrl=https\://mirrors.cloud.tencent.com/gradle/gradle-8.14.3-bin.zip
```

备用镜像（阿里云）：
```properties
distributionUrl=https\://mirrors.aliyun.com/gradle/distributions/gradle-8.14.3-bin.zip
```

### Android APK

```bash
# 1. 如果尚未初始化
cargo tauri android init
# ↓ 中国大陆用户：按上方说明配置 Cargo/Gradle 镜像后再继续 ↓

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

### GitHub Actions Android 构建

`.github/workflows/release.yml` 中的 `build-android` job 会在桌面构建完成后，使用
Java 17、Android SDK 36、NDK 27.2 和 `aarch64` Rust target 构建 Android APK，并将未签名的
`arm64-v8a` APK 上传到同一个草稿 Release。工作流可通过推送 `v*` 标签或手动触发；如需发布可安装的 APK，
还需要在工作流中接入 Android keystore 签名。

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

## 测试

```bash
# 全部测试
cargo test --manifest-path cli/Cargo.toml

# 仅集成测试（真实 QRC 数据比对）
cargo test --manifest-path cli/Cargo.toml --test lyrics_pipeline_test

# Tauri 后端测试
cargo test --manifest-path src-tauri/Cargo.toml
```

fixture 变更时重新生成参照 JSON：
```bash
cargo run --manifest-path cli/Cargo.toml --release -- search \
  --title <歌名> --artist <歌手> \
  --output cli/tests/fixtures/{slug}/expected.json
```

*需要人工校验参照JSON的内容*

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

本项目基于 GPLv3 许可证发布。
