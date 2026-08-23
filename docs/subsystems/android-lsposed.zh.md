# Android LSPosed bridge

[English](android-lsposed.md) | 中文

Android LSPosed 源码将 UtaBuild 集成到 Salt Player。它运行在目标播放器进程中，因此使用低侵入 bridge，而不是直接调用 Tauri/Rust 命令接口。

## 数据流

`UtaBuildSaltModule` hook Salt 的应用和 Activity 生命周期、歌曲打开/更新事件、歌词行 accessor 以及 `Canvas.drawText`。`UtaBuildLyricProvider` 按以下顺序解析结构化歌词：UtaBuild ContentProvider、本地 Downloads fixture，最后是 loopback HTTP endpoint。

provider 返回包含歌词行和字符范围 `RubyAnnotation` 的 `StructuredLyrics`。`RubyAlignmentEngine` 将这些注音与 Salt 观察到的歌词文本对齐。`RubyCanvasInjector` 和 `SaltRubyRenderer` 在原字符上方绘制读音，同时保留宿主渲染器处理普通文本。

## ContentProvider 契约

`UtaBuildLyricContentProvider` 暴露 `fyi.kongjianguan.utabuild.lyrics` authority。`lyrics` query 返回结构化 JSON；`pending` 接收 Salt 启动请求；`logs` 接收 bridge 日志写入并返回日志；`settings` 返回同步后的 LSP 设置。provider 对歌词 query 是只读的，并使用镜像应用文件作为传输存储。

## 失败行为

bridge 查询失败不会终止宿主流程。provider 会尝试下一个来源；没有可用 Ruby 数据时，Salt hook 保留宿主歌词。文件名和日志字段在跨越 bridge 前会被清理。

## 验证

生成项目和 SDK 可用时，通过 Android 构建验证 Java 编译。算法变更保持 `RubyAlignmentEngine` 和结构化 JSON fixture 聚焦于字符范围、假名规范化、重复抑制和纯文本回退。
