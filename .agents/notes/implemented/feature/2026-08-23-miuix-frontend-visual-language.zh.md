# Agent Note: Use Miuix-inspired frontend visual language

Status: implemented

[English](2026-08-23-miuix-frontend-visual-language.md) | 中文

## Problem

前端原本使用偏 web 的卡片、边框、紧凑按钮和仅位于底部的导航模式，无法表达原生移动端或桌面应用应有的层级。搜索、保存曲、设置、结果列表、歌词和对话框也各自使用不同的 surface 与控件样式。

## Decision

UtaBuild 在现有 HTML 和 SCSS 前端中实现 Miuix 风格的视觉语言。共享 token 定义圆角 surface、主按钮与次要按钮层级、分组 preference 行、40px 控件、48px 搜索框以及基于缩放的按压反馈。窄屏使用悬浮式底部 `NavigationBar`；宽度至少为 960px 时使用左侧 `NavigationRail`。

路由器将搜索、保存曲和设置保留为一级视图。这些页面通过主导航进入，不显示返回操作。结果、歌词、关于、LSP 设置和 LSP 日志属于嵌套视图，在左上角提供返回操作。页面滚动被锁定；只有显式的 view scroll 容器和 LSP 日志区域可以滚动。窄屏搜索在获得焦点后，从只有曲名的输入框展开为曲名、歌手和操作按钮。viewport guard 禁用双指缩放和带 modifier 的浏览器缩放手势。

实现继续使用现有的静态 TypeScript/HTML/SCSS 架构，不新增 Miuix runtime 依赖。MyGO 主题保留自己的配色和星空背景，同时使用共享的几何和组件层级。

## Alternatives considered

继续局部调整按钮和卡片会让每个页面保留不同的视觉语法，无法建立可复用的层级。直接接入平台组件库不适合当前静态 HTML/SCSS 前端；将 Miuix 的组件角色映射为共享 SCSS token 可以保留现有 runtime boundary。

## Consequences

视觉变更集中在 `src/scss/tokens/`、`src/scss/themes/`、`src/scss/layouts/_app.scss` 和各组件 partial 中。新增控件应使用共享的按钮、surface、preference 行、顶部栏和导航模式，不应引入一次性的几何样式。桌面端和窄屏端有意使用不同的主导航位置。静态浏览器预览仍然是必要的视觉验证，因为前端构建不能证明布局行为。

## Testing

前端变更运行 `pnpm run build`、`pnpm run lint` 和 `git diff --check`。视觉检查覆盖窄屏与宽屏、一级和嵌套页面导航、搜索展开、内部滚动以及 dark、light 和 MyGO 主题。
