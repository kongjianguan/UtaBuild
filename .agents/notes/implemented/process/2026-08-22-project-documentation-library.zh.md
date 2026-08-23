# Agent Note: Project documentation library

Status: implemented

[English](2026-08-22-project-documentation-library.md) | 中文

## Problem

UtaBuild 原本只有产品 README，没有关于架构、子系统归属、持久决策、双语文档或事故记录的仓库级约定。未来变更可能把同一事实分别写入 README、源码注释和实现细节记录，而没有机械方式发现这些事实已经分散。

## Decision

UtaBuild 使用结构化文档库：`.agents/notes/` 负责持久决策，`docs/` 负责当前架构、子系统契约、流程、用户指南、事故复盘和翻译规则。英文和简体中文文档使用同目录文件对，并通过 blob-hash sidecar 记录一致性。源码派生事实保留在代码或生成器中，项目使用 project-doc-library verifier 执行结构检查。

## Alternatives considered

只保留根 README 可以维持小目录，但会混合用户入门、架构、流程和决策理由。使用扁平 `docs/` 目录比 README-only 状态更容易发现文档，但仍然无法为不同文档目的分配归属。

## Consequences

非平凡变更现在必须新增 Agent Note，或更新已经拥有该决策的记录。根 README 保持为简短的产品和入口概览；详细的开发、架构、用户、子系统和 fixture 流程由各自的 owning document 负责。翻译工作需要同步维护配对文件，结构校验器可以发现缺失生命周期目录、过期 sidecar、断开的相对链接和双语结构差异。项目专属的文档检查仍然负责语义准确性和生成产物新鲜度。
