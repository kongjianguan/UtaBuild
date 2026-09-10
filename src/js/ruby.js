import { clampFontSize, defaultFontSizeForViewport, loadSettings, } from './settings.js';
// Preserve the reader's visible line instead of preserving a raw scrollTop.
function captureLyricsScrollAnchor(scrollContainer, lines) {
    if (lines.length === 0)
        return null;
    const scrollTop = scrollContainer.scrollTop;
    const maxScrollTop = Math.max(0, scrollContainer.scrollHeight - scrollContainer.clientHeight);
    const atBottom = maxScrollTop - scrollTop <= 2;
    let lineIndex = lines.findIndex((line) => line.offsetTop + line.offsetHeight > scrollTop + 1);
    if (lineIndex < 0)
        lineIndex = lines.length - 1;
    return {
        lineIndex,
        offsetFromViewportTop: lines[lineIndex].offsetTop - scrollTop,
        atBottom,
    };
}
function restoreLyricsScrollAnchor(scrollContainer, lines, anchor) {
    if (!anchor || lines.length === 0)
        return;
    const maxScrollTop = Math.max(0, scrollContainer.scrollHeight - scrollContainer.clientHeight);
    const line = lines[Math.min(anchor.lineIndex, lines.length - 1)];
    const targetScrollTop = anchor.atBottom
        ? maxScrollTop
        : line.offsetTop - anchor.offsetFromViewportTop;
    scrollContainer.scrollTop = Math.max(0, Math.min(maxScrollTop, targetScrollTop));
}
export function setLyricsFontSize(fontSize) {
    const normalized = clampFontSize(fontSize);
    const scrollContainer = document.querySelector('#lyrics-body');
    const inner = scrollContainer?.querySelector('.lyricBody > div');
    if (!scrollContainer || !inner)
        return normalized;
    const lines = Array.from(inner.children).filter((child) => child instanceof HTMLElement);
    const anchor = captureLyricsScrollAnchor(scrollContainer, lines);
    inner.style.setProperty('--lyric-font-size', `${normalized}px`);
    inner.dataset.fontSize = String(normalized);
    restoreLyricsScrollAnchor(scrollContainer, lines, anchor);
    return normalized;
}
/**
 * Render LyricElement array to DOM with ruby annotations.
 *
 * Data format:
 * { type: "text", base: "文字" }
 * { type: "ruby", base: "漢字", ruby: "ふりがな" }
 * { type: "linebreak" }
 */
export function renderLyrics(elements) {
    const wrapper = document.createElement('div');
    wrapper.className = 'lyricBody';
    const inner = document.createElement('div');
    const settings = loadSettings();
    const fontSize = settings.fontSize ?? defaultFontSizeForViewport();
    inner.style.setProperty('--lyric-font-size', `${fontSize}px`);
    inner.dataset.fontSize = String(fontSize);
    let currentLine = document.createElement('div');
    const arr = elements || [];
    for (const el of arr) {
        switch (el.type) {
            case 'text': {
                currentLine.appendChild(document.createTextNode(el.base ?? ''));
                break;
            }
            case 'ruby': {
                // Native <ruby>/<rt> keeps the annotation out of the text run for
                // screen readers; the classes remain for styling hooks.
                const ruby = document.createElement('ruby');
                ruby.className = 'ruby';
                ruby.appendChild(document.createTextNode(el.base ?? ''));
                const rt = document.createElement('rt');
                rt.className = 'rt';
                rt.textContent = el.ruby ?? '';
                ruby.appendChild(rt);
                currentLine.appendChild(ruby);
                break;
            }
            case 'linebreak': {
                inner.appendChild(currentLine);
                currentLine = document.createElement('div');
                break;
            }
        }
    }
    // Append last line
    if (currentLine.childNodes.length > 0) {
        inner.appendChild(currentLine);
    }
    wrapper.appendChild(inner);
    return wrapper;
}
//# sourceMappingURL=ruby.js.map