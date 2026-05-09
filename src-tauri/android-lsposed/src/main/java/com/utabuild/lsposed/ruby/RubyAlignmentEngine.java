package com.utabuild.lsposed.ruby;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;

public final class RubyAlignmentEngine {
    private int cursor;

    public List<RubyAnnotation> align(StructuredLyrics utaBuildLyrics, String saltLine) {
        if (utaBuildLyrics == null || saltLine == null || saltLine.trim().isEmpty()) {
            return Collections.emptyList();
        }

        String displayLine = stripLrcTimestamps(saltLine);
        NormalizedText normalizedSalt = normalize(displayLine);
        if (normalizedSalt.normalized.isEmpty()) {
            return Collections.emptyList();
        }

        Match best = null;
        int start = Math.max(0, cursor - 3);
        for (int i = start; i < utaBuildLyrics.lines.size(); i++) {
            StructuredLyricLine candidate = utaBuildLyrics.lines.get(i);
            if (!candidate.hasRuby()) {
                continue;
            }
            Match match = matchLine(candidate, normalizedSalt, displayLine);
            if (match == null) {
                continue;
            }
            match.lineIndex = i;
            if (best == null || match.score > best.score) {
                best = match;
            }
            if (match.score >= 100) {
                break;
            }
        }

        if (best == null) {
            return Collections.emptyList();
        }
        cursor = best.lineIndex + 1;
        return best.annotations;
    }

    private static Match matchLine(StructuredLyricLine utaLine, NormalizedText normalizedSalt, String displaySaltLine) {
        NormalizedText normalizedA = normalize(utaLine.text);
        if (normalizedA.normalized.isEmpty()) {
            return null;
        }

        int normalizedOffset = normalizedSalt.normalized.indexOf(normalizedA.normalized);
        int score = 100;
        if (normalizedOffset < 0) {
            normalizedOffset = normalizedA.normalized.indexOf(normalizedSalt.normalized);
            score = 75;
        }
        if (normalizedOffset < 0) {
            return null;
        }

        List<RubyAnnotation> mapped = new ArrayList<>();
        if (score == 100) {
            for (RubyAnnotation annotation : utaLine.annotations) {
                RubyAnnotation mappedAnnotation = mapAnnotationFromA(annotation, normalizedA, normalizedSalt, normalizedOffset, displaySaltLine);
                if (mappedAnnotation != null && mappedAnnotation.isValid()) {
                    mapped.add(mappedAnnotation);
                }
            }
        } else {
            for (RubyAnnotation annotation : utaLine.annotations) {
                RubyAnnotation mappedAnnotation = mapAnnotationFromSaltSubset(annotation, normalizedA, normalizedSalt, normalizedOffset, displaySaltLine);
                if (mappedAnnotation != null && mappedAnnotation.isValid()) {
                    mapped.add(mappedAnnotation);
                }
            }
        }

        if (mapped.isEmpty()) {
            return null;
        }
        mapped.sort(Comparator.comparingInt(a -> a.start));
        return new Match(score, mapped);
    }

    private static RubyAnnotation mapAnnotationFromA(
            RubyAnnotation annotation,
            NormalizedText normalizedA,
            NormalizedText normalizedSalt,
            int normalizedOffset,
            String displaySaltLine
    ) {
        int startNorm = normalizedIndexAtOriginalOffset(normalizedA, annotation.start);
        int endNorm = normalizedIndexAtOriginalOffset(normalizedA, annotation.end - 1);
        if (startNorm < 0 || endNorm < startNorm) {
            return null;
        }
        int saltStartNorm = normalizedOffset + startNorm;
        int saltEndNorm = normalizedOffset + endNorm;
        if (saltStartNorm < 0 || saltEndNorm >= normalizedSalt.originalIndexes.size()) {
            return null;
        }
        int saltStart = normalizedSalt.originalIndexes.get(saltStartNorm);
        int saltEnd = normalizedSalt.originalIndexes.get(saltEndNorm) + 1;
        String base = safeSubstring(displaySaltLine, saltStart, saltEnd);
        return new RubyAnnotation(saltStart, saltEnd, base, annotation.ruby);
    }

    private static RubyAnnotation mapAnnotationFromSaltSubset(
            RubyAnnotation annotation,
            NormalizedText normalizedA,
            NormalizedText normalizedSalt,
            int normalizedOffsetInA,
            String displaySaltLine
    ) {
        int startNorm = normalizedIndexAtOriginalOffset(normalizedA, annotation.start);
        int endNorm = normalizedIndexAtOriginalOffset(normalizedA, annotation.end - 1);
        if (startNorm < 0 || endNorm < startNorm) {
            return null;
        }
        int relativeStart = startNorm - normalizedOffsetInA;
        int relativeEnd = endNorm - normalizedOffsetInA;
        if (relativeStart < 0 || relativeEnd >= normalizedSalt.originalIndexes.size()) {
            return null;
        }
        int saltStart = normalizedSalt.originalIndexes.get(relativeStart);
        int saltEnd = normalizedSalt.originalIndexes.get(relativeEnd) + 1;
        String base = safeSubstring(displaySaltLine, saltStart, saltEnd);
        return new RubyAnnotation(saltStart, saltEnd, base, annotation.ruby);
    }

    private static int normalizedIndexAtOriginalOffset(NormalizedText text, int originalOffset) {
        int result = -1;
        for (int i = 0; i < text.originalIndexes.size(); i++) {
            int index = text.originalIndexes.get(i);
            if (index <= originalOffset) {
                result = i;
            } else {
                break;
            }
        }
        return result;
    }

    public static String stripLrcTimestamps(String line) {
        if (line == null) {
            return "";
        }
        return line.replaceAll("^(\\s*\\[[0-9]{1,2}:[0-9]{2}(?:[.:][0-9]{1,3})?])+", "");
    }

    static NormalizedText normalize(String text) {
        StringBuilder normalized = new StringBuilder();
        List<Integer> originalIndexes = new ArrayList<>();
        if (text == null) {
            return new NormalizedText("", originalIndexes);
        }
        for (int i = 0; i < text.length(); ) {
            int cp = text.codePointAt(i);
            int type = Character.getType(cp);
            if (!Character.isWhitespace(cp)
                    && type != Character.CONNECTOR_PUNCTUATION
                    && type != Character.DASH_PUNCTUATION
                    && type != Character.START_PUNCTUATION
                    && type != Character.END_PUNCTUATION
                    && type != Character.INITIAL_QUOTE_PUNCTUATION
                    && type != Character.FINAL_QUOTE_PUNCTUATION
                    && type != Character.OTHER_PUNCTUATION) {
                normalized.appendCodePoint(Character.toLowerCase(cp));
                originalIndexes.add(i);
            }
            i += Character.charCount(cp);
        }
        return new NormalizedText(normalized.toString(), originalIndexes);
    }

    private static String safeSubstring(String text, int start, int end) {
        int safeStart = Math.max(0, Math.min(start, text.length()));
        int safeEnd = Math.max(safeStart, Math.min(end, text.length()));
        return text.substring(safeStart, safeEnd);
    }

    static final class NormalizedText {
        final String normalized;
        final List<Integer> originalIndexes;

        NormalizedText(String normalized, List<Integer> originalIndexes) {
            this.normalized = normalized;
            this.originalIndexes = originalIndexes;
        }
    }

    private static final class Match {
        final int score;
        final List<RubyAnnotation> annotations;
        int lineIndex;

        Match(int score, List<RubyAnnotation> annotations) {
            this.score = score;
            this.annotations = annotations;
        }
    }
}
