import type { ExportData, LyricElement } from '../types';
import { invoke, isTauriEnv } from './tauri';

export interface LyricLine {
  elements: LyricElement[];
}

export function splitLyricLines(elements: LyricElement[] | null | undefined): LyricLine[] {
  const lines: LyricLine[] = [];
  let current: LyricElement[] = [];

  for (const el of elements || []) {
    if (el.type === 'linebreak') {
      lines.push({ elements: current });
      current = [];
    } else {
      current.push(el);
    }
  }
  if (current.length > 0) {
    lines.push({ elements: current });
  }
  return lines;
}

export function formatPlainText(elements: LyricElement[] | null | undefined): string {
  return splitLyricLines(elements)
    .map((line) => line.elements.map((el) => el.base ?? '').join(''))
    .join('\n')
    .trimEnd();
}

export function formatRubyBracketText(elements: LyricElement[] | null | undefined): string {
  return splitLyricLines(elements)
    .map((line) =>
      line.elements
        .map((el) => {
          if (el.type === 'ruby' && el.ruby) {
            return `${el.base ?? ''}(${el.ruby})`;
          }
          return el.base ?? '';
        })
        .join(''),
    )
    .join('\n')
    .trimEnd();
}

export function formatStructuredJson(data: ExportData): string {
  const lines = splitLyricLines(data.rubyAnnotations)
    .filter((line) => line.elements.length > 0)
    .map((line) => ({
      elements: line.elements.map((el) => {
        const unit: Record<string, unknown> = { type: el.type };
        if (el.base != null) unit.base = el.base;
        if (el.ruby != null) unit.ruby = el.ruby;
        return unit;
      }),
    }));

  return JSON.stringify(
    {
      status: 'success',
      title: data.title,
      artist: data.artist,
      url: data.lyricsUrl,
      lyrics: { lines },
    },
    null,
    2,
  );
}

export async function copyTextToClipboard(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Fallback below
  }

  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.style.position = 'fixed';
  textarea.style.opacity = '0';
  document.body.appendChild(textarea);
  textarea.select();
  try {
    return document.execCommand('copy');
  } catch {
    return false;
  } finally {
    document.body.removeChild(textarea);
  }
}

function downloadBrowserFile(filename: string, content: string, mimeType: string): void {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

export async function saveExportHtml(data: ExportData): Promise<boolean> {
  return invoke<boolean>('export_lyrics_html', {
    title: data.title,
    artist: data.artist,
    lyricsUrl: data.lyricsUrl,
    rubyAnnotations: data.rubyAnnotations,
    coverUrl: data.coverUrl,
  });
}

export async function saveExportTextFile(
  data: ExportData,
  extension: 'txt' | 'json',
  content: string,
): Promise<boolean> {
  if (isTauriEnv()) {
    return invoke<boolean>('export_text_file', {
      title: data.title,
      artist: data.artist,
      extension,
      content,
    });
  }

  const safeTitle = (data.title || 'lyrics').replace(/[<>:"/\\|?*]/g, '_');
  const safeArtist = (data.artist || 'Unknown').replace(/[<>:"/\\|?*]/g, '_');
  const mime = extension === 'json' ? 'application/json;charset=utf-8' : 'text/plain;charset=utf-8';
  downloadBrowserFile(`${safeArtist} - ${safeTitle}.${extension}`, content, mime);
  return true;
}

export function normalizeCoverUrl(value: unknown): string {
  if (!value || typeof value !== 'string') return '';
  const trimmed = value.trim();
  if (!trimmed) return '';
  try {
    const url = new URL(trimmed, window.location.href);
    return ['http:', 'https:'].includes(url.protocol) ? url.href : '';
  } catch {
    return '';
  }
}

export function resolveSourceMeta(urlOrSource: string): {
  label: string;
  key: 'utaten' | 'qq' | 'ne';
} {
  if (urlOrSource === 'qq_music' || urlOrSource.startsWith('qq:') || urlOrSource.startsWith('qq_music:')) {
    return { label: 'QQ', key: 'qq' };
  }
  if (urlOrSource === 'netease' || urlOrSource.startsWith('ne:')) {
    return { label: 'NE', key: 'ne' };
  }
  return { label: 'UtaTen', key: 'utaten' };
}
