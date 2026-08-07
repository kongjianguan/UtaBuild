import { invoke } from './tauri.js';
let currentExportData = null;
export function setExportData(data) {
    currentExportData = data;
}
export function getExportData() {
    return currentExportData;
}
export async function exportLyricsToFile(data) {
    return invoke('export_lyrics_html', {
        title: data.title,
        artist: data.artist,
        lyricsUrl: data.lyricsUrl,
        rubyAnnotations: data.rubyAnnotations,
        coverUrl: data.coverUrl,
    });
}
//# sourceMappingURL=export.js.map