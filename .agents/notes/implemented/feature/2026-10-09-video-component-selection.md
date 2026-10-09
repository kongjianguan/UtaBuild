# Agent Note: Select video components independently of editing operations

Status: implemented

English | [中文](2026-10-09-video-component-selection.zh.md)

## Problem

Song-wide row positioning, editing selected lyric occurrences, and moving several rows within one occurrence need the same object operations. Content removal must preserve deliberate placement until the user requests layout.

## Decision

The editor separates object selection from movement, visibility, deletion, and layout. Preview and object-list selection follow the [Keynote object-list pattern](https://support.apple.com/en-ie/guide/keynote/tanc5f5e5382/mac); occurrence ranges and component-type filters extend selection across the recording. Parent and descendant selections are normalized before applying offsets, so every object moves once. Source Ruby relationships drive optional linked selection.

Version 2 edit documents preserve visibility and arrangement separately from content and position. Automatic layout defaults to disabled in each orientation. Fixed arrangements bind component anchors, card rectangles, and page assignments to the shared Pillow composition. One-shot scene layout explicitly replaces those placements and removes scene offsets in the selected orientation. This supersedes the automatic layout policy in the [teaching-video decision](2026-10-03-song-teaching-video.md), which retains ownership of preparation, content authorship, and encoding.

Selection is transient browser state. Each persisted batch operation creates one history entry, and export captures the complete edit document. The preview catalog includes other occurrences, other pages, and hidden objects so rendered visibility does not determine what can be selected.

## Alternatives considered

**Dedicated commands for each lyric row and occurrence scope** duplicate operation behavior and make combinations such as hiding romaji and translation in selected occurrences require separate interfaces. Type and occurrence filters express those combinations through the shared selection mechanism.

**Continuous layout after every edit** changes unselected components when objects are removed or hidden. An explicit layout operation gives those spatial changes an observable scope and one undo step.

## Consequences

Fixed layouts retain gaps after hiding or deletion. Text can require explicit layout when its size changes. The renderer continues to reject unavailable glyphs, picture overflow, and insufficient teaching-page time. One-shot layout affects all visible objects in the chosen occurrences, and its interface names that scope independently of type filters.

Hidden objects retain their contents and manual offsets. Restoring visibility may expose overlap after the user has explicitly arranged the remaining objects. Undo restores a batch's complete document, including arrangement records and offsets.

## Testing

[`editor-selection-e2e.mjs`](../../../../tools/teaching-video/editor-selection-e2e.mjs) runs against the real local service and Chrome, using prepared recording, lyric, and teaching inputs. It checks Shift selection, pointer rectangle selection, three-row movement, parent-child normalization, cross-occurrence movement, marked-occurrence filtering, batch visibility and deletion, fixed geometry, one-shot layout, undo and redo, independent orientations, save and reopen, responsive layouts, and complete MP4 exports. The evidence includes actual preview images and component geometry, the saved document, an interaction report, and independent full-song media reports.

The [subsystem reference](../../../../docs/subsystems/teaching-video.md) owns the runtime contract, and the [user tutorial](../../../../docs/user/teaching-video.md) owns the execution procedure.
