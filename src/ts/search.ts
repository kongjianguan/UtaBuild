import type { SearchData, SearchItem, SearchResponse, UtatenPageState } from './types.js';
import { invoke, isTauriEnv } from './tauri.js';
import { shouldUseCache, selectedArtworkSource } from './settings.js';
import {
  el,
  show,
  hide,
  showLoading,
  hideLoading,
  showError,
  router,
  updateButtonStates,
} from './dom.js';
import { renderLyrics } from './ruby.js';
import { addSearchHistory } from './search-history.js';
import { appendAppLspLog } from './lsp.js';
import {
  attachResultLongPressMenu,
  cancelPendingResultLongPress,
  consumeResultLongPressClick,
} from './songs.js';
import { escapeHtml } from './utils.js';
import { setExportData } from './export.js';

// ==================== Search State ====================

export let currentSearchData: SearchData | null = null;
export let currentActiveTab = 'all';
export let currentSearchQuery: { title: string; artist: string | null } | null = null;
export let currentSearchRunId = 0;
export let isLoadingMoreResults = false;
export let utatenPageState: UtatenPageState = {
  currentPage: 1,
  totalPages: 1,
  hasNext: false,
  loadedPages: 1,
  loadingMore: false,
};
let _pendingSaltRequest: { title?: string; artist?: string | null } | null = null;
export let isSearching = false;
let currentLyricsRequestId = 0;

type SearchSourceKey = 'utaten' | 'qq_music' | 'netease';

const SEARCH_SOURCES: Array<{ key: SearchSourceKey; label: string }> = [
  { key: 'utaten', label: 'UtaTen' },
  { key: 'qq_music', label: 'QQ音楽' },
  { key: 'netease', label: '网易雲' },
];

export function getPendingSaltRequest(): {
  title?: string;
  artist?: string | null;
} | null {
  return _pendingSaltRequest;
}

export function setPendingSaltRequest(
  val: { title?: string; artist?: string | null } | null,
): void {
  _pendingSaltRequest = val;
}

export function clearPendingSaltRequest(): void {
  _pendingSaltRequest = null;
}

// ==================== Pagination ====================

function getPaginationInfo(): {
  currentPage: number;
  totalPages: number;
  hasNext: boolean;
  loadedPages: number;
  loadingMore: boolean;
} {
  return {
    currentPage: utatenPageState.currentPage,
    totalPages: utatenPageState.totalPages,
    hasNext: utatenPageState.hasNext,
    loadedPages: utatenPageState.loadedPages,
    loadingMore: utatenPageState.loadingMore,
  };
}

function updatePagination(): void {
  const { totalPages, loadedPages, hasNext, loadingMore } = getPaginationInfo();
  const showPagination = totalPages > 1;

  if (showPagination) {
    el<HTMLElement>('pagination-info').textContent = loadingMore
      ? `${loadedPages}/${totalPages}ページ読み込み済み · 続きを読み込み中...`
      : hasNext
        ? `${loadedPages}/${totalPages}ページ読み込み済み · 下にスクロールして続きを読み込む`
        : `${loadedPages}/${totalPages}ページを読み込み済み`;
  } else {
    el<HTMLElement>('pagination-info').textContent = '';
  }
  el<HTMLElement>('results-pagination').classList.toggle('hidden', !showPagination);
  syncInfiniteScrollObserver();
}

// ==================== Results Summary ====================

function updateResultsSummary(data: SearchData | null): void {
  const loadedResults = data?.allResults || [];
  const utatenCount = loadedResults.filter((item) => item.source === 'utaten').length;
  const qqCount = loadedResults.filter((item) => item.source === 'qq_music').length;
  const neCount = loadedResults.filter((item) => item.source === 'netease').length;
  const totalCount = utatenCount + qqCount + neCount;
  const { loadedPages, totalPages, loadingMore } = getPaginationInfo();
  const loadingSources = Object.values(data?.sources || {}).filter(
    (source) => source.loading,
  ).length;
  const title = data?.query?.title ?? currentSearchQuery?.title ?? '';
  const artist = data?.query?.artist ?? currentSearchQuery?.artist;
  const queryLabel = artist ? `${title} / ${artist}` : title;

  if (!queryLabel) {
    hide(el<HTMLElement>('results-summary'));
    return;
  }

  if (loadingSources > 0 && totalCount === 0) {
    el<HTMLElement>('results-summary').textContent = `「${queryLabel}」の検索結果を取得中...`;
    show(el<HTMLElement>('results-summary'));
    return;
  }

  const loadingSuffix = loadingMore
    ? '・続きを取得中'
    : loadingSources > 0
      ? `・${loadingSources}件のソースを読み込み中`
      : '';
  const perSource =
    totalCount > 0
      ? `（UtaTen: ${utatenCount} / QQ: ${qqCount} / NE: ${neCount}）`
      : '';
  el<HTMLElement>('results-summary').textContent = `「${queryLabel}」の検索結果 ${totalCount}件${perSource}（${loadedPages}/${totalPages}ページ読み込み済み${loadingSuffix}）`;
  show(el<HTMLElement>('results-summary'));
}

// ==================== Infinite Scroll ====================

let resultsScrollObserver: IntersectionObserver | null = null;
let resultsScrollEventsInitialized = false;

function syncInfiniteScrollObserver(): void {
  if (!resultsScrollObserver) return;

  resultsScrollObserver.disconnect();
  if (!el<HTMLElement>('results-pagination').classList.contains('hidden')) {
    resultsScrollObserver.observe(el<HTMLElement>('results-pagination'));
  }
}

export function initInfiniteScroll(): void {
  if (!resultsScrollEventsInitialized) {
    el<HTMLElement>('results-scroll').addEventListener('scroll', maybeLoadMoreResults, {
      passive: true,
    });
    window.addEventListener('resize', maybeLoadMoreResults);
    resultsScrollEventsInitialized = true;
  }

  el<HTMLElement>('results-container').addEventListener('click', (event) => {
    const target = (event.target as HTMLElement).closest<HTMLButtonElement>('.result-item');
    if (!target) return;
    if (consumeResultLongPressClick(target, event)) return;
    const index = Number(target.dataset.index);
    if (!Number.isInteger(index)) return;
    void handleSelectResult(index);
  });

  if (typeof window.IntersectionObserver === 'function') {
    resultsScrollObserver = new window.IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          maybeLoadMoreResults();
        }
      },
      {
        root: el<HTMLElement>('results-scroll'),
        rootMargin: '0px 0px 160px 0px',
        threshold: 0,
      },
    );

    syncInfiniteScrollObserver();
  }
}

// ==================== Load More Results ====================

async function loadRemainingSearchPages(searchRunId: number): Promise<void> {
  if (
    currentSearchRunId !== searchRunId ||
    !currentSearchQuery ||
    !currentSearchData ||
    isLoadingMoreResults
  ) {
    return;
  }

  if (
    !utatenPageState.hasNext ||
    utatenPageState.currentPage >= utatenPageState.totalPages
  ) {
    if (utatenPageState.loadingMore) {
      utatenPageState = { ...utatenPageState, loadingMore: false };
      if (currentSearchData) renderResultList(currentSearchData);
    }
    return;
  }

  const nextPage = utatenPageState.currentPage + 1;
  isLoadingMoreResults = true;
  utatenPageState = { ...utatenPageState, loadingMore: true };
  if (currentSearchData) renderResultList(currentSearchData);

  try {
    const nextResult = await invoke<SearchResponse & { results?: SearchItem[] }>(
      'search_lyrics',
      {
        title: currentSearchQuery.title,
        artist: currentSearchQuery.artist ?? null,
        page: nextPage,
        useCache: shouldUseCache(),
        lyricSource: 'utaten',
      },
    );

    if (currentSearchRunId !== searchRunId) return;

    if (nextResult.status !== 'select' || !Array.isArray(nextResult.results)) {
      throw new Error(nextResult.error || `ページ ${nextPage} の取得に失敗しました`);
    }

    const newItems = (nextResult.results || []).map((item) => ({
      ...item,
      source: 'utaten' as const,
    }));
    const existingUrls = new Set(
      (currentSearchData?.allResults || []).map((r) => r.url),
    );
    const dedupedNew = newItems.filter((item) => !existingUrls.has(item.url));

    currentSearchData = {
      ...(currentSearchData as SearchData),
      sources: {
        ...(currentSearchData as SearchData).sources,
        utaten: { data: nextResult, error: null },
      },
      allResults: [
        ...(currentSearchData as SearchData).allResults,
        ...dedupedNew,
      ],
    };

    utatenPageState = {
      currentPage: nextPage,
      totalPages: nextResult.pagination?.total_pages ?? utatenPageState.totalPages,
      hasNext: nextResult.pagination?.has_next ?? false,
      loadedPages: nextPage,
      loadingMore: false,
    };

    if (currentSearchData) renderResultList(currentSearchData);
  } catch (err) {
    if (currentSearchRunId !== searchRunId) return;

    utatenPageState = { ...utatenPageState, loadingMore: false, hasNext: false };
    if (currentSearchData) renderResultList(currentSearchData);
    console.error('Load more search results error:', err);
  } finally {
    isLoadingMoreResults = false;
    maybeLoadMoreResults();
  }
}

function maybeLoadMoreResults(): void {
  if (
    router.current !== 'results' ||
    !currentSearchData ||
    isLoadingMoreResults ||
    el<HTMLElement>('results-pagination').classList.contains('hidden')
  ) {
    return;
  }

  const scrollView = el<HTMLElement>('results-scroll');
  const rect = el<HTMLElement>('results-pagination').getBoundingClientRect();
  const viewportRect = scrollView.getBoundingClientRect();

  if (rect.top <= viewportRect.bottom + 160) {
    void loadRemainingSearchPages(currentSearchRunId);
  }
}

// ==================== Perform Search ====================

async function performSearch(
  page = 1,
  searchRunId = currentSearchRunId,
): Promise<void> {
  const title = currentSearchQuery?.title;
  const artist = currentSearchQuery?.artist ?? null;

  if (!title) {
    showError('曲名を入力してください');
    return;
  }

  showLoading();
  let firstSourceSettled = false;

  console.log(
    '\u{1F50D} 搜索:',
    title,
    '| isTauriEnv:',
    isTauriEnv(),
    '| invoke:',
    typeof invoke,
  );

  try {
    currentSearchData = {
      query: { title, artist },
      sources: {
        utaten: { data: null, error: null, loading: true },
        qq_music: { data: null, error: null, loading: true },
        netease: { data: null, error: null, loading: true },
      },
      allResults: [],
    };
    currentActiveTab = 'all';
    utatenPageState = {
      currentPage: 1,
      totalPages: 1,
      hasNext: false,
      loadedPages: 1,
      loadingMore: false,
    };
    router.navigate('results', { resetScroll: true });
    renderResultList(currentSearchData);

    const sourceResults = SEARCH_SOURCES.map(async ({ key, label }) => {
      try {
        const result = await invoke<SearchResponse>('search_lyrics', {
          title,
          artist,
          page,
          useCache: shouldUseCache(),
          lyricSource: key,
        });

        if (searchRunId !== currentSearchRunId || !currentSearchData) return;

        const hasResults =
          result?.status === 'select' && Array.isArray(result.results) && result.results.length > 0;
        const items = hasResults
          ? result.results.map((item) => ({ ...item, source: key }))
          : [];

        currentSearchData = {
          ...currentSearchData,
          sources: {
            ...currentSearchData.sources,
            [key]: {
              data: hasResults ? result : null,
              error: hasResults ? null : result?.error || `${label} で結果が見つかりませんでした`,
              loading: false,
            },
          },
          allResults: [
            ...currentSearchData.allResults,
            ...items.filter(
              (item) => !currentSearchData?.allResults.some((existing) => existing.url === item.url),
            ),
          ],
        };

        if (key === 'utaten') {
          utatenPageState = {
            currentPage: result?.pagination?.current_page || 1,
            totalPages: result?.pagination?.total_pages || 1,
            hasNext: result?.pagination?.has_next ?? false,
            loadedPages: 1,
            loadingMore: false,
          };
        }

        renderResultList(currentSearchData);
        if (key === 'utaten') {
          maybeLoadMoreResults();
        }
        if (!firstSourceSettled) {
          firstSourceSettled = true;
          hideLoading();
        }
      } catch (err) {
        if (searchRunId !== currentSearchRunId || !currentSearchData) return;

        const reason = err as { message?: string } | null;
        currentSearchData = {
          ...currentSearchData,
          sources: {
            ...currentSearchData.sources,
            [key]: {
              data: null,
              error: reason?.message || String(err) || `${label} 検索に失敗しました`,
              loading: false,
            },
          },
        };
        renderResultList(currentSearchData);
        if (!firstSourceSettled) {
          firstSourceSettled = true;
          hideLoading();
        }
      }
    });

    await Promise.all(sourceResults);

    if (searchRunId !== currentSearchRunId || !currentSearchData) return;

    const counts = SEARCH_SOURCES.map(({ key }) =>
      currentSearchData?.allResults.filter((item) => item.source === key).length || 0,
    );
    if (currentSearchData.allResults.length > 0) {
      void appendAppLspLog(
        'search',
        `search success utaten=${counts[0]} qq=${counts[1]} ne=${counts[2]}`,
      );
      maybeLoadMoreResults();
    } else {
      const sourceStates = SEARCH_SOURCES.map(
        ({ key }) => currentSearchData?.sources?.[key],
      );
      const realErrors = sourceStates
        .map((source) => source?.error)
        .filter(
          (error): error is string => !!error && !error.includes('見つかりませんでした'),
        );
      if (realErrors.length > 0) {
        void appendAppLspLog('search', `search failed: ${realErrors[0]}`);
        showError(realErrors[0]);
      } else {
        void appendAppLspLog('search', 'search not_found');
        showError('結果が見つかりませんでした');
      }
    }
  } catch (err) {
    console.error('Search error:', err);
    void appendAppLspLog('search', `search error ${String(err)}`);
    showError(`検索エラー: ${err}`);
  } finally {
    if (searchRunId === currentSearchRunId && !firstSourceSettled) {
      hideLoading();
    }
  }
}

// ==================== Handle Search (entry point) ====================

export async function handleSearch(): Promise<void> {
  if (isSearching) return;

  const title = el<HTMLInputElement>('search-title').value.trim();
  const artist = el<HTMLInputElement>('search-artist').value.trim() || null;

  if (!title) {
    showError('曲名を入力してください');
    el<HTMLInputElement>('search-title').focus();
    return;
  }

  currentSearchQuery = { title, artist };
  currentSearchRunId += 1;
  currentLyricsRequestId += 1;
  isLoadingMoreResults = false;
  currentSearchData = null;
  currentActiveTab = 'all';

  addSearchHistory(title, artist);
  void appendAppLspLog(
    'ui',
    `search requested title="${title}" artist="${artist || ''}"`,
  );

  isSearching = true;
  const searchButton = el<HTMLButtonElement>('search-btn');
  searchButton.classList.add('is-searching');
  searchButton.disabled = true;
  searchButton.setAttribute('aria-busy', 'true');

  try {
    await performSearch(1, currentSearchRunId);
  } finally {
    isSearching = false;
    searchButton.classList.remove('is-searching');
    searchButton.disabled = false;
    searchButton.removeAttribute('aria-busy');
  }
}

// ==================== Render Result List ====================

export function renderResultList(data: SearchData | null): void {
  cancelPendingResultLongPress();
  el<HTMLElement>('results-container').innerHTML = '';
  updateResultsSummary(data);
  updatePagination();

  let filteredResults: SearchItem[] =
    data && data.allResults ? data.allResults : [];
  if (currentActiveTab !== 'all') {
    filteredResults = filteredResults.filter(
      (item) => item.source === currentActiveTab,
    );
  }

  updateSourceTabs(data);

  if (filteredResults.length === 0 && currentActiveTab !== 'all') {
    const sourceData = data?.sources?.[currentActiveTab];
    if (sourceData?.loading) {
      const loadingEl = document.createElement('div');
      loadingEl.className = 'source-loading';
      loadingEl.setAttribute('role', 'status');
      loadingEl.textContent = 'このソースの検索結果を読み込み中...';
      el<HTMLElement>('results-container').appendChild(loadingEl);
      return;
    }
    if (sourceData?.error) {
      const errorEl = document.createElement('div');
      errorEl.className = 'source-error';
      errorEl.setAttribute('role', 'alert');
      errorEl.textContent = sourceData.error;
      el<HTMLElement>('results-container').appendChild(errorEl);
      return;
    }
  }

  if (
    filteredResults.length === 0 &&
    currentActiveTab === 'all' &&
    Object.values(data?.sources || {}).some((source) => source.loading)
  ) {
    const loadingEl = document.createElement('div');
    loadingEl.className = 'source-loading';
    loadingEl.setAttribute('role', 'status');
    loadingEl.textContent = '検索結果を読み込み中...';
    el<HTMLElement>('results-container').appendChild(loadingEl);
  }

  filteredResults.forEach((item) => {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'result-item';

    let sourceBadge: string;
    if (item.source === 'qq_music') {
      sourceBadge = '<span class="source-badge source-badge--qq">QQ</span>';
    } else if (item.source === 'netease') {
      sourceBadge = '<span class="source-badge source-badge--ne">NE</span>';
    } else {
      sourceBadge = '<span class="source-badge source-badge--utaten">UtaTen</span>';
    }

    button.innerHTML = `
      ${sourceBadge}
      <div class="title">${escapeHtml(item.title)}</div>
      <div class="artist">${escapeHtml(item.artist)}</div>
    `;

    const realIndex = (data?.allResults || []).indexOf(item);
    button.dataset.index = String(realIndex);
    attachResultLongPressMenu(button, item);
    el<HTMLElement>('results-container').appendChild(button);
  });
}

function updateSourceTabs(data: SearchData | null): void {
  if (!el<HTMLElement>('source-tabs')) return;

  const tabs = el<HTMLElement>('source-tabs').querySelectorAll('.source-tab');
  const loadedResults = data?.allResults || [];
  const utatenCount = loadedResults.filter((item) => item.source === 'utaten').length;
  const qqCount = loadedResults.filter((item) => item.source === 'qq_music').length;
  const neCount = loadedResults.filter((item) => item.source === 'netease').length;
  const totalCount = utatenCount + qqCount + neCount;
  const utatenErr = data?.sources?.utaten?.error ?? null;
  const qqErr = data?.sources?.qq_music?.error ?? null;
  const neErr = data?.sources?.netease?.error ?? null;

  tabs.forEach((tab) => {
    const tabEl = tab as HTMLElement;
    const source = tabEl.dataset.source;
    if (!source) return;

    tabEl.classList.toggle('active', source === currentActiveTab);
    tabEl.setAttribute('aria-selected', String(source === currentActiveTab));
    // Roving tabindex: only the active tab stays in the tab order.
    tabEl.tabIndex = source === currentActiveTab ? 0 : -1;

    let countEl = tabEl.querySelector('.tab-count') as HTMLElement | null;
    let count = totalCount;
    let errorMsg: string | null = null;
    if (source === 'utaten') {
      count = utatenCount;
      errorMsg = utatenErr;
    } else if (source === 'qq_music') {
      count = qqCount;
      errorMsg = qqErr;
    } else if (source === 'netease') {
      count = neCount;
      errorMsg = neErr;
    }

    if (!countEl) {
      countEl = document.createElement('span');
      countEl.className = 'tab-count';
      tabEl.appendChild(countEl);
    }

    const sourceData = source === 'all' ? null : data?.sources?.[source];
    if (sourceData?.loading) {
      countEl.textContent = '…';
      tabEl.dataset.empty = 'false';
      tabEl.title = `${source} を読み込み中`;
    } else if (errorMsg) {
      countEl.textContent = '\u2715';
      tabEl.dataset.empty = 'true';
      tabEl.title = errorMsg;
    } else {
      countEl.textContent = `(${count})`;
      tabEl.dataset.empty = count === 0 ? 'true' : 'false';
      tabEl.title = '';
    }
  });

  show(el<HTMLElement>('source-tabs'));
}

// ==================== Handle Select Result ====================

async function handleSelectResult(index: number): Promise<void> {
  if (!currentSearchData) return;
  const selectedItem = currentSearchData.allResults[index];
  if (!selectedItem) return;

  const requestId = ++currentLyricsRequestId;
  const saltRequest = _pendingSaltRequest;

  if (saltRequest) {
    const confirmed = window.confirm(
      `Salt Player の「${saltRequest.title || ''}」に UtaBuild の「${selectedItem.title}」を紐付け、今後 Ruby 表示に使用しますか？`,
    );
    if (!confirmed) {
      void appendAppLspLog(
        'salt',
        `binding cancelled salt="${saltRequest.title || ''}"`,
      );
      clearPendingSaltRequest();
      return;
    }
  }

  showLoading();

  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let result: any = await invoke('get_lyrics', {
      url: selectedItem.url,
      title: selectedItem.title,
      artist: selectedItem.artist || null,
      useCache: shouldUseCache(),
      saveSaltBridge: !saltRequest,
      artworkSource: selectedArtworkSource(),
      lyricSource: selectedItem.source || 'utaten',
    });

    // Fallback: if QQ source failed, try matching UtaTen result
    if (result.status !== 'success' && selectedItem.source === 'qq_music') {
      void appendAppLspLog(
        'lyrics',
        `QQ failed for "${selectedItem.title}", trying UtaTen fallback`,
      );
      const utatenItems =
        (currentSearchData?.allResults || []).filter((r) => r.source === 'utaten');
      const match = utatenItems.find(
        (r) =>
          r.title === selectedItem.title && r.artist === selectedItem.artist,
      );
      if (match) {
        result = await invoke('get_lyrics', {
          url: match.url,
          title: match.title,
          artist: match.artist || null,
          useCache: shouldUseCache(),
          saveSaltBridge: !saltRequest,
          artworkSource: selectedArtworkSource(),
          lyricSource: 'utaten',
        });
      }
    }

    // Fallback: if NetEase source failed, try matching UtaTen result
    if (result.status !== 'success' && selectedItem.source === 'netease') {
      void appendAppLspLog(
        'lyrics',
        `NetEase failed for "${selectedItem.title}", trying UtaTen fallback`,
      );
      const utatenItems =
        (currentSearchData?.allResults || []).filter((r) => r.source === 'utaten');
      const match = utatenItems.find(
        (r) =>
          r.title === selectedItem.title && r.artist === selectedItem.artist,
      );
      if (match) {
        result = await invoke('get_lyrics', {
          url: match.url,
          title: match.title,
          artist: match.artist || null,
          useCache: shouldUseCache(),
          saveSaltBridge: !saltRequest,
          artworkSource: selectedArtworkSource(),
          lyricSource: 'utaten',
        });
      }
    }

    if (result.status === 'success') {
      // A newer selection or a navigation away invalidates this request:
      // never render stale lyrics or force-navigate back to them.
      if (requestId !== currentLyricsRequestId || router.current !== 'results') return;
      if (saltRequest) {
        await invoke('bind_salt_song_lyrics', {
          saltTitle: saltRequest.title || selectedItem.title,
          saltArtist: saltRequest.artist || null,
          lyrics: result,
        });
        _pendingSaltRequest = null;
        void appendAppLspLog(
          'salt',
          `binding saved salt="${saltRequest.title || ''}" selected="${selectedItem.title}"`,
        );
      }
      if (requestId !== currentLyricsRequestId || router.current !== 'results') return;

      el<HTMLElement>('lyrics-title').textContent = result.found_title;
      el<HTMLElement>('lyrics-artist').textContent = result.found_artist;

      el<HTMLElement>('lyrics-body').innerHTML = '';
      const lyricsEl = renderLyrics(result.ruby_annotations);
      el<HTMLElement>('lyrics-body').appendChild(lyricsEl);

      updateButtonStates();
      router.navigate('lyrics', { resetScroll: true });

      setExportData({
        title: result.found_title,
        artist: result.found_artist,
        lyricsUrl: result.lyrics_url,
        rubyAnnotations: result.ruby_annotations,
        coverUrl: result.cover_url ?? null,
      });
    } else {
      void appendAppLspLog(
        'lyrics',
        `get lyrics failed selected="${selectedItem.title}"`,
      );
      showError(result.error || '歌詞の取得に失敗しました');
    }
  } catch (err) {
    console.error('Select error:', err);
    void appendAppLspLog('lyrics', `select error ${String(err)}`);
    showError(`エラー: ${err}`);
  } finally {
    hideLoading();
  }
}

export function setCurrentActiveTab(val: string): void {
  currentActiveTab = val;
}

const SOURCE_TAB_ORDER = ['all', 'utaten', 'qq_music', 'netease'];

export function initSourceTabKeyboardNav(): void {
  el<HTMLElement>('source-tabs').addEventListener('keydown', (event) => {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;

    const tabs = Array.from(
      el<HTMLElement>('source-tabs').querySelectorAll<HTMLElement>('.source-tab'),
    );
    if (tabs.length === 0) return;

    event.preventDefault();
    const currentIndex = SOURCE_TAB_ORDER.indexOf(currentActiveTab);
    const delta = event.key === 'ArrowRight' ? 1 : -1;
    const nextIndex = (currentIndex + delta + SOURCE_TAB_ORDER.length) % SOURCE_TAB_ORDER.length;
    const nextSource = SOURCE_TAB_ORDER[nextIndex];
    if (!nextSource) return;

    setCurrentActiveTab(nextSource);
    renderResultList(currentSearchData);
    tabs[nextIndex]?.focus();
  });
}
