import { invoke, isTauriEnv } from './tauri';
import type { LyricElement } from './types';

export interface ExportData {
  title: string;
  artist: string | null;
  lyricsUrl: string;
  rubyAnnotations: LyricElement[];
  coverUrl: string | null;
}

let currentExportData: ExportData | null = null;

export function setExportData(data: ExportData): void {
  currentExportData = data;
}

export function getExportData(): ExportData | null {
  return currentExportData;
}

function defaultFilename(data: ExportData): string {
  return `${data.artist ?? 'Unknown'} - ${data.title}.html`
    .replace(/[<>:"/\\|?*]/g, '_');
}

export async function exportLyricsToFile(data: ExportData): Promise<void> {
  if (isTauriEnv()) {
    const dialog = (window as any).__TAURI__?.dialog;
    if (dialog?.save) {
      const path = await dialog.save({
        defaultPath: defaultFilename(data),
        filters: [{ name: 'HTML', extensions: ['html'] }],
      });
      if (!path) return;
      await invoke('export_lyrics_html', {
        title: data.title,
        artist: data.artist,
        lyricsUrl: data.lyricsUrl,
        rubyAnnotations: data.rubyAnnotations,
        coverUrl: data.coverUrl,
        outputPath: path,
      });
      return;
    }
  }
  console.warn('Native save dialog unavailable, export skipped');
  throw new Error('Native save dialog unavailable');
}
