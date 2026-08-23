# Agent Note: Use pnpm for frontend tooling

Status: implemented

[English](2026-08-22-pnpm-frontend-tooling.md) | 中文

## Problem

仓库的前端脚本、Tauri 构建 hook、文档和依赖锁文件使用 npm，但项目实际工作流使用 pnpm。两套约定并存会让文档命令与 Tauri 调用的命令不一致，也会让两个包管理器锁文件竞争依赖状态的所有权。

## Decision

UtaBuild 使用 pnpm 安装前端依赖并执行脚本。`pnpm-lock.yaml` 是前端依赖锁文件，不再维护 `package-lock.json`；package scripts 使用 `pnpm run` 组合，Tauri 在开发和发布构建中调用 `pnpm run build`。仓库在 `package.json` 中固定 `pnpm@11.17.0`，并通过 `pnpm-workspace.yaml` 允许当前 Sass 版本所需的 `@parcel/watcher` 构建。

## Alternatives considered

继续使用 npm 可以保留现有锁文件，但会与项目要求的工作流冲突。兼容 npm 和 pnpm 可以保留更多入口，但会继续产生双锁文件和双份文档维护，也会让 Tauri 使用哪个包管理器变得不明确。

## Consequences

仓库文档中的前端命令统一使用 pnpm。全新 checkout 从 `pnpm-lock.yaml` 执行 `pnpm install`，`cargo tauri dev` 和 `cargo tauri build` 调用同一条 pnpm 构建路径。原生依赖的构建许可由项目配置显式声明，不再依赖交互式初始化。前端依赖变更必须同时更新 `package.json` 和 `pnpm-lock.yaml`。
