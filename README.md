# UtaBuild - 跨平台歌词搜索与显示工具

复刻 utaten.com 的歌词搜索与振假名（Ruby）显示体验。

## 架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        UtaBuild                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌─────────────┐    IPC (tauri::invoke)    ┌──────────────┐    │
│   │   前端       │ ←───────────────────────→ │   后端       │    │
│   │ HTML/CSS/JS │    command handlers       │   Rust       │    │
│   │ 响应式布局   │                           │   Tauri v2   │    │
│   │ Ruby渲染    │                           │              │    │
│   └─────────────┘                           └──────┬───────┘    │
│                                                    │             │
│                                          ┌─────────▼─────────┐  │
│                                          │     CLI Library    │  │
│                                          │   (utabuild-cli)   │  │
│                                          │   - 搜索utaten     │  │
│                                          │   - 解析ruby歌词   │  │
│                                          │   - 缓存管理       │  │
│                                          └───────────────────┘  │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  跨平台目标:                                                     │
│  • Windows (x86_64)                                              │
│  • Android (aarch64)                                             │
│  未来: iOS, macOS, Linux                                         │
└─────────────────────────────────────────────────────────────────┘
```

## 三层架构

### 1. CLI Library (`cli/`)
无运行时状态的纯命令行工具，负责：
- 搜索 utaten.com 歌词
- 解析HTML提取带ruby注音的歌词
- 本地缓存（搜索结果 + 歌词内容）
- 历史记录管理

CLI代码相对稳定，从原 `utabuild-cli` 项目直接复用。

### 2. Backend (`src-tauri/`)
Tauri v2 后端，负责：
- 暴露 `tauri::command` 给前端调用
- 调用CLI库执行搜索/获取歌词
- 平台特定逻辑（Android文件路径、Windows注册表等）
- 自动更新、通知、窗口管理

### 3. Frontend (`src/`)
Web前端（HTML/CSS/JS），负责：
- 搜索界面（输入框、搜索按钮、结果列表）
- 歌词显示（带振假名的Ruby渲染）
- 响应式布局（桌面 + 移动端自适应）
- 暗色模式

## 开发指南

### 环境要求
- Rust 1.77+
- Node.js 18+ (用于前端构建工具，可选)
- Tauri v2 CLI: `cargo install tauri-cli --version "^2"`

### 开发模式
```bash
# 前置：确保Rust toolchain已安装
rustup default stable

# 安装Tauri CLI
cargo install tauri-cli --version "^2"

# 运行开发服务器
cd /home/misaka/project/utabuild-tauri
cargo tauri dev
```

### 构建

#### Windows
```bash
cargo tauri build
# 输出: src-tauri/target/release/bundle/
```

#### Android
```bash
# 需要在Windows端用Android Studio
# 1. git pull 在Windows端
# 2. 用Android Studio打开项目
# 3. Build > Build Bundle(s) / APK(s)
```

## 关键技术决策

### Ruby（振假名）渲染方案
详见 `docs/RUBY_RENDERING.md`

### 为什么不直接用Slint？
Slint的DSL对LLM不友好，生成代码质量差。Tauri的前后端分离架构使用主流技术栈（Rust + HTML/CSS/JS），更适合LLM辅助开发。

### 为什么CLI独立为Library？
- CLI本身已经稳定，可独立测试
- Tauri后端通过 `use utabuild_cli::UtaTenSearcher` 直接调用
- 未来可以作为独立crate发布

## 项目结构

```
utabuild-tauri/
├── README.md                    # 本文件
├── docs/
│   └── RUBY_RENDERING.md        # Ruby显示方案文档
├── cli/                         # CLI库代码（从utabuild-cli复制）
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── models.rs            # 数据模型（LyricElement等）
│       ├── searcher.rs          # utaten搜索+解析
│       ├── cache_manager.rs     # 缓存管理
│       └── ...
├── src-tauri/                   # Tauri后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── lib.rs
│       └── main.rs
└── src/                         # 前端
    ├── index.html
    ├── css/
    │   └── style.css
    └── js/
        └── app.js
```
