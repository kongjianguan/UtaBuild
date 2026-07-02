import type { HistoryEntry } from './types.js';
import { el } from './dom.js';
import { escapeHtml } from './utils.js';

const SEARCH_HISTORY_KEY = 'utabuild-search-history';
const SEARCH_HISTORY_LIMIT = 300;

// ==================== Normalize & Load/Save ====================

function normalizeHistoryEntry(entry: unknown): HistoryEntry | null {
  if (!entry || typeof (entry as Record<string, unknown>).title !== 'string') {
    return null;
  }

  const e = entry as Record<string, unknown>;
  const title = String(e.title).trim();
  if (!title) return null;

  const artist =
    typeof e.artist === 'string' && e.artist.trim() ? e.artist.trim() : null;
  const searchedAt = Number.isFinite(e.searchedAt) ? Number(e.searchedAt) : Date.now();

  return { title, artist, searchedAt };
}

export function loadSearchHistory(): HistoryEntry[] {
  try {
    const saved = localStorage.getItem(SEARCH_HISTORY_KEY);
    if (!saved) return [];

    const parsed = JSON.parse(saved);
    if (!Array.isArray(parsed)) return [];

    const normalized = parsed
      .map(normalizeHistoryEntry)
      .filter(Boolean) as HistoryEntry[];
    const sorted = normalized
      .sort((a, b) => b.searchedAt - a.searchedAt)
      .slice(0, SEARCH_HISTORY_LIMIT);

    if (sorted.length !== parsed.length) {
      saveSearchHistory(sorted);
    }

    return sorted;
  } catch {
    return [];
  }
}

export function saveSearchHistory(historyItems: HistoryEntry[]): void {
  try {
    const normalized = historyItems
      .map(normalizeHistoryEntry)
      .filter(Boolean)
      .slice(0, SEARCH_HISTORY_LIMIT);
    localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify(normalized));
  } catch (e) {
    console.warn('Failed to save search history:', e);
  }
}

// ==================== Key & Add ====================

function historyKey(title: string, artist: string | null): string {
  return `${title.trim().toLocaleLowerCase()}\u0000${(artist || '').trim().toLocaleLowerCase()}`;
}

export function addSearchHistory(title: string, artist: string | null): void {
  const entry = normalizeHistoryEntry({
    title,
    artist,
    searchedAt: Date.now(),
  });

  if (!entry) return;

  const newKey = historyKey(entry.title, entry.artist);
  const existing = loadSearchHistory().filter(
    (item) => historyKey(item.title, item.artist) !== newKey,
  );
  saveSearchHistory([entry, ...existing].slice(0, SEARCH_HISTORY_LIMIT));
  renderSearchHistory();
}

// ==================== Format & Fill ====================

function formatHistoryTime(timestamp: number): string {
  if (!Number.isFinite(timestamp)) return '';

  try {
    return new Date(timestamp).toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return '';
  }
}

function fillSearchFromHistory(entry: HistoryEntry): void {
  el<HTMLInputElement>('search-title').value = entry.title;
  el<HTMLInputElement>('search-artist').value = entry.artist || '';
  el<HTMLInputElement>('search-title').focus();
}

// ==================== Render ====================

export function renderSearchHistory(): void {
  const list = el<HTMLElement>('search-history-list');
  const empty = el<HTMLElement>('search-history-empty');

  const historyItems = loadSearchHistory();
  list.innerHTML = '';
  empty.classList.toggle('hidden', historyItems.length > 0);

  historyItems.forEach((entry) => {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'history-item';
    button.innerHTML = `
      <span class="history-item__title">${escapeHtml(entry.title)}</span>
      <span class="history-item__meta">
        <span>${escapeHtml(entry.artist || 'アーティスト未指定')}</span>
        <span>${escapeHtml(formatHistoryTime(entry.searchedAt))}</span>
      </span>
    `;
    button.addEventListener('click', () => fillSearchFromHistory(entry));
    list.appendChild(button);
  });
}
