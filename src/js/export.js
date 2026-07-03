import { invoke } from './tauri';
let currentExportData = null;
export function setExportData(data) {
    currentExportData = data;
}
export function getExportData() {
    return currentExportData;
}
export async function exportLyricsToFile(data) {
    await invoke('export_lyrics_html', {
        title: data.title,
        artist: data.artist,
        lyricsUrl: data.lyricsUrl,
        rubyAnnotations: data.rubyAnnotations,
        coverUrl: data.coverUrl,
    });
}
//# sourceMappingURL=export.js.map