# Agent Note: Use pnpm for frontend tooling

Status: implemented

English | [中文](2026-08-22-pnpm-frontend-tooling.zh.md)

## Problem

The repository's frontend scripts, Tauri build hooks, documentation, and dependency lockfile used npm while the project workflow used pnpm. Keeping both conventions made the documented commands and the commands invoked by Tauri diverge, and allowed two package-manager lockfiles to become competing sources of dependency state.

## Decision

UtaBuild uses pnpm for frontend dependency installation and script execution. `pnpm-lock.yaml` is the frontend dependency lockfile, `package-lock.json` is not maintained, package scripts compose through `pnpm run`, and Tauri invokes `pnpm run build` for development and release builds. The repository pins `pnpm@11.17.0` in `package.json`; pnpm permits the `@parcel/watcher` build required by the installed Sass version through `pnpm-workspace.yaml`.

## Alternatives considered

Keeping npm would preserve the existing lockfile but would conflict with the requested project workflow. Supporting both npm and pnpm would keep more entry points working, but would retain duplicate lockfile and documentation maintenance and leave Tauri's package-manager choice implicit.

## Consequences

Frontend commands in repository documentation use pnpm. A fresh checkout runs `pnpm install` from `pnpm-lock.yaml`, and `cargo tauri dev` or `cargo tauri build` invokes the same pnpm build path. Native dependency build approval is an explicit project configuration rather than an interactive setup step. Changes to frontend dependencies must update `package.json` and `pnpm-lock.yaml` together.
