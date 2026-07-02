import { loadSettings } from './settings.js';
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
    inner.className = settings.fontSize || 'medium';
    let currentLine = document.createElement('div');
    const arr = elements || [];
    for (const el of arr) {
        switch (el.type) {
            case 'text': {
                currentLine.appendChild(document.createTextNode(el.base ?? ''));
                break;
            }
            case 'ruby': {
                const ruby = document.createElement('span');
                ruby.className = 'ruby';
                const rb = document.createElement('span');
                rb.className = 'rb';
                rb.textContent = el.base ?? '';
                const rt = document.createElement('span');
                rt.className = 'rt';
                rt.textContent = el.ruby ?? '';
                ruby.appendChild(rb);
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