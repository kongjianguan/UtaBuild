package com.utabuild.lsposed.ruby;

public final class RubyAnnotation {
    public final int start;
    public final int end;
    public final String base;
    public final String ruby;

    public RubyAnnotation(int start, int end, String base, String ruby) {
        this.start = start;
        this.end = end;
        this.base = base == null ? "" : base;
        this.ruby = ruby == null ? "" : ruby;
    }

    public boolean isValid() {
        return start >= 0 && end > start && !base.isEmpty() && !ruby.isEmpty();
    }

    public RubyAnnotation shifted(int delta) {
        return new RubyAnnotation(start + delta, end + delta, base, ruby);
    }
}
