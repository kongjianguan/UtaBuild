import { computed, reactive, ref } from 'vue';
import type {
  ExportData,
  HistoryEntry,
  LyricsResponse,
  LyricSource,
  SavedSong,
  SearchData,
  SearchItem,
  SearchResponse,
  UtatenPageState,
} from '../types';
import { invoke, tauriReady } from '../services/tauri';
import { normalizeCoverUrl } from '../services/formatters';
import {
  effectiveArtworkSource,
  effectiveUseCache,
  useSettings,
} from './useSettings';
import { confirmDialog, showError, showInfo, showSuccess } from './useFeedback';
import { navigateTo, useRouter } from './useRouter';

const SEARCH_HISTORY_KEY = 'utabuild-search-history';
const SEARCH_HISTORY_LIMIT = 300;

const SEARCH_SOURCES: Array<{ key: LyricSource; label: string }> = [
  { key: 'utaten', label: 'UtaTen' },
  { key: 'qq_music', label: 'QQ音楽' },
  { key: 'netease', label: 'NetEase' },
];

// ==================== Search History ====================

function normalizeHistoryEntry(entry: unknown): HistoryEntry | null {
  if (!entry || typeof (entry as Record<string, unknown>).title !== 'string') {
    return null;
  }
  const e = entry as Record<string, unknown>;
  const title = String(e.title).trim();
  if (!title) return null;

  const artist = typeof e.artist === 'string' && e.artist.trim() ? e.artist.trim() : null;
  const searchedAt = Number.isFinite(e.searchedAt) ? Number(e.searchedAt) : Date.now();
  return { title, artist, searchedAt };
}

function loadStoredHistory(): HistoryEntry[] {
  try {
    const saved = localStorage.getItem(SEARCH_HISTORY_KEY);
    if (!saved) return [];
    const parsed = JSON.parse(saved);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map(normalizeHistoryEntry)
      .filter((x): x is HistoryEntry => Boolean(x))
      .sort((a, b) => b.searchedAt - a.searchedAt)
      .slice(0, SEARCH_HISTORY_LIMIT);
  } catch {
    return [];
  }
}

function persistHistory(items: HistoryEntry[]): void {
  try {
    localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify(items.slice(0, SEARCH_HISTORY_LIMIT)));
  } catch (err) {
    console.warn('Failed to save search history:', err);
  }
}

function historyKey(title: string, artist: string | null): string {
  return `${title.trim().toLocaleLowerCase()}\u0000${(artist || '').trim().toLocaleLowerCase()}`;
}

// ==================== Reactive App State ====================

const searchTitle = ref('');
const searchArtist = ref('');
const searchHistory = ref<HistoryEntry[]>(loadStoredHistory());

const isSearching = ref(false);
const searchData = ref<SearchData | null>(null);
const activeSourceTab = ref<'all' | LyricSource>('all');
const isLoadingMoreResults = ref(false);
const utatenPageState = reactive<UtatenPageState>({
  currentPage: 1,
  totalPages: 1,
  hasNext: false,
  loadedPages: 1,
  loadingMore: false,
});

const pendingSaltRequest = ref<{ title?: string; artist?: string | null } | null>(null);

const currentLyrics = ref<LyricsResponse | null>(null);
const isLoadingLyrics = ref(false);
const exportTarget = ref<ExportData | null>(null);

const savedSongs = ref<SavedSong[]>([]);
const songsSortBy = ref<'title' | 'artist'>('title');
const songsFilterQuery = ref('');
const isLoadingSongs = ref(false);
const hydratingUrls = reactive(new Set<string>());

const lspLogs = ref('');
const isLoadingLspLogs = ref(false);
const lspLogZoom = ref(1);

let searchRunId = 0;
let lyricsRequestId = 0;
let savedLyricsLoadId = 0;
let metadataHydrationRunId = 0;
let metadataHydrationRequestId = 0;
const metadataHydrationRequests = new Map<string, { requestId: number; runId: number }>();
let lspLogRequestId = 0;

// ==================== LSP Actions ====================

export async function setBackendLspLogging(enabled: boolean): Promise<void> {
  try {
    await tauriReady;
    await invoke('set_lsp_logging_enabled', { enabled });
  } catch (err) {
    console.warn('Failed to sync lsp logging setting:', err);
  }
}

export async function syncLspSettings(): Promise<void> {
  const { settings } = useSettings();
  const payload = {
    lspLogEnabled: settings.lspLogEnabled === true,
    showProofPopup: settings.showProofPopup !== false,
    autoLaunchUtaBuild: settings.autoLaunchUtaBuild !== false,
  };
  try {
    await tauriReady;
    await invoke('set_lsp_settings', { settings: payload });
  } catch (err) {
    console.warn('Failed to sync lsp settings:', err);
  }
}

export async function appendAppLspLog(scope: string, message: string): Promise<void> {
  const { settings } = useSettings();
  if (settings.lspLogEnabled !== true) return;
  try {
    await tauriReady;
    await invoke('append_lsp_log', { scope, message });
  } catch (err) {
    console.warn('Failed to append lsp log:', err);
  }
}

export async function fetchLspLogs(): Promise<void> {
  const requestId = ++lspLogRequestId;
  isLoadingLspLogs.value = true;
  lspLogs.value = 'LSP ログを読み込み中です...';
  void appendAppLspLog('settings', 'view lsp logs');

  try {
    await tauriReady;
    const logs = await invoke<string>('get_lsp_logs');
    if (requestId !== lspLogRequestId) return;
    lspLogs.value = logs && String(logs).trim() ? String(logs) : 'LSP ログはまだありません';
  } catch (err) {
    if (requestId !== lspLogRequestId) return;
    lspLogs.value = `LSP ログの読み込みに失敗しました: ${err}`;
  } finally {
    if (requestId === lspLogRequestId) {
      isLoadingLspLogs.value = false;
    }
  }
}

export function adjustLspLogZoom(delta: number): void {
  lspLogZoom.value = Math.min(1.8, Math.max(0.7, Number((lspLogZoom.value + delta).toFixed(2))));
}

// ==================== Salt Player Launch ====================

export async function checkSaltLaunchRequest(): Promise<void> {
  try {
    await tauriReady;
    const request = await invoke<{ title?: string; artist?: string | null } | null>(
      'take_salt_launch_request',
    );
    if (!request?.title) return;

    pendingSaltRequest.value = request;
    searchTitle.value = request.title;
    searchArtist.value = request.artist || '';
    navigateTo('search', { replace: true, resetScroll: true });
    void appendAppLspLog(
      'salt',
      `launch request received title="${request.title}" artist="${request.artist || ''}"`,
    );
    showInfo(
      `Salt Player から「${request.title}」を受け取りました。検索して候補を選ぶと Ruby 表示を紐付けます。`,
    );
  } catch (err) {
    console.warn('Salt launch request check failed:', err);
  }
}

// ==================== History Actions ====================

export function addSearchHistory(title: string, artist: string | null): void {
  const entry = normalizeHistoryEntry({ title, artist, searchedAt: Date.now() });
  if (!entry) return;
  const key = historyKey(entry.title, entry.artist);
  const next = [
    entry,
    ...searchHistory.value.filter((item) => historyKey(item.title, item.artist) !== key),
  ].slice(0, SEARCH_HISTORY_LIMIT);
  searchHistory.value = next;
  persistHistory(next);
}

export function removeHistoryEntry(entry: HistoryEntry): void {
  const key = historyKey(entry.title, entry.artist);
  const next = searchHistory.value.filter((item) => historyKey(item.title, item.artist) !== key);
  searchHistory.value = next;
  persistHistory(next);
}

export async function clearAllSearchHistory(): Promise<void> {
  if (searchHistory.value.length === 0) return;
  const ok = await confirmDialog({
    title: '検索履歴を削除',
    message: '保存されている検索履歴をすべて削除しますか？',
    confirmLabel: 'すべて削除',
    danger: true,
  });
  if (!ok) return;
  searchHistory.value = [];
  persistHistory([]);
  showSuccess('検索履歴を削除しました');
}

// ==================== Search Actions ====================

export async function submitSearch(
  overrideTitle?: string,
  overrideArtist?: string | null,
): Promise<void> {
  const title = (overrideTitle ?? searchTitle.value).trim();
  const artistRaw = overrideArtist !== undefined ? overrideArtist : searchArtist.value;
  const artist = artistRaw ? artistRaw.trim() || null : null;

  if (!title) {
    showError('曲名を入力してください');
    return;
  }

  searchTitle.value = title;
  searchArtist.value = artist || '';

  const runId = ++searchRunId;
  lyricsRequestId += 1;
  isLoadingMoreResults.value = false;
  activeSourceTab.value = 'all';

  Object.assign(utatenPageState, {
    currentPage: 1,
    totalPages: 1,
    hasNext: false,
    loadedPages: 1,
    loadingMore: false,
  });

  addSearchHistory(title, artist);
  void appendAppLspLog('ui', `search requested title="${title}" artist="${artist || ''}"`);

  searchData.value = {
    query: { title, artist },
    sources: {
      utaten: { data: null, error: null, loading: true },
      qq_music: { data: null, error: null, loading: true },
      netease: { data: null, error: null, loading: true },
    },
    allResults: [],
  };

  isSearching.value = true;
  navigateTo('results', { resetScroll: true });

  try {
    await Promise.all(
      SEARCH_SOURCES.map(async ({ key, label }) => {
        try {
          const result = await invoke<SearchResponse>('search_lyrics', {
            title,
            artist,
            page: 1,
            useCache: effectiveUseCache(),
            lyricSource: key,
          });

          if (runId !== searchRunId || !searchData.value) return;

          const hasResults =
            result?.status === 'select' &&
            Array.isArray(result.results) &&
            result.results.length > 0;
          const items = hasResults
            ? result.results.map((item) => ({ ...item, source: key }))
            : [];

          const existingUrls = new Set(searchData.value.allResults.map((r) => r.url));
          const deduped = items.filter((item) => !existingUrls.has(item.url));

          searchData.value = {
            ...searchData.value,
            sources: {
              ...searchData.value.sources,
              [key]: {
                data: hasResults ? result : null,
                error: hasResults ? null : result?.error || `${label} で結果が見つかりませんでした`,
                loading: false,
              },
            },
            allResults: [...searchData.value.allResults, ...deduped],
          };

          if (key === 'utaten') {
            Object.assign(utatenPageState, {
              currentPage: result?.pagination?.current_page || 1,
              totalPages: result?.pagination?.total_pages || 1,
              hasNext: result?.pagination?.has_next ?? false,
              loadedPages: 1,
              loadingMore: false,
            });
          }
        } catch (err) {
          if (runId !== searchRunId || !searchData.value) return;
          const reason = err as { message?: string } | null;
          searchData.value = {
            ...searchData.value,
            sources: {
              ...searchData.value.sources,
              [key]: {
                data: null,
                error: reason?.message || String(err) || `${label} の検索に失敗しました`,
                loading: false,
              },
            },
          };
        }
      }),
    );

    if (runId !== searchRunId || !searchData.value) return;

    if (searchData.value.allResults.length > 0) {
      const counts = SEARCH_SOURCES.map(
        ({ key }) => searchData.value?.allResults.filter((i) => i.source === key).length || 0,
      );
      void appendAppLspLog(
        'search',
        `search success utaten=${counts[0]} qq=${counts[1]} ne=${counts[2]}`,
      );
    } else {
      const realErrors = SEARCH_SOURCES.map(({ key }) => searchData.value?.sources[key].error)
        .filter((e): e is string => typeof e === 'string' && e.length > 0 && !e.includes('見つかりませんでした'));
      if (realErrors.length > 0) {
        void appendAppLspLog('search', `search failed: ${realErrors[0]}`);
        showError(realErrors[0]);
      } else {
        void appendAppLspLog('search', 'search not_found');
      }
    }
  } catch (err) {
    void appendAppLspLog('search', `search error ${String(err)}`);
    showError(`検索エラー: ${err}`);
  } finally {
    if (runId === searchRunId) {
      isSearching.value = false;
    }
  }
}

export async function loadNextUtatenPage(): Promise<void> {
  const data = searchData.value;
  if (!data || isLoadingMoreResults.value) return;
  if (!utatenPageState.hasNext || utatenPageState.currentPage >= utatenPageState.totalPages) return;

  const runId = searchRunId;
  const nextPage = utatenPageState.currentPage + 1;
  isLoadingMoreResults.value = true;
  utatenPageState.loadingMore = true;

  try {
    const nextResult = await invoke<SearchResponse>('search_lyrics', {
      title: data.query.title,
      artist: data.query.artist,
      page: nextPage,
      useCache: effectiveUseCache(),
      lyricSource: 'utaten',
    });

    if (runId !== searchRunId || !searchData.value) return;
    if (nextResult.status !== 'select' || !Array.isArray(nextResult.results)) {
      throw new Error(nextResult.error || `ページ ${nextPage} の取得に失敗しました`);
    }

    const existingUrls = new Set(searchData.value.allResults.map((r) => r.url));
    const newItems = nextResult.results
      .map((item) => ({ ...item, source: 'utaten' as const }))
      .filter((item) => !existingUrls.has(item.url));

    searchData.value = {
      ...searchData.value,
      sources: {
        ...searchData.value.sources,
        utaten: { data: nextResult, error: null, loading: false },
      },
      allResults: [...searchData.value.allResults, ...newItems],
    };

    Object.assign(utatenPageState, {
      currentPage: nextPage,
      totalPages: nextResult.pagination?.total_pages ?? utatenPageState.totalPages,
      hasNext: nextResult.pagination?.has_next ?? false,
      loadedPages: nextPage,
      loadingMore: false,
    });
  } catch (err) {
    if (runId === searchRunId) {
      utatenPageState.loadingMore = false;
      utatenPageState.hasNext = false;
      console.error('Load more search results error:', err);
    }
  } finally {
    isLoadingMoreResults.value = false;
  }
}

export async function fetchLyricsForItem(
  item: SearchItem,
  options: { saveSaltBridge?: boolean } = {},
): Promise<LyricsResponse | null> {
  const result = await invoke<LyricsResponse>('get_lyrics', {
    url: item.url,
    title: item.title,
    artist: item.artist || null,
    useCache: effectiveUseCache(),
    saveSaltBridge: options.saveSaltBridge ?? true,
    artworkSource: effectiveArtworkSource(),
    lyricSource: item.source || 'utaten',
  });

  if (result.status !== 'success' && (item.source === 'qq_music' || item.source === 'netease')) {
    const utatenItems = (searchData.value?.allResults || []).filter((r) => r.source === 'utaten');
    const match = utatenItems.find(
      (r) => r.title === item.title && r.artist === item.artist,
    );
    if (match) {
      return invoke<LyricsResponse>('get_lyrics', {
        url: match.url,
        title: match.title,
        artist: match.artist || null,
        useCache: effectiveUseCache(),
        saveSaltBridge: options.saveSaltBridge ?? true,
        artworkSource: effectiveArtworkSource(),
        lyricSource: 'utaten',
      });
    }
  }

  return result;
}

export async function selectSearchResult(item: SearchItem): Promise<void> {
  const { routerState } = useRouter();
  const requestId = ++lyricsRequestId;
  const saltRequest = pendingSaltRequest.value;

  if (saltRequest) {
    const confirmed = await confirmDialog({
      title: 'Salt Player と紐付け',
      message: `Salt Player の「${saltRequest.title || ''}」に UtaBuild の「${item.title}」を紐付け、今後 Ruby 表示に使用しますか？`,
      confirmLabel: '紐付ける',
    });
    if (!confirmed) {
      void appendAppLspLog('salt', `binding cancelled salt="${saltRequest.title || ''}"`);
      pendingSaltRequest.value = null;
      return;
    }
  }

  // Navigate immediately and show skeleton state in the lyrics page for native feel
  currentLyrics.value = {
    status: 'loading',
    found_title: item.title,
    found_artist: item.artist,
    lyrics_url: item.url,
    ruby_annotations: [],
  };
  isLoadingLyrics.value = true;
  navigateTo('lyrics', { resetScroll: true });

  try {
    const result = await fetchLyricsForItem(item, { saveSaltBridge: !saltRequest });

    if (requestId !== lyricsRequestId || routerState.current !== 'lyrics') return;

    if (result && result.status === 'success') {
      if (saltRequest) {
        await invoke('bind_salt_song_lyrics', {
          saltTitle: saltRequest.title || item.title,
          saltArtist: saltRequest.artist || null,
          lyrics: result,
        });
        pendingSaltRequest.value = null;
        void appendAppLspLog(
          'salt',
          `binding saved salt="${saltRequest.title || ''}" selected="${item.title}"`,
        );
      }
      if (requestId !== lyricsRequestId || routerState.current !== 'lyrics') return;

      currentLyrics.value = result;
      exportTarget.value = {
        title: result.found_title,
        artist: result.found_artist,
        lyricsUrl: result.lyrics_url,
        rubyAnnotations: result.ruby_annotations,
        coverUrl: result.cover_url ?? null,
      };
    } else {
      void appendAppLspLog('lyrics', `get lyrics failed selected="${item.title}"`);
      showError(result?.error || '歌詞の取得に失敗しました');
      currentLyrics.value = null;
    }
  } catch (err) {
    if (requestId !== lyricsRequestId) return;
    void appendAppLspLog('lyrics', `select error ${String(err)}`);
    showError(`エラー: ${err}`);
    currentLyrics.value = null;
  } finally {
    if (requestId === lyricsRequestId) {
      isLoadingLyrics.value = false;
    }
  }
}

export async function openExportFromResult(item: SearchItem): Promise<void> {
  try {
    const result = await fetchLyricsForItem(item, { saveSaltBridge: false });
    if (!result || result.status !== 'success') {
      showError(result?.error || '歌詞の取得に失敗しました');
      return;
    }
    exportTarget.value = {
      title: result.found_title,
      artist: result.found_artist,
      lyricsUrl: result.lyrics_url,
      rubyAnnotations: result.ruby_annotations,
      coverUrl: result.cover_url ?? null,
    };
    navigateTo('export', { resetScroll: true });
  } catch (err) {
    showError(`取得に失敗しました: ${err}`);
  }
}

// ==================== Saved Songs Actions ====================

function artworkSourceForSong(song: SavedSong, preferredSource?: string): string {
  const url = song.lyrics_url || '';
  if (url.startsWith('ne:')) return 'netease';
  if (url.startsWith('qq:')) return 'qq';
  return preferredSource ?? effectiveArtworkSource();
}

async function hydrateMissingSongMetadata(
  songs: SavedSong[],
  options: { refreshArtwork?: boolean; runId?: number } = {},
): Promise<void> {
  const refreshArtwork = options.refreshArtwork === true;
  const runId = options.runId ?? ++metadataHydrationRunId;
  const artworkSource = effectiveArtworkSource();

  const candidates = songs.filter(
    (song) => song.lyrics_url && (refreshArtwork || !normalizeCoverUrl(song.cover_url)),
  );

  for (const song of candidates) {
    if (runId !== metadataHydrationRunId) return;
    const activeReq = metadataHydrationRequests.get(song.lyrics_url);
    if (!refreshArtwork && activeReq?.runId === runId) continue;

    const reqId = ++metadataHydrationRequestId;
    metadataHydrationRequests.set(song.lyrics_url, { requestId: reqId, runId });
    hydratingUrls.add(song.lyrics_url);

    try {
      const metadata = await invoke<{
        status?: string;
        lyrics_url?: string;
        album?: string;
        cover_url?: string;
      }>('hydrate_saved_lyrics_metadata', {
        url: song.lyrics_url,
        forceRefresh: refreshArtwork,
        artworkSource: artworkSourceForSong(song, artworkSource),
      });

      if (
        metadata?.status === 'success' &&
        metadataHydrationRunId === runId &&
        metadataHydrationRequests.get(song.lyrics_url)?.requestId === reqId
      ) {
        savedSongs.value = savedSongs.value.map((s) =>
          s.lyrics_url === song.lyrics_url
            ? {
                ...s,
                cover_url: metadata.cover_url || s.cover_url,
                album: metadata.album || s.album,
              }
            : s,
        );
      }
    } catch (err) {
      console.warn('Hydrate metadata failed:', song.lyrics_url, err);
    } finally {
      if (metadataHydrationRequests.get(song.lyrics_url)?.requestId === reqId) {
        metadataHydrationRequests.delete(song.lyrics_url);
        hydratingUrls.delete(song.lyrics_url);
      }
    }
  }
}

export async function loadSavedLyrics(options: { refreshArtwork?: boolean } = {}): Promise<void> {
  const loadId = ++savedLyricsLoadId;
  const runId = ++metadataHydrationRunId;
  isLoadingSongs.value = true;

  try {
    const result = await invoke<{ songs?: SavedSong[] }>('list_saved_lyrics', {
      sortBy: songsSortBy.value,
    });
    if (loadId !== savedLyricsLoadId) return;
    const list = Array.isArray(result?.songs) ? result.songs : [];
    savedSongs.value = list;
    void hydrateMissingSongMetadata(list, {
      refreshArtwork: options.refreshArtwork === true,
      runId,
    });
  } catch (err) {
    if (loadId !== savedLyricsLoadId) return;
    showError(`保存済み歌詞の読み込みに失敗しました: ${err}`);
  } finally {
    if (loadId === savedLyricsLoadId) {
      isLoadingSongs.value = false;
    }
  }
}

export async function openSavedLyrics(song: SavedSong): Promise<void> {
  if (!song.lyrics_url) {
    showError('保存済み歌詞の URL がありません');
    return;
  }

  const { routerState } = useRouter();
  const requestId = ++lyricsRequestId;

  currentLyrics.value = {
    status: 'loading',
    found_title: song.title,
    found_artist: song.artist,
    cover_url: song.cover_url,
    lyrics_url: song.lyrics_url,
    ruby_annotations: [],
  };
  isLoadingLyrics.value = true;
  navigateTo('lyrics', { resetScroll: true });

  try {
    const result = await invoke<LyricsResponse>('get_saved_lyrics', { url: song.lyrics_url });
    if (requestId !== lyricsRequestId || routerState.current !== 'lyrics') return;

    if (result.status !== 'success') {
      showError(result.error || '保存済み歌詞の読み込みに失敗しました');
      currentLyrics.value = null;
      return;
    }

    currentLyrics.value = result;
    exportTarget.value = {
      title: result.found_title,
      artist: result.found_artist,
      lyricsUrl: song.lyrics_url,
      rubyAnnotations: result.ruby_annotations,
      coverUrl: result.cover_url ?? song.cover_url ?? null,
    };
  } catch (err) {
    if (requestId !== lyricsRequestId) return;
    showError(`保存済み歌詞の読み込みに失敗しました: ${err}`);
    currentLyrics.value = null;
  } finally {
    if (requestId === lyricsRequestId) {
      isLoadingLyrics.value = false;
    }
  }
}

export async function openExportFromSavedSong(song: SavedSong): Promise<void> {
  try {
    const result = await invoke<LyricsResponse>('get_saved_lyrics', { url: song.lyrics_url });
    if (result.status !== 'success') {
      throw new Error(result.error || '保存済み歌詞の読み込みに失敗しました');
    }
    exportTarget.value = {
      title: result.found_title,
      artist: result.found_artist,
      lyricsUrl: result.lyrics_url,
      rubyAnnotations: result.ruby_annotations,
      coverUrl: result.cover_url ?? song.cover_url ?? null,
    };
    navigateTo('export', { resetScroll: true });
  } catch (err) {
    showError(`エクスポートの準備に失敗しました: ${err}`);
  }
}

export function openExportForCurrentLyrics(): void {
  const lyrics = currentLyrics.value;
  if (!lyrics || lyrics.status !== 'success') return;

  exportTarget.value = {
    title: lyrics.found_title,
    artist: lyrics.found_artist,
    lyricsUrl: lyrics.lyrics_url,
    rubyAnnotations: lyrics.ruby_annotations,
    coverUrl: lyrics.cover_url ?? null,
  };
  navigateTo('export', { resetScroll: true });
}

export async function refreshSavedSongArtwork(song: SavedSong): Promise<void> {
  if (!song.lyrics_url) {
    showError('保存済み歌詞の URL がありません');
    return;
  }
  if (hydratingUrls.has(song.lyrics_url)) {
    showInfo('ジャケット画像を更新中です...');
    return;
  }

  hydratingUrls.add(song.lyrics_url);
  const reqId = ++metadataHydrationRequestId;
  metadataHydrationRequests.set(song.lyrics_url, {
    requestId: reqId,
    runId: metadataHydrationRunId,
  });

  try {
    const metadata = await invoke<{
      status?: string;
      cover_url?: string;
      album?: string;
      error?: string;
    }>('hydrate_saved_lyrics_metadata', {
      url: song.lyrics_url,
      forceRefresh: true,
      artworkSource: artworkSourceForSong(song),
    });

    const isCurrent = metadataHydrationRequests.get(song.lyrics_url)?.requestId === reqId;
    if (metadata?.status === 'success' && isCurrent) {
      const nextCover = metadata.cover_url || song.cover_url || '';
      const nextAlbum = metadata.album || song.album || '';
      savedSongs.value = savedSongs.value.map((s) =>
        s.lyrics_url === song.lyrics_url
          ? { ...s, cover_url: nextCover, album: nextAlbum }
          : s,
      );
      showSuccess(nextCover ? 'ジャケット画像を更新しました' : 'ジャケット画像が見つかりませんでした');
    } else if (metadata?.status !== 'success') {
      showError(metadata?.error || 'ジャケット画像の更新に失敗しました');
    }
  } catch (err) {
    showError(`ジャケット画像の更新に失敗しました: ${err}`);
  } finally {
    if (metadataHydrationRequests.get(song.lyrics_url)?.requestId === reqId) {
      metadataHydrationRequests.delete(song.lyrics_url);
      hydratingUrls.delete(song.lyrics_url);
    }
  }
}

export async function deleteSavedSong(song: SavedSong): Promise<void> {
  if (!song.lyrics_url) return;
  const ok = await confirmDialog({
    title: '保存済み歌詞を削除',
    message: `「${song.title || 'この曲'}」の保存済み歌詞を削除しますか？`,
    confirmLabel: '削除',
    danger: true,
  });
  if (!ok) return;

  try {
    await invoke('delete_saved_lyrics', { url: song.lyrics_url });
    savedSongs.value = savedSongs.value.filter((s) => s.lyrics_url !== song.lyrics_url);
    showSuccess('保存済み歌詞を削除しました');
  } catch (err) {
    showError(`削除に失敗しました: ${err}`);
  }
}

export async function clearAllCaches(): Promise<void> {
  const ok = await confirmDialog({
    title: 'キャッシュを削除',
    message: '検索結果キャッシュとメモリキャッシュを削除します。保存済みの曲は削除されません。',
    confirmLabel: '削除',
  });
  if (!ok) return;

  try {
    await invoke('clear_cache');
    showSuccess('検索キャッシュを削除しました。保存済みの曲は削除されていません');
  } catch (err) {
    showError(`キャッシュの削除に失敗しました: ${err}`, {
      label: '再試行',
      onClick: () => void clearAllCaches(),
    });
  }
}

// ==================== Computed Helpers ====================

export function useLyricsApp() {
  const filteredSongs = computed(() => {
    const q = songsFilterQuery.value.trim().toLocaleLowerCase();
    if (!q) return savedSongs.value;
    return savedSongs.value.filter(
      (song) =>
        (song.title || '').toLocaleLowerCase().includes(q) ||
        (song.artist || '').toLocaleLowerCase().includes(q) ||
        (song.album || '').toLocaleLowerCase().includes(q),
    );
  });

  const filteredSearchResults = computed(() => {
    const list = searchData.value?.allResults || [];
    if (activeSourceTab.value === 'all') return list;
    return list.filter((item) => item.source === activeSourceTab.value);
  });

  return {
    searchTitle,
    searchArtist,
    searchHistory,
    isSearching,
    searchData,
    activeSourceTab,
    isLoadingMoreResults,
    utatenPageState,
    pendingSaltRequest,
    currentLyrics,
    isLoadingLyrics,
    exportTarget,
    savedSongs,
    filteredSongs,
    filteredSearchResults,
    songsSortBy,
    songsFilterQuery,
    isLoadingSongs,
    hydratingUrls,
    lspLogs,
    isLoadingLspLogs,
    lspLogZoom,
    submitSearch,
    loadNextUtatenPage,
    fetchLyricsForItem,
    selectSearchResult,
    openExportFromResult,
    openExportFromSavedSong,
    openExportForCurrentLyrics,
    loadSavedLyrics,
    openSavedLyrics,
    refreshSavedSongArtwork,
    deleteSavedSong,
    addSearchHistory,
    removeHistoryEntry,
    clearAllSearchHistory,
    clearAllCaches,
    fetchLspLogs,
    adjustLspLogZoom,
    syncLspSettings,
    setBackendLspLogging,
    appendAppLspLog,
    checkSaltLaunchRequest,
  };
}
