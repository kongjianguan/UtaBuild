package com.utabuild.lsposed.ruby;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public final class StructuredLyricLine {
    public final String text;
    public final List<RubyAnnotation> annotations;

    public StructuredLyricLine(String text, List<RubyAnnotation> annotations) {
        this.text = text == null ? "" : text;
        this.annotations = annotations == null
                ? Collections.emptyList()
                : Collections.unmodifiableList(new ArrayList<>(annotations));
    }

    public boolean hasRuby() {
        for (RubyAnnotation annotation : annotations) {
            if (annotation.isValid()) {
                return true;
            }
        }
        return false;
    }
}
