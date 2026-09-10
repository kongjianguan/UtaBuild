# Agent Note: Match NetEase historical kana spellings during Ruby alignment

Status: implemented

English | [中文](2026-09-02-netease-historical-kana-ruby-boundary.zh.md)

## Problem

NetEase `romalrc` may spell the pronunciation of an original `づ` or `ぢ` as `ず` or `じ`. The Ruby aligner previously treated these orthographic variants as different when searching for an okurigana boundary. A failed boundary search fell back to consuming most of the remaining line, so `気づけなかったんだ` could attach `きずけなかったん` to `気`.

## Decision

The kana matcher in [`ruby_align.rs`](../../../../cli/src/ruby_align.rs) normalizes `ぢ` to `じ` and `づ` to `ず` for comparisons, alongside the existing particle and long-vowel equivalences. This matcher is used by anchor search, fallback boundary detection, and kana consumption, so the same rule applies throughout the alignment algorithm.

The generated Ruby remains the pronunciation derived from NetEase `romalrc`; the fix changes only boundary detection. Therefore `気づけ` is emitted with Ruby `きず`, consistent with the existing convention of preserving NetEase's pronunciation spelling instead of rewriting it to the source's orthography.

## Alternatives considered

**Rewrite `ず` and `じ` back to `づ` and `ぢ` in Ruby output.** This would mix source orthography into a reading string that otherwise comes from `romalrc`, and would require broader source-aware rewriting than the boundary defect needs.

**Switch NetEase to the tokenized LRC/YRC path.** That path distributes tokens by kanji blocks and does not reliably account for okurigana in this input shape. Changing the shared NetEase path would broaden the behavior change without addressing the matching invariant directly.

## Consequences

Lines whose source and romaji use these historical kana variants no longer trigger the over-consumption fallback. Existing lyrics and the public `LyricElement` shape remain unchanged, except that affected Ruby boundaries are now attached to the correct source text.

Already persisted Ruby or Salt bridge cache entries are not rewritten by this parser change. Affected songs need to be fetched again or have their cache cleared before an old malformed entry is replaced.

## Testing

[`lrc_parser.rs`](../../../../cli/src/lrc_parser.rs) contains a regression using the `spiral` line and its NetEase romaji. The CLI all-target test suite, the complete QRC pipeline fixture test, and `src-tauri` compilation pass with the change.
