# 前端

[English](frontend.md) | 中文

前端是位于 `src/` 下的静态 TypeScript、HTML 和 SCSS 应用。Tauri 将构建后的 `src/` 目录作为前端产物；浏览器环境可以使用 `src/ts/tauri.ts` 中的 mock 调用路径。

## 职责

`app.ts` 初始化控件，并协调搜索、已保存歌曲和设置三个一级视图。`dom.ts` 负责视图路由、DOM 访问、加载状态、toast、滚动位置恢复和共享控件状态。`search.ts` 负责提交搜索、数据源 tab、分页、结果选择和歌词获取。`songs.ts` 负责已保存歌词浏览和元数据刷新。

`ruby.ts` 将 `LyricElement[]` 转换为 DOM 节点。Ruby 注音使用 `<ruby>`/`<rt>` 元素，非 Ruby element 使用纯文本。`settings.ts`、`cache.ts`、`lsp.ts` 和 `export.ts` 分别负责设置、缓存、bridge 日志和导出交互。

## 后端边界

通过 `src/ts/tauri.ts` 使用 `invoke`。前端传递 `title`、`artist`、`page`、`useCache`、`lyricSource` 和 `artworkSource` 等可序列化选项，并消费 `types.ts` 中定义的响应形状。不要从浏览器层直接请求歌词数据源。

## 状态与视图

路由器区分搜索、已保存歌曲、设置三个一级视图，以及结果、歌词、关于、LSP 设置和 LSP 日志嵌套视图。一级的已保存歌曲页和设置页没有返回操作；嵌套页面在左上角提供返回操作。视图切换会保存和恢复滚动位置。

前端使用 SCSS 实现 Miuix 风格的视觉系统，不直接引入平台组件库。共享 token 定义圆角 surface、分组 preference 行、40px 控件、48px 搜索框、主按钮与次要按钮层级以及按压反馈。窄屏使用悬浮式底部 `NavigationBar`；宽度至少为 960px 时使用左侧 `NavigationRail`。搜索、结果、保存曲、设置、日志、对话框和歌词页面共享相同的 surface 与控件 token，MyGO 主题保留自己的配色和星空背景。

文档和 body 锁定页面滚动。只有显式标记 `data-view-scroll` 的元素，以及支持横向和纵向滚动的 LSP 日志区域可以滚动。窄屏搜索表单收起时只显示曲名输入框和操作按钮；输入框获得焦点后展开为曲名、歌手和操作按钮。viewport guard 禁用双指缩放及带 modifier 的浏览器缩放手势。

搜索状态保留每个数据源的响应和合并结果列表；设置持久化到本地，并在 Tauri 可用时与后端同步 LSP 设置。

## 验证

前端变更运行 `pnpm run build:ts`、`pnpm run lint` 和 `pnpm run build:scss`。使用 `http://127.0.0.1:4173/` 的静态浏览器路径检查窄屏与宽屏布局、一级和嵌套页面导航、内部滚动、搜索展开和主题渲染；它不能证明 Tauri command 行为。
