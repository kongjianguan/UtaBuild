package com.utabuild.lsposed.ruby;

import java.util.List;

public final class SaltRubyRenderer {
    private SaltRubyRenderer() {}

    /**
     * Preserve Salt's own lyric text and styling by emitting a guide line immediately above the
     * original line. Salt still renders both lines with its existing font/color/effects.
     */
    public static String renderInterlinear(String originalLine, List<RubyAnnotation> annotations) {
        if (originalLine == null || annotations == null || annotations.isEmpty()) {
            return originalLine;
        }
        String prefix = extractLeadingLrcTimestamps(originalLine);
        String displayLine = RubyAlignmentEngine.stripLrcTimestamps(originalLine);
        String rubyGuide = buildRubyGuide(displayLine, annotations);
        if (rubyGuide.trim().isEmpty()) {
            return originalLine;
        }
        return prefix + rubyGuide + "\n" + originalLine;
    }

    static String buildRubyGuide(String displayLine, List<RubyAnnotation> annotations) {
        StringBuilder guide = new StringBuilder();
        int cursor = 0;
        for (RubyAnnotation annotation : annotations) {
            if (!annotation.isValid()) {
                continue;
            }
            int start = Math.max(0, Math.min(annotation.start, displayLine.length()));
            while (cursor < start) {
                int cp = displayLine.codePointAt(cursor);
                if (Character.isWhitespace(cp)) {
                    guide.appendCodePoint(cp);
                } else {
                    guide.append('\u3000');
                }
                cursor += Character.charCount(cp);
            }
            guide.append(annotation.ruby);
            cursor = Math.max(cursor, Math.min(annotation.end, displayLine.length()));
        }
        return guide.toString();
    }

    private static String extractLeadingLrcTimestamps(String line) {
        if (line == null) {
            return "";
        }
        int index = 0;
        StringBuilder prefix = new StringBuilder();
        while (index < line.length()) {
            while (index < line.length() && Character.isWhitespace(line.charAt(index))) {
                prefix.append(line.charAt(index));
                index++;
            }
            if (index >= line.length() || line.charAt(index) != '[') {
                break;
            }
            int end = line.indexOf(']', index);
            if (end < 0) {
                break;
            }
            String tag = line.substring(index, end + 1);
            if (!tag.matches("\\[[0-9]{1,2}:[0-9]{2}(?:[.:][0-9]{1,3})?]")) {
                break;
            }
            prefix.append(tag);
            index = end + 1;
        }
        return prefix.toString();
    }
}
