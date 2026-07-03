import { invoke } from './tauri';
import type { LyricElement } from './types';

export interface ExportData {
  title: string;
  artist: string | null;
  lyricsUrl: string;
  rubyAnnotations: LyricElement[];
  coverUrl: string | null;
}

function downloadBlob(content: string, filename: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export async function exportLyricsToFile(data: ExportData): Promise<void> {
  try {
    const html = await invoke<string>('export_lyrics_html', {
      title: data.title,
      artist: data.artist,
      lyricsUrl: data.lyricsUrl,
      rubyAnnotations: data.rubyAnnotations,
      coverUrl: data.coverUrl,
    });

    const filename = `${data.artist ?? 'Unknown'} - ${data.title}.html`
      .replace(/[<>:"/\\|?*]/g, '_');
    downloadBlob(html, filename, 'text/html');
  } catch (error) {
    console.error('导出失败:', error);
  }
}
