# Agent Note: Project documentation library

Status: implemented

English | [中文](2026-08-22-project-documentation-library.zh.md)

## Problem

UtaBuild had a product README but no repository-local contract for architecture, subsystem ownership, durable decisions, bilingual documentation, or incident records. Future changes could place the same fact in the README, source comments, and implementation-specific notes without a mechanical way to detect the split.

## Decision

UtaBuild uses a structured documentation library with `.agents/notes/` for durable decisions and `docs/` for current architecture, subsystem contracts, procedures, user guides, postmortems, and translation rules. English and Simplified Chinese documents use sibling pairs with blob-hash sidecars. The project keeps source-derived facts in code or generators and uses the project-doc-library verifier for structural checks.

## Alternatives considered

Keeping the root README as the only documentation home would preserve a small tree but would mix user onboarding, architecture, procedures, and decision rationale. A flat `docs/` directory would improve visibility over the README-only state but would not assign ownership to different document purposes.

## Consequences

Non-trivial changes now require an Agent Note or an update to the existing owner. The root README remains a short product and entry-point overview; detailed development, architecture, user, subsystem, and fixture procedures live in their owning documents. Translation work carries a pairing obligation, and the structural verifier can detect missing lifecycle directories, stale sidecars, broken relative links, and mismatched bilingual structure. Project-specific documentation commands remain responsible for semantic accuracy and generated freshness.
