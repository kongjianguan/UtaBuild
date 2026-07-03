import { invoke } from './tauri';
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

export async function exportLyricsToFile(data: ExportData): Promise<void> {
  await invoke('export_lyrics_html', {
    title: data.title,
    artist: data.artist,
    lyricsUrl: data.lyricsUrl,
    rubyAnnotations: data.rubyAnnotations,
    coverUrl: data.coverUrl,
  });
}
