package com.utabuild.lsposed.ruby;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public final class StructuredLyrics {
    public final String title;
    public final String artist;
    public final List<StructuredLyricLine> lines;

    public StructuredLyrics(String title, String artist, List<StructuredLyricLine> lines) {
        this.title = title == null ? "" : title;
        this.artist = artist == null ? "" : artist;
        this.lines = lines == null ? Collections.emptyList() : Collections.unmodifiableList(new ArrayList<>(lines));
    }

    public boolean isUsable() {
        for (StructuredLyricLine line : lines) {
            if (line.hasRuby()) {
                return true;
            }
        }
        return false;
    }

    public static StructuredLyrics fromUtaBuildJson(String json) throws Exception {
        JSONObject root = new JSONObject(json);
        String title = root.optString("found_title", root.optString("title", ""));
        String artist = root.optString("found_artist", root.optString("artist", ""));

        if (root.has("lines")) {
            return new StructuredLyrics(title, artist, parseLineArray(root.getJSONArray("lines")));
        }

        JSONArray elements = root.optJSONArray("ruby_annotations");
        if (elements == null) {
            elements = root.optJSONArray("annotations");
        }
        if (elements == null) {
            return new StructuredLyrics(title, artist, Collections.emptyList());
        }
        return new StructuredLyrics(title, artist, parseElementArray(elements));
    }

    private static List<StructuredLyricLine> parseLineArray(JSONArray linesJson) throws Exception {
        List<StructuredLyricLine> lines = new ArrayList<>();
        for (int i = 0; i < linesJson.length(); i++) {
            JSONObject item = linesJson.getJSONObject(i);
            String text = item.optString("text", item.optString("main", ""));
            List<RubyAnnotation> annotations = new ArrayList<>();
            JSONArray ruby = item.optJSONArray("ruby");
            if (ruby == null) {
                ruby = item.optJSONArray("annotations");
            }
            if (ruby != null) {
                for (int j = 0; j < ruby.length(); j++) {
                    JSONObject r = ruby.getJSONObject(j);
                    int start = r.optInt("start", r.optInt("startIndex", -1));
                    int end = r.optInt("end", r.optInt("endIndex", -1));
                    String base = r.optString("base", start >= 0 && end <= text.length() && end > start ? text.substring(start, end) : "");
                    String reading = r.optString("ruby", r.optString("reading", r.optString("rt", "")));
                    RubyAnnotation annotation = new RubyAnnotation(start, end, base, reading);
                    if (annotation.isValid()) {
                        annotations.add(annotation);
                    }
                }
            }
            if (!text.isEmpty()) {
                lines.add(new StructuredLyricLine(text, annotations));
            }
        }
        return lines;
    }

    private static List<StructuredLyricLine> parseElementArray(JSONArray elements) throws Exception {
        List<StructuredLyricLine> lines = new ArrayList<>();
        StringBuilder text = new StringBuilder();
        List<RubyAnnotation> annotations = new ArrayList<>();
        for (int i = 0; i < elements.length(); i++) {
            JSONObject element = elements.getJSONObject(i);
            String type = element.optString("type", "text");
            if ("linebreak".equals(type)) {
                flushLine(lines, text, annotations);
                continue;
            }

            String base = element.optString("base", "");
            if (base.isEmpty()) {
                continue;
            }
            if ("ruby".equals(type)) {
                int start = text.length();
                text.append(base);
                String ruby = element.optString("ruby", "");
                RubyAnnotation annotation = new RubyAnnotation(start, text.length(), base, ruby);
                if (annotation.isValid()) {
                    annotations.add(annotation);
                }
            } else {
                text.append(base);
            }
        }
        flushLine(lines, text, annotations);
        return lines;
    }

    private static void flushLine(List<StructuredLyricLine> lines, StringBuilder text, List<RubyAnnotation> annotations) {
        String line = text.toString();
        if (!line.trim().isEmpty()) {
            lines.add(new StructuredLyricLine(line, annotations));
        }
        text.setLength(0);
        annotations.clear();
    }
}
