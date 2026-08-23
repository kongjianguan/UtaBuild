# Agent Note: Use Miuix-inspired frontend visual language

Status: implemented

English | [中文](2026-08-23-miuix-frontend-visual-language.zh.md)

## Problem

The frontend used web-style cards, borders, compact buttons, and a bottom-only navigation pattern that did not communicate the same hierarchy as a native mobile or desktop application. Search, saved songs, settings, result lists, lyrics, and dialogs also had separate surface and control treatments.

## Decision

UtaBuild implements a Miuix-inspired visual language in the existing HTML and SCSS frontend. Shared tokens define rounded surfaces, grouped preference rows, primary and secondary button roles, 40px controls, 48px search fields, and scale-based press feedback. Small windows use a floating bottom `NavigationBar`; windows at least 960px wide use a left `NavigationRail`.

The router keeps search, saved songs, and settings as first-level views. These views are entered through the main navigation and do not show a back action. Results, lyrics, About, LSP settings, and LSP logs are nested views and expose a top-left back action. Page scrolling is locked; only explicit view scroll containers and the LSP log surface can scroll. Narrow search expands from a title-only field into title, artist, and action fields on focus. The viewport guard disables pinch zoom and modifier-based browser zoom gestures.

The implementation keeps the existing static TypeScript/HTML/SCSS architecture and does not add a Miuix runtime dependency. The MyGO theme retains its palette and starfield while using the shared geometry and component hierarchy.

## Alternatives considered

Continuing with local button and card adjustments would leave each view with a separate visual grammar and would not establish a reusable hierarchy. A direct platform component-library integration would not fit the current static HTML/SCSS frontend; mapping the relevant Miuix roles to shared SCSS tokens preserves the existing runtime boundary.

## Consequences

Visual changes are centralized in `src/scss/tokens/`, `src/scss/themes/`, `src/scss/layouts/_app.scss`, and the component partials. New controls should use the shared button, surface, preference-row, top-bar, and navigation patterns instead of introducing one-off geometry. Desktop and narrow-window layouts intentionally use different primary navigation placements. The static browser preview remains necessary for visual verification because the frontend build does not prove layout behavior.

## Testing

Frontend changes run `pnpm run build`, `pnpm run lint`, and `git diff --check`. Visual checks cover narrow and wide windows, first-level and nested navigation, search expansion, internal scrolling, and dark, light, and MyGO themes.
