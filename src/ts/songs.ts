import type { SavedSong, SearchItem, LyricElement } from './types.js';
import { invoke } from './tauri.js';
import { shouldUseCache, selectedArtworkSource } from './settings.js';
import { el, $$, showLoading, hideLoading, showError, router, updateButtonStates, currentPageScrollY, setBottomMenuAutoHidden } from './dom.js';
import { renderLyrics } from './ruby.js';
import { exportLyricsToFile, type ExportData } from './export.js';

// ==================== State ====================

export let songsSortBy = 'title';
export const hydratingSongMetadataUrls = new Set<string>();
export let activeSongContextMenu: HTMLDivElement | null = null;
export let activeSongContextItem: HTMLElement | null = null;
export let activeResultContextMenu: HTMLDivElement | null = null;
export let activeResultContextItem: HTMLElement | null = null;
export let songLongPressTriggered = false;
export let resultLongPressTriggered = false;
let songLongPressTimer: ReturnType<typeof setTimeout> | null = null;
let resultLongPressTimer: ReturnType<typeof setTimeout> | null = null;


export function setResultLongPressTriggered(val: boolean): void {
  resultLongPressTriggered = val;
}

// ==================== Helpers ====================

function artworkSourceForSong(song: SavedSong): string {
  const url = (song && song.lyrics_url) || '';
  if (url.startsWith('ne:')) return 'netease';
  if (url.startsWith('qq:')) return 'qq';
  return 'utaten';
}

function formatSongSubtitle(song: SavedSong): string {
  const artist = song.artist || 'アーティスト不明';
  return song.album ? `${artist} - ${song.album}` : artist;
}

function normalizeCoverUrl(value: unknown): string {
  if (!value || typeof value !== 'string') return '';
  const trimmed = value.trim();
  if (!trimmed) return '';
  try {
    const url = new URL(trimmed, window.location.href);
    return ['http:', 'https:'].includes(url.protocol) ? url.href : '';
  } catch (_err) {
    return '';
  }
}

function applySongCoverArt(artEl: HTMLElement, coverUrl: unknown): void {
  const normalized = normalizeCoverUrl(coverUrl);
  artEl.classList.toggle('has-cover', Boolean(normalized));
  artEl.style.backgroundImage = normalized ? `url("${normalized}")` : '';
}

// ==================== Build Song Item ====================

export function buildSongItem(song: SavedSong): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'song-item';
  button.dataset.lyricsUrl = song.lyrics_url || '';
  button.draggable = false;

  const art = document.createElement('span');
  art.className = 'song-item__art';
  art.setAttribute('aria-hidden', 'true');
  applySongCoverArt(art, song.cover_url);

  const body = document.createElement('span');
  body.className = 'song-item__body';

  const url = song.lyrics_url || '';
  let sourceLabel: string;
  let sourceClass: string;
  if (url.startsWith('ne:')) {
    sourceLabel = 'NE';
    sourceClass = 'source-badge--ne';
  } else if (url.startsWith('qq:')) {
    sourceLabel = 'QQ';
    sourceClass = 'source-badge--qq';
  } else {
    sourceLabel = 'UtaTen';
    sourceClass = 'source-badge--utaten';
  }

  const badge = document.createElement('span');
  badge.className = `source-badge ${sourceClass}`;
  badge.textContent = sourceLabel;

  const titleEl = document.createElement('span');
  titleEl.className = 'song-item__title';
  titleEl.textContent = song.title || 'タイトル未設定';

  const meta = document.createElement('span');
  meta.className = 'song-item__meta';
  meta.textContent = formatSongSubtitle(song);

  body.append(badge, titleEl, meta);
  button.append(art, body);

  const exportBtn = document.createElement('button');
  exportBtn.className = 'song-item__export';
  exportBtn.setAttribute('aria-label', 'エクスポート');
  exportBtn.textContent = '⤓';
  exportBtn.addEventListener('click', async (e) => {
    e.stopPropagation();
    try {
      const lyricsData = await invoke<any>('get_saved_lyrics', { url: song.lyrics_url });
      const data: ExportData = {
        title: lyricsData.found_title,
        artist: lyricsData.found_artist,
        lyricsUrl: lyricsData.lyrics_url,
        rubyAnnotations: lyricsData.ruby_annotations,
        coverUrl: lyricsData.cover_url ?? null,
      };
      await exportLyricsToFile(data);
    } catch (err) {
      console.error('Export failed:', err);
    }
  });
  button.append(exportBtn);

  return button;
}

// ==================== Update Metadata ====================

export function updateRenderedSongMetadata(
  metadata: Record<string, unknown> | null,
): void {
  if (!metadata?.lyrics_url) return;

  const item = Array.from(el<HTMLElement>('songs-list')?.querySelectorAll('.song-item') || []).find(
    (candidate) =>
      (candidate as HTMLElement).dataset.lyricsUrl === metadata.lyrics_url,
  );
  if (!item) return;

  const art = item.querySelector('.song-item__art') as HTMLElement | null;
  if (art) {
    applySongCoverArt(art, metadata.cover_url);
  }

  if (metadata.album) {
    const meta = item.querySelector('.song-item__meta') as HTMLElement | null;
    if (meta) {
      const artist =
        ((item as unknown as Record<string, unknown>).__songArtist as string) ||
        'アーティスト不明';
      meta.textContent = `${artist} - ${metadata.album}`;
    }
  }
}

// ==================== Hydrate Missing Metadata ====================

export async function hydrateMissingSongMetadata(
  songs: SavedSong[],
): Promise<void> {
  const missing = songs.filter(
    (song) => song.lyrics_url && !normalizeCoverUrl(song.cover_url),
  );

  for (const song of missing) {
    if (hydratingSongMetadataUrls.has(song.lyrics_url)) continue;

    hydratingSongMetadataUrls.add(song.lyrics_url);
    try {
      const metadata = await invoke('hydrate_saved_lyrics_metadata', {
        url: song.lyrics_url,
        artworkSource: artworkSourceForSong(song),
      });
      if (metadata?.status === 'success') {
        updateRenderedSongMetadata(metadata);
      }
    } catch (err) {
      console.warn('Hydrate saved song metadata failed:', song.lyrics_url, err);
    } finally {
      hydratingSongMetadataUrls.delete(song.lyrics_url);
    }
  }
}

// ==================== Song Context Menu ====================

function closeSongContextMenu(): void {
  if (songLongPressTimer) {
    clearTimeout(songLongPressTimer);
    songLongPressTimer = null;
  }

  if (activeSongContextMenu) {
    activeSongContextMenu.remove();
    activeSongContextMenu = null;
  }

  if (activeSongContextItem) {
    activeSongContextItem.classList.remove('is-menu-open');
    activeSongContextItem = null;
  }
}

function positionSongContextMenu(
  menu: HTMLElement,
  clientX: number,
  clientY: number,
): void {
  const margin = 12;
  const rect = menu.getBoundingClientRect();
  const x = Math.min(Math.max(clientX, margin), window.innerWidth - rect.width - margin);
  const y = Math.min(
    Math.max(clientY, margin),
    window.innerHeight - rect.height - margin,
  );
  menu.style.left = `${x}px`;
  menu.style.top = `${y}px`;
}

async function refreshSavedSongArtwork(song: SavedSong): Promise<void> {
  if (!song?.lyrics_url) {
    showError('保存済み歌詞の URL がありません');
    return;
  }

  if (hydratingSongMetadataUrls.has(song.lyrics_url)) {
    showError('ジャケット画像を更新中です...');
    return;
  }

  hydratingSongMetadataUrls.add(song.lyrics_url);
  showLoading();
  try {
    const metadata = await invoke('hydrate_saved_lyrics_metadata', {
      url: song.lyrics_url,
      forceRefresh: true,
      artworkSource: artworkSourceForSong(song),
    });

    if (metadata?.status === 'success') {
      song.cover_url = metadata.cover_url || song.cover_url || '';
      song.album = metadata.album || song.album || '';
      updateRenderedSongMetadata({
        ...metadata,
        cover_url: song.cover_url,
        album: song.album,
      });
      showError(
        song.cover_url
          ? 'ジャケット画像を更新しました'
          : 'UtaTen でジャケット画像が見つかりませんでした',
      );
    } else {
      showError(metadata?.error || 'ジャケット画像の更新に失敗しました');
    }
  } catch (err) {
    console.error('Refresh saved song artwork error:', err);
    showError(`ジャケット画像の更新に失敗しました: ${err}`);
  } finally {
    hydratingSongMetadataUrls.delete(song.lyrics_url);
    hideLoading();
  }
}

async function deleteSavedSong(song: SavedSong): Promise<void> {
  if (!song?.lyrics_url) {
    showError('保存済み歌詞の URL がありません');
    return;
  }

  const label = song.title || 'この曲';
  if (!window.confirm(`「${label}」の保存済み歌詞を削除しますか？`)) return;

  showLoading();
  try {
    await invoke('delete_saved_lyrics', { url: song.lyrics_url });
    showError('保存済み歌詞を削除しました');
    await loadSavedLyrics();
  } catch (err) {
    console.error('Delete saved lyrics error:', err);
    showError(`削除に失敗しました: ${err}`);
  } finally {
    hideLoading();
  }
}

export function showSongContextMenu(
  song: SavedSong,
  trigger: HTMLElement,
  event: PointerEvent | MouseEvent | null,
): void {
  closeSongContextMenu();

  const menu = document.createElement('div');
  menu.className = 'long-press-menu';
  menu.setAttribute('role', 'menu');
  menu.innerHTML = `
    <button class="long-press-menu__item long-press-menu__item--refresh" type="button" role="menuitem" data-song-action="refresh-art">画像を更新</button>
    <button class="long-press-menu__item long-press-menu__item--danger" type="button" role="menuitem" data-song-action="delete">削除</button>
  `;

  menu
    .querySelector('[data-song-action="refresh-art"]')
    ?.addEventListener('click', async () => {
      closeSongContextMenu();
      await refreshSavedSongArtwork(song);
    });

  menu
    .querySelector('[data-song-action="delete"]')
    ?.addEventListener('click', async () => {
      closeSongContextMenu();
      await deleteSavedSong(song);
    });

  document.body.appendChild(menu);
  trigger.classList.add('is-menu-open');
  activeSongContextMenu = menu;
  activeSongContextItem = trigger;

  const rect = trigger.getBoundingClientRect();
  const clientX = event?.clientX ?? rect.right - 18;
  const clientY = event?.clientY ?? rect.top + rect.height / 2;
  positionSongContextMenu(menu, clientX, clientY);

  requestAnimationFrame(() => {
    menu.classList.add('is-visible');
    const menuitem = menu.querySelector('[role="menuitem"]') as HTMLElement | null;
    menuitem?.focus();
  });
}

export function attachSongLongPressMenu(
  button: HTMLButtonElement,
  song: SavedSong,
): void {
  button.addEventListener('pointerdown', (event) => {
    if (event.pointerType === 'mouse' && event.button !== 0) return;

    closeSongContextMenu();
    songLongPressTriggered = false;
    songLongPressTimer = setTimeout(() => {
      songLongPressTriggered = true;
      if (event.pointerType !== 'mouse') {
        try {
          button.setPointerCapture?.(event.pointerId);
        } catch (_) {
          // Pointer may have been released by the time the timeout fires.
        }
      }
      showSongContextMenu(song, button, event);
    }, 560);
  });

  ['pointerup', 'pointercancel', 'pointerleave'].forEach((type) => {
    button.addEventListener(type, () => {
      if (songLongPressTimer) {
        clearTimeout(songLongPressTimer);
        songLongPressTimer = null;
      }
    });
  });

  button.addEventListener('contextmenu', (event) => {
    event.preventDefault();
    songLongPressTriggered = true;
    showSongContextMenu(song, button, event);
  });
}

// ==================== Result Context Menu ====================

export function closeResultContextMenu(): void {
  if (resultLongPressTimer) {
    clearTimeout(resultLongPressTimer);
    resultLongPressTimer = null;
  }

  if (activeResultContextMenu) {
    activeResultContextMenu.remove();
    activeResultContextMenu = null;
  }

  if (activeResultContextItem) {
    activeResultContextItem.classList.remove('is-menu-open');
    activeResultContextItem = null;
  }
}

function positionResultContextMenu(
  menu: HTMLElement,
  clientX: number,
  clientY: number,
): void {
  const margin = 12;
  const rect = menu.getBoundingClientRect();
  const x = Math.min(Math.max(clientX, margin), window.innerWidth - rect.width - margin);
  const y = Math.min(
    Math.max(clientY, margin),
    window.innerHeight - rect.height - margin,
  );
  menu.style.left = `${x}px`;
  menu.style.top = `${y}px`;
}

async function copyResultItemJson(item: SearchItem): Promise<void> {
  showLoading();
  try {
    const result = await invoke<{
      status: string;
      error?: string;
      found_title: string;
      found_artist: string | null;
      lyrics_url: string;
      ruby_annotations: Array<{
        type: string;
        base: string;
        ruby?: string;
      }>;
    }>('get_lyrics', {
      url: item.url,
      title: item.title,
      artist: item.artist || null,
      useCache: shouldUseCache(),
      saveSaltBridge: false,
      artworkSource: selectedArtworkSource(),
      lyricSource: item.source || 'utaten',
    });

    if (result.status !== 'success') {
      showError(result.error || '歌詞の取得に失敗しました');
      return;
    }

    const obj: Record<string, unknown> = {
      status: 'success',
      title: result.found_title,
      artist: result.found_artist,
      url: result.lyrics_url,
      lyrics: { lines: [] as Array<{ elements: unknown[] }> },
    };

    const lines = obj.lyrics as { lines: Array<{ elements: unknown[] }> };
    let currentLine: unknown[] = [];
    for (const el of result.ruby_annotations || []) {
      if (el.type === 'linebreak') {
        if (currentLine.length > 0) {
          lines.lines.push({ elements: currentLine });
          currentLine = [];
        }
      } else {
        const unit: Record<string, unknown> = { type: el.type };
        if (el.base != null) unit.base = el.base;
        if (el.ruby != null) unit.ruby = el.ruby;
        currentLine.push(unit);
      }
    }
    if (currentLine.length > 0) {
      lines.lines.push({ elements: currentLine });
    }

    const json = JSON.stringify(obj, null, 2);
    try {
      await navigator.clipboard.writeText(json);
      showError('JSONをクリップボードにコピーしました');
    } catch (_err) {
      const textarea = document.createElement('textarea');
      textarea.value = json;
      textarea.style.position = 'fixed';
      textarea.style.opacity = '0';
      document.body.appendChild(textarea);
      textarea.select();
      try {
        document.execCommand('copy');
        showError('JSONをクリップボードにコピーしました');
      } catch (_e) {
        showError('クリップボードへのコピーに失敗しました');
      }
      document.body.removeChild(textarea);
    }
  } catch (err) {
    console.error('Copy lyrics JSON error:', err);
    showError(`コピーに失敗しました: ${err}`);
  } finally {
    hideLoading();
  }
}

export function showResultContextMenu(
  item: SearchItem,
  trigger: HTMLElement,
  event: PointerEvent | MouseEvent | null,
): void {
  closeResultContextMenu();

  const menu = document.createElement('div');
  menu.className = 'long-press-menu';
  menu.setAttribute('role', 'menu');
  menu.innerHTML = `
    <button class="long-press-menu__item long-press-menu__item--copy" type="button" role="menuitem" data-result-action="copy-json">复制utaten-cli输出的json到剪切板</button>
  `;

  menu
    .querySelector('[data-result-action="copy-json"]')
    ?.addEventListener('click', async () => {
      closeResultContextMenu();
      await copyResultItemJson(item);
    });

  document.body.appendChild(menu);
  trigger.classList.add('is-menu-open');
  activeResultContextMenu = menu;
  activeResultContextItem = trigger;

  const rect = trigger.getBoundingClientRect();
  const clientX = event?.clientX ?? rect.right - 18;
  const clientY = event?.clientY ?? rect.top + rect.height / 2;
  positionResultContextMenu(menu, clientX, clientY);

  requestAnimationFrame(() => {
    menu.classList.add('is-visible');
    const menuitem = menu.querySelector('[role="menuitem"]') as HTMLElement | null;
    menuitem?.focus();
  });
}

export function attachResultLongPressMenu(
  button: HTMLButtonElement,
  item: SearchItem,
): void {
  button.addEventListener('pointerdown', (event) => {
    if (event.pointerType === 'mouse' && event.button !== 0) return;

    closeResultContextMenu();
    resultLongPressTriggered = false;
    resultLongPressTimer = setTimeout(() => {
      resultLongPressTriggered = true;
      if (event.pointerType !== 'mouse') {
        try {
          button.setPointerCapture?.(event.pointerId);
        } catch (_) {
          // Pointer may have been released by the time the timeout fires.
        }
      }
      showResultContextMenu(item, button, event);
    }, 560);
  });

  ['pointerup', 'pointercancel', 'pointerleave'].forEach((type) => {
    button.addEventListener(type, () => {
      if (resultLongPressTimer) {
        clearTimeout(resultLongPressTimer);
        resultLongPressTimer = null;
      }
    });
  });

  button.addEventListener('contextmenu', (event) => {
    event.preventDefault();
    resultLongPressTriggered = true;
    showResultContextMenu(item, button, event);
  });
}

// ==================== Render & Load Saved Lyrics ====================

export function renderSavedLyrics(songs: SavedSong[]): void {
  if (!el<HTMLElement>('songs-list') || !el<HTMLElement>('songs-empty')) return;

  closeSongContextMenu();
  el<HTMLElement>('songs-list').innerHTML = '';
  el<HTMLElement>('songs-empty').classList.toggle('hidden', songs.length > 0);

  songs.forEach((song) => {
    const button = buildSongItem(song);
    (button as unknown as Record<string, unknown>).__songArtist =
      song.artist || 'アーティスト不明';
    attachSongLongPressMenu(button, song);
    button.addEventListener('click', () => {
      if (songLongPressTriggered) {
        songLongPressTriggered = false;
        return;
      }
      void openSavedLyrics(song.lyrics_url);
    });
    el<HTMLElement>('songs-list').appendChild(button);
  });

  void hydrateMissingSongMetadata(songs);
}

export async function loadSavedLyrics(): Promise<void> {
  if (!el<HTMLElement>('songs-list') || !el<HTMLElement>('songs-empty')) return;

  el<HTMLElement>('songs-list').innerHTML = '';
  el<HTMLElement>('songs-empty').textContent = '保存済み歌詞を読み込み中です...';
  el<HTMLElement>('songs-empty').classList.remove('hidden');

  try {
    const result = await invoke<{ songs?: SavedSong[] }>('list_saved_lyrics', {
      sortBy: songsSortBy,
    });
    const songs = Array.isArray(result?.songs) ? result.songs : [];
    el<HTMLElement>('songs-empty').textContent =
      '保存済みの歌詞はまだありません。搜索并打开歌词后会永久保存到这里。';
    renderSavedLyrics(songs);
  } catch (err) {
    console.error('Load saved lyrics error:', err);
    el<HTMLElement>('songs-empty').textContent = `保存済み歌詞の読み込みに失敗しました: ${err}`;
  }
}

export async function openSavedLyrics(url: string): Promise<void> {
  if (!url) {
    showError('保存済み歌詞の URL がありません');
    return;
  }

  showLoading();
  try {
    const result = await invoke<{
      status: string;
      error?: string;
      found_title: string;
      found_artist: string | null;
      ruby_annotations: Array<{ type: string; base?: string; ruby?: string }>;
    }>('get_saved_lyrics', { url });

    if (result.status !== 'success') {
      showError(result.error || '保存済み歌詞の読み込みに失敗しました');
      return;
    }

    el<HTMLElement>('lyrics-title').textContent = result.found_title;
    el<HTMLElement>('lyrics-artist').textContent = result.found_artist;
    el<HTMLElement>('lyrics-body').innerHTML = '';
    el<HTMLElement>('lyrics-body').appendChild(renderLyrics(result.ruby_annotations as LyricElement[]));
    updateButtonStates();
    router.navigate('lyrics', { resetScroll: true });

    (window as any).__currentLyricsExportData = {
      title: result.found_title,
      artist: result.found_artist,
      lyricsUrl: url,
      rubyAnnotations: result.ruby_annotations as LyricElement[],
      coverUrl: (result as any).cover_url ?? null,
    } satisfies ExportData;
  } catch (err) {
    console.error('Open saved lyrics error:', err);
    showError(`保存済み歌詞の読み込みに失敗しました: ${err}`);
  } finally {
    hideLoading();
  }
}

// ==================== Songs Dock Auto-Hide ====================

export let lastSongsScrollY = 0;

export function handleSongsDockAutoHide(): void {
  setBottomMenuAutoHidden(false);
}

export function initSongsDockAutoHide(): void {
  lastSongsScrollY = currentPageScrollY();
}

// ==================== Init Songs Controls ====================

export function initSongsControls(): void {
  $$('[data-song-sort]').forEach((button) => {
    const btn = button as HTMLElement;
    btn.addEventListener('click', () => {
      songsSortBy = btn.dataset.songSort === 'artist' ? 'artist' : 'title';
      $$('[data-song-sort]').forEach((item) => {
        const it = item as HTMLElement;
        const isActive = it.dataset.songSort === songsSortBy;
        it.classList.toggle('active', isActive);
        it.setAttribute('aria-pressed', String(isActive));
      });
      void loadSavedLyrics();
    });
  });

  document.addEventListener('click', (event) => {
    if (
      activeSongContextMenu &&
      !activeSongContextMenu.contains(event.target as Node)
    ) {
      closeSongContextMenu();
    }
    if (
      activeResultContextMenu &&
      !activeResultContextMenu.contains(event.target as Node)
    ) {
      closeResultContextMenu();
    }
  });
  document.addEventListener('keydown', (event) => {
    if (activeSongContextMenu) {
      if (event.key === 'Escape') {
        closeSongContextMenu();
      } else if (['ArrowDown', 'ArrowUp'].includes(event.key)) {
        event.preventDefault();
        const items = Array.from(
          activeSongContextMenu.querySelectorAll('[role="menuitem"]'),
        );
        const currentIndex = Math.max(
          0,
          items.indexOf(document.activeElement as HTMLElement),
        );
        const nextIndex =
          event.key === 'ArrowDown'
            ? (currentIndex + 1) % items.length
            : (currentIndex - 1 + items.length) % items.length;
        (items[nextIndex] as HTMLElement)?.focus();
      }
    } else if (activeResultContextMenu) {
      if (event.key === 'Escape') {
        closeResultContextMenu();
      }
    }
  });
  window.addEventListener(
    'scroll',
    () => {
      closeSongContextMenu();
      closeResultContextMenu();
    },
    { passive: true },
  );
  window.addEventListener('resize', () => {
    closeSongContextMenu();
    closeResultContextMenu();
  });
}
