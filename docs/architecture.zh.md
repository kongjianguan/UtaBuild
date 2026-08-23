# UtaBuild 架构

[English](architecture.md) | 中文

UtaBuild 是一个包含可复用 Rust 歌词流水线的跨平台歌词应用。Tauri 应用负责桌面端和移动端命令接口，TypeScript 前端负责展示和交互，Android LSPosed 源码负责 Salt Player 集成。

## 组合关系

```text
src/                         TypeScript、HTML 和 SCSS 前端
└── invoke() ────────────────┐
                             ▼
src-tauri/src/lib.rs         Tauri 命令和应用状态
└── utabuild-cli             ▼
cli/src/                     搜索、解析、对齐、缓存、历史、输出

src-tauri/android-lsposed/   Salt Player hook 和结构化歌词 bridge
```

`cli` crate 是共享领域层。`src-tauri` 通过本地 path dependency 引用它，因此搜索、解析、Ruby 对齐、缓存模型和输出模型不会在前端重复实现。

## 运行流程

正常搜索流程如下：

1. 前端收集标题、可选歌手、数据源偏好和缓存偏好。
2. `invoke('search_lyrics')` 调用 Tauri 命令。
3. Tauri 命令先检查搜索缓存，再将请求交给 `UtaTenSearcher`，使用 UtaTen、QQ Music 或 NetEase 数据源。
4. 前端展示 `SearchResponse.results`，用户选择一条结果。
5. `invoke('get_lyrics')` 获取或读取结构化 `ruby_annotations`，保存歌曲，并可以写入 Salt bridge 缓存。
6. `ruby.ts` 将文本、Ruby 注音和换行渲染到歌词视图。

`search_and_get` 是一步完成路径。它返回缓存中的成功结果，或选择第一条结果后保存同样的结构化输出。

## 核心边界

### 前端

`src/ts/` 负责 DOM 状态、视图路由、设置、搜索结果交互、已保存歌曲、日志页面、导出控件和 Ruby 渲染。它不负责网络获取或歌词解析。`src/ts/tauri.ts` 是调用边界，也为非 Tauri 浏览器环境提供 mock 路径。

### Tauri 后端

`src-tauri/src/lib.rs` 负责 `AppState` 中的 searcher、Tauri 命令注册、缓存编排、已保存歌词持久化、LSP 设置与日志、Salt 启动请求以及 HTML 导出。后端将 Rust 结果转换为前端消费的 JSON value。

### CLI 领域层

`cli/src/searcher.rs` 负责数据源访问和数据源偏好路由。`lrc_parser.rs` 与 `qrc_parser.rs` 将带时间歌词格式转换为 `LyricElement`。`ruby_align.rs` 和 tokenized LRC/YRC 路径生成 `ruby` 元素。`cache.rs`、`cache_manager.rs` 和 `commands/history.rs` 负责本地持久化与历史记录。`output.rs` 和 `output_html.rs` 负责 JSON 与 HTML 契约。

### Android LSPosed bridge

`src-tauri/android-lsposed/` 中的 Java 代码运行在 Salt Player 进程内。`UtaBuildSaltModule` 观察 Salt 生命周期和歌词绘制路径。`UtaBuildLyricProvider` 从 UtaBuild ContentProvider、本地文件或 loopback bridge 读取结构化歌词。`RubyAlignmentEngine` 将 UtaBuild 注音映射到 Salt 观察到的歌词文本，`RubyCanvasInjector`/`SaltRubyRenderer` 在原字符上方绘制读音。

## 数据契约

共享 element 契约是由 `type: text | ruby | linebreak` 组成的列表。文本使用 `base`，Ruby 使用 `base` 和 `ruby`。JSON 输出将这些 element 包装在 `found_title`、`found_artist`、`lyrics_url` 和 `ruby_annotations` 等响应字段中。

缓存键包含数据源信息。QQ Music 和 NetEase 在可用时使用数据源专属标识符；UtaTen 使用歌词 URL。删除已保存歌曲时也会失效内存歌词缓存，避免后续缓存命中重新创建已删除文件。

## 新行为的归属

- 数据源获取、解析、对齐或共享输出行为放入 `cli/src/`。
- 桌面/移动端命令编排或持久化放入 `src-tauri/src/lib.rs`。
- 交互、视图状态或渲染放入 `src/ts/`。
- Salt Player hook 或 bridge 行为放入 `src-tauri/android-lsposed/`。
- 新增持久边界或改变共享契约时，更新所属子系统页面和 Agent Note。

不要把数据源网络逻辑放入 TypeScript。`cli` crate 可以负责的 Rust 领域行为，不要在 Tauri 命令中复制一份。
