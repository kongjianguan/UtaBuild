# UtaBuild

跨平台歌词搜索与显示工具，复刻 utaten.com 的歌词搜索与振假名（Ruby）显示体验。  

---

## 功能特点

- **歌词搜索** — 从 utaten.com QQ 网易云等来源搜索歌词
- **Ruby/振假名渲染** — 在汉字的顶部标注读音假名
- **跨平台桌面应用** — Windows / Linux / macOS（Tauri v2）
- **Android APK** — 移动端歌词浏览
- **CLI 工具** — 命令行歌词搜索与解析

---

## 目标功能

- **LSPosed 集成** — 为 Salt Player 提供日语歌词优化显示（尚未实现）

---

## 环境要求

| 组件 | 版本 |
|------|------|
| Rust | 1.77+ |
| Tauri CLI v2 | `cargo install tauri-cli --version "^2"` |
| Node.js | 18+ |

### Android 构建要求

| 组件 | 说明 |
|------|------|
| **JDK** | 17+ |
| **Android Studio** | 2024.1+ |
| **Android SDK** | API 34+ |
| **Android NDK** | 27+ |
| **Gradle** | |

---

## 安装

```bash
# 克隆项目
git clone https://github.com/kongjianguan/UtaBuild.git
cd UtaBuild

# 安装 Rust（如果还没有）
rustup default stable

# 安装 Tauri CLI
cargo install tauri-cli --version "^2"

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

# 2. 在已连接的 Android 设备上运行开发版本
cargo tauri android dev
```

### 单独使用 CLI

> UtaBuild 有 CLI 版本，位于`/cli`

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
# 生成 .msi 或 .exe
```

### macOS / Linux 桌面版

```bash
cargo tauri build
# macOS: src-tauri/target/release/bundle/dmg/
# Linux: src-tauri/target/release/bundle/deb/ 或 AppImage
```

### 镜像配置

#### Cargo 镜像

在 `~/.cargo/config.toml`中添加：

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

（阿里云镜像）：
```properties
distributionUrl=https\://mirrors.aliyun.com/gradle/distributions/gradle-8.14.3-bin.zip
```

### Android APK

```bash
# 1. 如果尚未初始化
cargo tauri android init
# ↓ 中国大陆用户：按上方说明配置 Cargo/Gradle 镜像后再继续 ↓

# 2. 构建发布版 APK
cargo tauri android build --target aarch64 --apk
# 输出路径: src-tauri/gen/android/app/build/outputs/apk/universal/release/
# 生成文件: app-universal-release-unsigned.apk
```

> **多架构构建**：如需同时构建多种 CPU 架构，省略 `--target` 参数：
> ```bash
> cargo tauri android build --apk
> ```
> 这将会构建 arm64-v8a、armeabi-v7a、x86、x86_64 四种架构的 APK。


## 测试生成

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
├── src/                    # 前端
├── src-tauri/              # Tauri v2 Rust 后端
│   ├── src/                # Rust 源码（IPC 命令、App 入口）
│   └── tauri.conf.json     # Tauri 配置
├── cli/                    # 可复用的 Rust 歌词库 + CLI 工具
│   ├── src/                # 搜索、解析、缓存、历史
└── scripts/                # 辅助脚本
```

---

## 许可证

本项目基于 GPLv3 许可证发布。
