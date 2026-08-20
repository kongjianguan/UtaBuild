package com.utabuild.lsposed.ruby;

import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.Log;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;

/**
 * Hooks {@link Canvas#drawText} overloads to inject ruby (furigana) above CJK characters.
 *
 * <p>逐字歌词适配要点 (Salt Player 最新版):</p>
 * <ul>
 *   <li>Salt 的逐字模式会按"词/字"分批绘制，单次 {@code drawText} 可能包含 1~N 个字符（不再保证 len==1），</li>
 *   <li>振假名为"区间"概念（annotation 的 [start,end) 覆盖 1~N 个汉字，共享同一假名），因此需要在</li>
 *   <li>绘制区间宽度上居中绘制一次，而不是按汉字重复绘制同一个假名。</li>
 *   <li>Hook 覆盖 {@code Canvas} 的全部常用重载，避免新版切换到 char[]/CharSequence 分支后注音丢失。</li>
 * </ul>
 */
public final class RubyCanvasInjector {
    private static final String TAG = "UtaBuildRubyCanvas";
    private static final float RUBY_SIZE_RATIO = 0.45f;
    private static final float RUBY_Y_OFFSET_RATIO = 0.38f;
    private static final int MIN_LYRIC_FONT_PX = 18;
    private static final int MAX_LYRIC_FONT_PX = 320;

    private static volatile String currentLineText = "";
    private static volatile List<RubyChar> currentRubyChars = Collections.emptyList();
    private static volatile List<RubyAnnotation> currentAnnotations = Collections.emptyList();
    private static final ConcurrentMap<String, List<RubyChar>> lineRubyCache = new ConcurrentHashMap<>();
    private static final ConcurrentMap<String, List<RubyAnnotation>> lineAnnotationCache = new ConcurrentHashMap<>();
    private static volatile float currentTextSize = 0f;

    private static volatile int lineSearchCursor = 0;
    private static volatile String lastLineKey = "";
    private static final ConcurrentMap<String, Long> dedupeByKey = new ConcurrentHashMap<>();
    private static final long DEDUPE_MS = 80L;

    private RubyCanvasInjector() {}

    public static void onLyricsLine(String pureMainText, StructuredLyrics sessionLyrics) {
        if (pureMainText == null || pureMainText.isEmpty() || sessionLyrics == null) {
            clearLine();
            return;
        }
        String cacheKey = sessionLyrics.title + "\u0000" + pureMainText;
        List<RubyChar> cachedChars = lineRubyCache.get(cacheKey);
        List<RubyAnnotation> cachedAnn = lineAnnotationCache.get(cacheKey);
        if (cachedChars != null && cachedAnn != null) {
            currentLineText = pureMainText;
            currentRubyChars = cachedChars;
            currentAnnotations = cachedAnn;
            resetSearchCursorIfNeeded(cacheKey);
            return;
        }
        List<RubyChar> builtChars = buildCharMap(pureMainText, sessionLyrics);
        List<RubyAnnotation> builtAnn = buildAnnotationList(pureMainText, sessionLyrics);
        lineRubyCache.put(cacheKey, builtChars);
        lineAnnotationCache.put(cacheKey, builtAnn);
        currentLineText = pureMainText;
        currentRubyChars = builtChars;
        currentAnnotations = builtAnn;
        resetSearchCursorIfNeeded(cacheKey);
    }

    /** Word-level entry from w61 cells — optional hint to reset cursor per word batch. */
    public static void onLyricsWordCells(java.util.List<String> cells, StructuredLyrics sessionLyrics) {
        if (cells == null || cells.isEmpty() || sessionLyrics == null) return;
        // Word batch hint: ensure current line still valid; no extra state needed beyond line.
        // Keep cursor tracking for substring matching; word order matches line order.
    }

    public static void clearLine() {
        currentLineText = "";
        currentRubyChars = Collections.emptyList();
        currentAnnotations = Collections.emptyList();
        currentTextSize = 0f;
        lineSearchCursor = 0;
    }

    public static void clearCache() {
        lineRubyCache.clear();
        lineAnnotationCache.clear();
        dedupeByKey.clear();
        clearLine();
    }

    private static void resetSearchCursorIfNeeded(String cacheKey) {
        if (!cacheKey.equals(lastLineKey)) {
            lastLineKey = cacheKey;
            lineSearchCursor = 0;
        }
    }

    public static void beforeDrawText(Canvas canvas, String text, float x, float y, Paint paint) {
        if (canvas == null || text == null || paint == null || text.isEmpty()) return;
        List<RubyAnnotation> annotations = currentAnnotations;
        if (annotations.isEmpty()) return;
        String line = currentLineText;
        if (line.isEmpty()) return;
        float textSize = paint.getTextSize();
        if (textSize < MIN_LYRIC_FONT_PX || textSize > MAX_LYRIC_FONT_PX) return;
        currentTextSize = textSize;
        if (text.length() == 1) {
            char c = text.charAt(0);
            if (!isCjk(c)) return;
            String ruby = findRubyOrdered(c, currentRubyChars);
            if (ruby != null && !ruby.isEmpty()) {
                int offset = indexOfWithCursor(line, text, lineSearchCursor);
                if (offset >= 0) {
                    RubyAnnotation span = findSpanCovering(offset, annotations);
                    if (span != null) {
                        drawRubyForSpan(canvas, text, x, y, paint, span, offset);
                        advanceCursorAfterDraw(text, offset);
                        return;
                    }
                }
                drawRubySingle(canvas, text, x, y, paint, ruby);
                advanceCursorAfterDraw(text, -1);
            }
            return;
        }
        drawMultiChar(canvas, text, x, y, paint, line, annotations);
    }

    public static void beforeDrawText(Canvas canvas, char[] text, int index, int count, float x, float y, Paint paint) {
        if (text == null || count <= 0) return;
        int end = Math.min(text.length, index + count);
        int start = Math.max(0, index);
        if (start >= end) return;
        String s = new String(text, start, end - start);
        beforeDrawText(canvas, s, x, y, paint);
    }

    public static void beforeDrawText(Canvas canvas, CharSequence text, int start, int end, float x, float y, Paint paint) {
        if (text == null) return;
        int s = Math.max(0, start);
        int e = Math.min(text.length(), end);
        if (s >= e) return;
        String sub = text.subSequence(s, e).toString();
        beforeDrawText(canvas, sub, x, y, paint);
    }

    private static void drawMultiChar(Canvas canvas, String text, float x, float y, Paint paint,
                                      String line, List<RubyAnnotation> annotations) {
        int lineOffset = indexOfWithCursor(line, text, lineSearchCursor);
        if (lineOffset < 0) {
            fallbackPerChar(canvas, text, x, y, paint);
            return;
        }
        float[] charWidths = new float[text.length()];
        for (int i = 0; i < text.length(); i++) {
            charWidths[i] = paint.measureText(text, i, i + 1);
        }
        Set<RubyAnnotation> seen = new HashSet<>();
        for (int i = 0; i < text.length(); i++) {
            int absOffset = lineOffset + i;
            char c = text.charAt(i);
            if (!isCjk(c)) continue;
            RubyAnnotation span = findSpanCovering(absOffset, annotations);
            if (span == null || !seen.add(span)) continue;
            int spanStartInDraw = Math.max(span.start, lineOffset) - lineOffset;
            int spanEndInDraw = Math.min(span.end, lineOffset + text.length()) - lineOffset;
            if (spanEndInDraw <= spanStartInDraw) continue;
            float spanWidth = 0f;
            float spanPrefix = 0f;
            for (int k = 0; k < spanStartInDraw; k++) spanPrefix += charWidths[k];
            for (int k = spanStartInDraw; k < spanEndInDraw; k++) spanWidth += charWidths[k];
            if (spanWidth <= 0f) continue;
            String ruby = span.ruby;
            if (ruby == null || ruby.isEmpty()) continue;
            String dedupeKey = span.start + ":" + span.end + ":" + ruby + ":" + Math.round(x + spanPrefix);
            Long last = dedupeByKey.get(dedupeKey);
            long now = System.currentTimeMillis();
            if (last != null && now - last < DEDUPE_MS) continue;
            dedupeByKey.put(dedupeKey, now);
            Paint rubyPaint = makeRubyPaint(paint);
            float rubyWidth = rubyPaint.measureText(ruby);
            float rubyX = x + spanPrefix + (spanWidth - rubyWidth) / 2f;
            float rubyY = y - paint.getTextSize() * RUBY_Y_OFFSET_RATIO;
            try { canvas.drawText(ruby, rubyX, rubyY, rubyPaint); } catch (Throwable t) { Log.d(TAG, "draw ruby failed", t); }
        }
        advanceCursorAfterDraw(text, lineOffset);
    }

    private static void fallbackPerChar(Canvas canvas, String text, float x, float y, Paint paint) {
        float curX = x;
        for (int i = 0; i < text.length(); i++) {
            String ch = text.substring(i, i + 1);
            char c = ch.charAt(0);
            float w = paint.measureText(ch);
            if (isCjk(c)) {
                String ruby = findRubyOrdered(c, currentRubyChars);
                if (ruby != null && !ruby.isEmpty()) {
                    String key = c + ":" + ruby + ":" + Math.round(curX);
                    Long last = dedupeByKey.get(key);
                    long now = System.currentTimeMillis();
                    if (last == null || now - last >= DEDUPE_MS) {
                        dedupeByKey.put(key, now);
                        drawRubySingle(canvas, ch, curX, y, paint, ruby);
                    }
                }
            }
            curX += w;
        }
    }

    private static void drawRubySingle(Canvas canvas, String ch, float x, float y, Paint paint, String ruby) {
        String key = ch + ":" + ruby + ":" + Math.round(x) + ":" + Math.round(y);
        Long last = dedupeByKey.get(key);
        long now = System.currentTimeMillis();
        if (last != null && now - last < DEDUPE_MS) return;
        dedupeByKey.put(key, now);
        float charWidth = paint.measureText(ch);
        Paint rubyPaint = makeRubyPaint(paint);
        float rubyWidth = rubyPaint.measureText(ruby);
        float rubyX = x + (charWidth - rubyWidth) / 2f;
        float rubyY = y - paint.getTextSize() * RUBY_Y_OFFSET_RATIO;
        try { canvas.drawText(ruby, rubyX, rubyY, rubyPaint); } catch (Throwable t) { Log.d(TAG, "draw ruby single failed", t); }
    }

    private static void drawRubyForSpan(Canvas canvas, String ch, float x, float y, Paint paint, RubyAnnotation span, int offset) {
        drawRubySingle(canvas, ch, x, y, paint, span.ruby);
    }

    private static Paint makeRubyPaint(Paint original) {
        Paint rubyPaint = new Paint(original);
        rubyPaint.setTextSize(original.getTextSize() * RUBY_SIZE_RATIO);
        int originalColor = original.getColor();
        rubyPaint.setColor(originalColor);
        int alpha = (originalColor >>> 24) & 0xFF;
        if (alpha == 0) alpha = 0xFF;
        rubyPaint.setAlpha(alpha);
        rubyPaint.setAntiAlias(true);
        return rubyPaint;
    }

    private static String findRubyOrdered(char c, List<RubyChar> chars) {
        if (chars.isEmpty()) return null;
        for (RubyChar rc : chars) {
            if (rc.character == c && rc.ruby != null && !rc.ruby.isEmpty()) return rc.ruby;
        }
        return null;
    }

    private static List<RubyChar> buildCharMap(String pureMainText, StructuredLyrics lyrics) {
        if (pureMainText.isEmpty()) return Collections.emptyList();
        for (StructuredLyricLine line : lyrics.lines) {
            if (line.text.equals(pureMainText)) return buildFromAnnotations(line);
        }
        for (StructuredLyricLine line : lyrics.lines) {
            if (line.text.contains(pureMainText) || pureMainText.contains(line.text)) return buildFromAnnotations(line);
        }
        String normPure = normalizeForMatch(pureMainText);
        for (StructuredLyricLine line : lyrics.lines) {
            if (normalizeForMatch(line.text).contains(normPure) || normPure.contains(normalizeForMatch(line.text))) return buildFromAnnotations(line);
        }
        return Collections.emptyList();
    }

    private static List<RubyAnnotation> buildAnnotationList(String pureMainText, StructuredLyrics lyrics) {
        if (pureMainText.isEmpty()) return Collections.emptyList();
        for (StructuredLyricLine line : lyrics.lines) {
            if (line.text.equals(pureMainText)) return new ArrayList<>(line.annotations);
        }
        for (StructuredLyricLine line : lyrics.lines) {
            if (line.text.contains(pureMainText) || pureMainText.contains(line.text)) return remapAnnotations(line, pureMainText);
        }
        String normPure = normalizeForMatch(pureMainText);
        for (StructuredLyricLine line : lyrics.lines) {
            if (normalizeForMatch(line.text).contains(normPure) || normPure.contains(normalizeForMatch(line.text))) return remapAnnotations(line, pureMainText);
        }
        return Collections.emptyList();
    }

    private static List<RubyAnnotation> remapAnnotations(StructuredLyricLine line, String target) {
        int off = line.text.indexOf(target);
        List<RubyAnnotation> out = new ArrayList<>();
        if (off >= 0) {
            for (RubyAnnotation ann : line.annotations) {
                if (ann.start >= off && ann.end <= off + target.length()) out.add(new RubyAnnotation(ann.start - off, ann.end - off, ann.base, ann.ruby));
            }
        } else {
            off = target.indexOf(line.text);
            if (off >= 0) {
                for (RubyAnnotation ann : line.annotations) out.add(new RubyAnnotation(ann.start + off, ann.end + off, ann.base, ann.ruby));
            }
        }
        if (out.isEmpty()) return new ArrayList<>(line.annotations);
        return out;
    }

    private static String normalizeForMatch(String s) { return s.replaceAll("\\s+", "").toLowerCase(); }

    private static List<RubyChar> buildFromAnnotations(StructuredLyricLine line) {
        List<RubyChar> result = new ArrayList<>();
        for (RubyAnnotation ann : line.annotations) {
            if (!ann.isValid()) continue;
            if (ann.start < 0 || ann.end > line.text.length() || ann.end <= ann.start) continue;
            String base = line.text.substring(ann.start, ann.end);
            for (int i = 0; i < base.length(); i++) { char c = base.charAt(i); if (isCjk(c)) result.add(new RubyChar(c, ann.ruby)); }
        }
        return result;
    }

    private static RubyAnnotation findSpanCovering(int absOffset, List<RubyAnnotation> list) {
        for (RubyAnnotation a : list) if (a.start <= absOffset && absOffset < a.end) return a;
        return null;
    }

    private static int indexOfWithCursor(String line, String sub, int cursor) {
        if (sub.isEmpty() || line.isEmpty()) return -1;
        int idx = line.indexOf(sub, Math.max(0, Math.min(cursor, line.length())));
        if (idx >= 0) return idx;
        idx = line.indexOf(sub);
        return idx;
    }

    private static void advanceCursorAfterDraw(String text, int foundOffset) {
        if (foundOffset >= 0) lineSearchCursor = Math.min(currentLineText.length(), foundOffset + text.length());
        else if (!currentLineText.isEmpty() && lineSearchCursor >= currentLineText.length()) lineSearchCursor = 0;
    }

    private static boolean isCjk(char c) {
        Character.UnicodeScript script = Character.UnicodeScript.of(c);
        return script == Character.UnicodeScript.HAN;
    }

    static final class RubyChar {
        final char character;
        final String ruby;
        RubyChar(char character, String ruby) { this.character = character; this.ruby = ruby; }
    }
}

