# CLI 与歌词流水线

[English](cli.md) | 中文

`cli` crate 是 UtaBuild 共享的歌词领域层，编译产物名为 `utabuild-cli`，Tauri 后端通过本地 path dependency 引用它。

## 职责

- 负责 UtaTen 分页搜索、限速、响应解码和 HTML 提取。
- 通过独立数据源适配器搜索并获取 QQ Music 与 NetEase 歌词。
- 将 LRC、YRC 和 QRC 输入解析为规范化歌词 element。
- 将罗马音或数据源时间信息与汉字、假名对齐，生成 Ruby 注音。
- 持久化搜索与歌词缓存，管理历史记录，并序列化 JSON 或 HTML 输出。

## 核心契约

`LyricElement` 是共享渲染模型。`text` 携带 `base`，`ruby` 携带 `base` 和 `ruby`，`linebreak` 不携带这两个字段。`LyricsSearchResponse` 表示搜索和选择状态，`LyricsOutput` 表示结构化歌词输出。

`UtaTenSearcher` 负责数据源偏好路由和提供方编排。`ArtworkSourcePreference` 与 `LyricSourcePreference` 将用户设置规范化为支持的数据源：UtaTen、QQ Music、NetEase 或自动选择。

## 解析与对齐

`lrc_parser.rs` 处理带时间的 LRC/YRC 行，并可以按时间对齐罗马音轨。`qrc_parser.rs` 解析 QQ Music 的 XML 风格逐词时间格式，并执行字符级重叠对齐。`ruby_align.rs` 提供通用的文本到读音对齐和清理路径。缺少、为空或没有时间重叠的读音回退为纯文本。

## 持久化与输出

`CacheManager` 负责内存搜索和歌词缓存。文件缓存与历史模块将 JSON 持久化到平台数据目录或缓存目录。`output.rs` 定义 CLI、Tauri 后端和 Android bridge 消费的 JSON 字段；`output_html.rs` 在生成 HTML 前转义标题、歌手、URL 和歌词文本。

## 验证

运行 `cargo test --manifest-path cli/Cargo.toml`。`cli/tests/lyrics_pipeline_test.rs` 将 QRC fixture 输出与 expected JSON 比较，并覆盖空罗马音和无时间重叠时的纯文本回退。
