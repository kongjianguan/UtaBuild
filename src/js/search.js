import { invoke, isTauriEnv } from './tauri.js';
import { shouldUseCache, selectedArtworkSource } from './settings.js';
import { el, show, hide, showLoading, hideLoading, showError, router, updateButtonStates } from './dom.js';
import { renderLyrics } from './ruby.js';
import { addSearchHistory } from './search-history.js';
import { appendAppLspLog } from './lsp.js';
import { attachResultLongPressMenu, resultLongPressTriggered, setResultLongPressTriggered, } from './songs.js';
import { escapeHtml } from './utils.js';
import { setExportData } from './export.js';
// ==================== Search State ====================
export let currentSearchData = null;
export let currentActiveTab = 'all';
export let currentSearchQuery = null;
export let currentSearchRunId = 0;
export let isLoadingMoreResults = false;
export let utatenPageState = {
    currentPage: 1,
    totalPages: 1,
    hasNext: false,
    loadedPages: 1,
    loadingMore: false,
};
let _pendingSaltRequest = null;
export let isSearching = false;
export function getPendingSaltRequest() {
    return _pendingSaltRequest;
}
export function setPendingSaltRequest(val) {
    _pendingSaltRequest = val;
}
export function clearPendingSaltRequest() {
    _pendingSaltRequest = null;
}
// ==================== Pagination ====================
function getPaginationInfo() {
    return {
        currentPage: utatenPageState.currentPage,
        totalPages: utatenPageState.totalPages,
        hasNext: utatenPageState.hasNext,
        loadedPages: utatenPageState.loadedPages,
        loadingMore: utatenPageState.loadingMore,
    };
}
function updatePagination() {
    const { totalPages, loadedPages, hasNext, loadingMore } = getPaginationInfo();
    const showPagination = totalPages > 1;
    if (showPagination) {
        el('pagination-info').textContent = loadingMore
            ? `${loadedPages}/${totalPages}ページ読み込み済み · 続きを読み込み中...`
            : hasNext
                ? `${loadedPages}/${totalPages}ページ読み込み済み · 下にスクロールして続きを読み込む`
                : `${loadedPages}/${totalPages}ページを読み込み済み`;
    }
    else {
        el('pagination-info').textContent = '';
    }
    el('results-pagination').classList.toggle('hidden', !showPagination);
    syncInfiniteScrollObserver();
}
// ==================== Results Summary ====================
function updateResultsSummary(data) {
    const utatenCount = data?.sources?.utaten?.data?.results?.length ?? 0;
    const qqCount = data?.sources?.qq_music?.data?.results?.length ?? 0;
    const neCount = data?.sources?.netease?.data?.results?.length ?? 0;
    const totalCount = utatenCount + qqCount + neCount;
    const { loadedPages, totalPages, loadingMore } = getPaginationInfo();
    const title = data?.query?.title ?? currentSearchQuery?.title ?? '';
    const artist = data?.query?.artist ?? currentSearchQuery?.artist;
    const queryLabel = artist ? `${title} / ${artist}` : title;
    if (!queryLabel) {
        hide(el('results-summary'));
        return;
    }
    const loadingSuffix = loadingMore ? '・続きを取得中' : '';
    const perSource = totalCount > 0
        ? `（UtaTen: ${utatenCount} / QQ: ${qqCount} / NE: ${neCount}）`
        : '';
    el('results-summary').textContent = `「${queryLabel}」の検索結果 ${totalCount}件${perSource}（${loadedPages}/${totalPages}ページ読み込み済み${loadingSuffix}）`;
    show(el('results-summary'));
}
// ==================== Infinite Scroll ====================
let resultsScrollObserver = null;
let resultsScrollEventsInitialized = false;
function syncInfiniteScrollObserver() {
    if (!resultsScrollObserver)
        return;
    resultsScrollObserver.disconnect();
    if (!el('results-pagination').classList.contains('hidden')) {
        resultsScrollObserver.observe(el('results-pagination'));
    }
}
export function initInfiniteScroll() {
    if (!resultsScrollEventsInitialized) {
        window.addEventListener('scroll', maybeLoadMoreResults, { passive: true });
        window.addEventListener('resize', maybeLoadMoreResults);
        resultsScrollEventsInitialized = true;
    }
    if (typeof window.IntersectionObserver === 'function') {
        resultsScrollObserver = new window.IntersectionObserver((entries) => {
            if (entries.some((entry) => entry.isIntersecting)) {
                maybeLoadMoreResults();
            }
        }, {
            root: null,
            rootMargin: '0px 0px 160px 0px',
            threshold: 0,
        });
        syncInfiniteScrollObserver();
    }
}
// ==================== Load More Results ====================
async function loadRemainingSearchPages(searchRunId) {
    if (currentSearchRunId !== searchRunId ||
        !currentSearchQuery ||
        !currentSearchData ||
        isLoadingMoreResults) {
        return;
    }
    if (!utatenPageState.hasNext ||
        utatenPageState.currentPage >= utatenPageState.totalPages) {
        if (utatenPageState.loadingMore) {
            utatenPageState = { ...utatenPageState, loadingMore: false };
            if (currentSearchData)
                renderResultList(currentSearchData);
        }
        return;
    }
    const nextPage = utatenPageState.currentPage + 1;
    isLoadingMoreResults = true;
    utatenPageState = { ...utatenPageState, loadingMore: true };
    if (currentSearchData)
        renderResultList(currentSearchData);
    try {
        const nextResult = await invoke('search_lyrics', {
            title: currentSearchQuery.title,
            artist: currentSearchQuery.artist ?? null,
            page: nextPage,
            useCache: shouldUseCache(),
            lyricSource: 'utaten',
        });
        if (currentSearchRunId !== searchRunId)
            return;
        if (nextResult.status !== 'select' || !Array.isArray(nextResult.results)) {
            throw new Error(nextResult.error || `ページ ${nextPage} の取得に失敗しました`);
        }
        const newItems = (nextResult.results || []).map((item) => ({
            ...item,
            source: 'utaten',
        }));
        const existingUrls = new Set((currentSearchData?.allResults || []).map((r) => r.url));
        const dedupedNew = newItems.filter((item) => !existingUrls.has(item.url));
        currentSearchData = {
            ...currentSearchData,
            sources: {
                ...currentSearchData.sources,
                utaten: { data: nextResult, error: null },
            },
            allResults: [
                ...currentSearchData.allResults,
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
        if (currentSearchData)
            renderResultList(currentSearchData);
    }
    catch (err) {
        if (currentSearchRunId !== searchRunId)
            return;
        utatenPageState = { ...utatenPageState, loadingMore: false, hasNext: false };
        if (currentSearchData)
            renderResultList(currentSearchData);
        console.error('Load more search results error:', err);
    }
    finally {
        isLoadingMoreResults = false;
        maybeLoadMoreResults();
    }
}
function maybeLoadMoreResults() {
    if (router.current !== 'results' ||
        !currentSearchData ||
        isLoadingMoreResults ||
        el('results-pagination').classList.contains('hidden')) {
        return;
    }
    const rect = el('results-pagination').getBoundingClientRect();
    const viewportHeight = window.innerHeight || document.documentElement?.clientHeight || 0;
    if (rect.top <= viewportHeight + 160) {
        void loadRemainingSearchPages(currentSearchRunId);
    }
}
// ==================== Perform Search ====================
async function performSearch(page = 1, searchRunId = currentSearchRunId) {
    const title = currentSearchQuery?.title;
    const artist = currentSearchQuery?.artist ?? null;
    if (!title) {
        showError('曲名を入力してください');
        return;
    }
    showLoading();
    console.log('\u{1F50D} 搜索:', title, '| isTauriEnv:', isTauriEnv(), '| invoke:', typeof invoke);
    try {
        const results = await Promise.allSettled([
            invoke('search_lyrics', {
                title,
                artist,
                page,
                useCache: shouldUseCache(),
                lyricSource: 'utaten',
            }),
            invoke('search_lyrics', {
                title,
                artist,
                page,
                useCache: shouldUseCache(),
                lyricSource: 'qq_music',
            }),
            invoke('search_lyrics', {
                title,
                artist,
                page,
                useCache: shouldUseCache(),
                lyricSource: 'netease',
            }),
        ]);
        if (searchRunId !== currentSearchRunId)
            return;
        // eslint-disable-next-line no-inner-declarations
        function extractResult(index, sourceName) {
            if (results[index].status !== 'fulfilled') {
                return {
                    data: null,
                    sourceError: results[index].reason?.message ||
                        results[index].reason ||
                        `${sourceName} 検索に失敗しました`,
                };
            }
            const val = results[index].value;
            if (val && val.status === 'select' && val.results && val.results.length > 0) {
                return { data: val, sourceError: null };
            }
            return { data: null, sourceError: `${sourceName} で結果が見つかりませんでした` };
        }
        const utatenInfo = extractResult(0, 'UtaTen');
        const qqMusicInfo = extractResult(1, 'QQ音楽');
        const neteaseInfo = extractResult(2, '网易雲');
        const utatenItems = (utatenInfo.data?.results || []).map((item) => ({
            ...item,
            source: 'utaten',
        }));
        const qqMusicItems = (qqMusicInfo.data?.results || []).map((item) => ({
            ...item,
            source: 'qq_music',
        }));
        const neteaseItems = (neteaseInfo.data?.results || []).map((item) => ({
            ...item,
            source: 'netease',
        }));
        currentSearchData = {
            query: { title, artist },
            sources: {
                utaten: { data: utatenInfo.data ?? null, error: utatenInfo.sourceError },
                qq_music: {
                    data: qqMusicInfo.data ?? null,
                    error: qqMusicInfo.sourceError,
                },
                netease: {
                    data: neteaseInfo.data ?? null,
                    error: neteaseInfo.sourceError,
                },
            },
            allResults: [...utatenItems, ...qqMusicItems, ...neteaseItems],
        };
        currentActiveTab = 'all';
        utatenPageState = {
            currentPage: utatenInfo.data?.pagination?.current_page || 1,
            totalPages: utatenInfo.data?.pagination?.total_pages || 1,
            hasNext: utatenInfo.data?.pagination?.has_next ?? false,
            loadedPages: 1,
            loadingMore: false,
        };
        if (currentSearchData.allResults.length > 0) {
            void appendAppLspLog('search', `search success utaten=${utatenItems.length} qq=${qqMusicItems.length} ne=${neteaseItems.length}`);
            renderResultList(currentSearchData);
            if (router.current === 'results') {
                router.navigate('results');
            }
            else {
                router.navigate('results');
            }
            maybeLoadMoreResults();
        }
        else {
            void appendAppLspLog('search', 'search not_found');
            showError('結果が見つかりませんでした');
        }
    }
    catch (err) {
        console.error('Search error:', err);
        void appendAppLspLog('search', `search error ${String(err)}`);
        showError(`検索エラー: ${err}`);
    }
    finally {
        hideLoading();
    }
}
// ==================== Handle Search (entry point) ====================
export async function handleSearch() {
    if (isSearching)
        return;
    const title = el('search-title').value.trim();
    const artist = el('search-artist').value.trim() || null;
    currentSearchQuery = { title, artist };
    currentSearchRunId += 1;
    isLoadingMoreResults = false;
    currentSearchData = null;
    currentActiveTab = 'all';
    addSearchHistory(title, artist);
    void appendAppLspLog('ui', `search requested title="${title}" artist="${artist || ''}"`);
    isSearching = true;
    el('search-btn').classList.add('is-searching');
    try {
        await performSearch(1, currentSearchRunId);
    }
    finally {
        isSearching = false;
        el('search-btn').classList.remove('is-searching');
    }
}
// ==================== Render Result List ====================
export function renderResultList(data) {
    el('results-container').innerHTML = '';
    updateResultsSummary(data);
    updatePagination();
    let filteredResults = data && data.allResults ? data.allResults : [];
    if (currentActiveTab !== 'all') {
        filteredResults = filteredResults.filter((item) => item.source === currentActiveTab);
    }
    updateSourceTabs(data);
    if (filteredResults.length === 0 && currentActiveTab !== 'all') {
        const sourceData = data?.sources?.[currentActiveTab];
        if (sourceData?.error) {
            const errorEl = document.createElement('div');
            errorEl.className = 'source-error';
            errorEl.setAttribute('role', 'alert');
            errorEl.textContent = sourceData.error;
            el('results-container').appendChild(errorEl);
            return;
        }
    }
    filteredResults.forEach((item) => {
        const button = document.createElement('button');
        button.type = 'button';
        button.className = 'result-item';
        let sourceBadge;
        if (item.source === 'qq_music') {
            sourceBadge = '<span class="source-badge source-badge--qq">QQ</span>';
        }
        else if (item.source === 'netease') {
            sourceBadge = '<span class="source-badge source-badge--ne">NE</span>';
        }
        else {
            sourceBadge = '<span class="source-badge source-badge--utaten">UtaTen</span>';
        }
        button.innerHTML = `
      ${sourceBadge}
      <div class="title">${escapeHtml(item.title)}</div>
      <div class="artist">${escapeHtml(item.artist)}</div>
    `;
        const realIndex = (data?.allResults || []).indexOf(item);
        attachResultLongPressMenu(button, item);
        button.addEventListener('click', () => {
            if (resultLongPressTriggered) {
                setResultLongPressTriggered(false);
                return;
            }
            void handleSelectResult(realIndex);
        });
        el('results-container').appendChild(button);
    });
}
function updateSourceTabs(data) {
    if (!el('source-tabs'))
        return;
    const tabs = el('source-tabs').querySelectorAll('.source-tab');
    const utatenCount = data?.sources?.utaten?.data?.results?.length ?? 0;
    const qqCount = data?.sources?.qq_music?.data?.results?.length ?? 0;
    const neCount = data?.sources?.netease?.data?.results?.length ?? 0;
    const totalCount = utatenCount + qqCount + neCount;
    const utatenErr = data?.sources?.utaten?.error ?? null;
    const qqErr = data?.sources?.qq_music?.error ?? null;
    const neErr = data?.sources?.netease?.error ?? null;
    tabs.forEach((tab) => {
        const tabEl = tab;
        const source = tabEl.dataset.source;
        if (!source)
            return;
        tabEl.classList.toggle('active', source === currentActiveTab);
        tabEl.setAttribute('aria-selected', String(source === currentActiveTab));
        let countEl = tabEl.querySelector('.tab-count');
        let count = totalCount;
        let errorMsg = null;
        if (source === 'utaten') {
            count = utatenCount;
            errorMsg = utatenErr;
        }
        else if (source === 'qq_music') {
            count = qqCount;
            errorMsg = qqErr;
        }
        else if (source === 'netease') {
            count = neCount;
            errorMsg = neErr;
        }
        if (!countEl) {
            countEl = document.createElement('span');
            countEl.className = 'tab-count';
            tabEl.appendChild(countEl);
        }
        if (errorMsg) {
            countEl.textContent = '\u2715';
            tabEl.dataset.empty = 'true';
            tabEl.title = errorMsg;
        }
        else {
            countEl.textContent = `(${count})`;
            tabEl.dataset.empty = count === 0 ? 'true' : 'false';
            tabEl.title = '';
        }
    });
    show(el('source-tabs'));
}
// ==================== Handle Select Result ====================
async function handleSelectResult(index) {
    if (!currentSearchData)
        return;
    const selectedItem = currentSearchData.allResults[index];
    if (!selectedItem)
        return;
    const saltRequest = _pendingSaltRequest;
    if (saltRequest) {
        const confirmed = window.confirm(`Salt Player の「${saltRequest.title || ''}」に UtaBuild の「${selectedItem.title}」を紐付け、今後 Ruby 表示に使用しますか？`);
        if (!confirmed) {
            void appendAppLspLog('salt', `binding cancelled selected="${selectedItem.title}"`);
            return;
        }
    }
    showLoading();
    try {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let result = await invoke('get_lyrics', {
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
            void appendAppLspLog('lyrics', `QQ failed for "${selectedItem.title}", trying UtaTen fallback`);
            const utatenResults = currentSearchData?.sources?.utaten?.data?.results || [];
            const match = utatenResults.find((r) => r.title === selectedItem.title && r.artist === selectedItem.artist);
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
            void appendAppLspLog('lyrics', `NetEase failed for "${selectedItem.title}", trying UtaTen fallback`);
            const utatenResults = currentSearchData?.sources?.utaten?.data?.results || [];
            const match = utatenResults.find((r) => r.title === selectedItem.title && r.artist === selectedItem.artist);
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
            if (saltRequest) {
                await invoke('bind_salt_song_lyrics', {
                    saltTitle: saltRequest.title || selectedItem.title,
                    saltArtist: saltRequest.artist || null,
                    lyrics: result,
                });
                _pendingSaltRequest = null;
                void appendAppLspLog('salt', `binding saved salt="${saltRequest.title || ''}" selected="${selectedItem.title}"`);
            }
            el('lyrics-title').textContent = result.found_title;
            el('lyrics-artist').textContent = result.found_artist;
            el('lyrics-body').innerHTML = '';
            const lyricsEl = renderLyrics(result.ruby_annotations);
            el('lyrics-body').appendChild(lyricsEl);
            updateButtonStates();
            router.navigate('lyrics', { resetScroll: true });
            setExportData({
                title: result.found_title,
                artist: result.found_artist,
                lyricsUrl: result.lyrics_url,
                rubyAnnotations: result.ruby_annotations,
                coverUrl: result.cover_url ?? null,
            });
        }
        else {
            void appendAppLspLog('lyrics', `get lyrics failed selected="${selectedItem.title}"`);
            showError(result.error || '歌詞の取得に失敗しました');
        }
    }
    catch (err) {
        console.error('Select error:', err);
        void appendAppLspLog('lyrics', `select error ${String(err)}`);
        showError(`エラー: ${err}`);
    }
    finally {
        hideLoading();
    }
}
export function setCurrentActiveTab(val) {
    currentActiveTab = val;
}
//# sourceMappingURL=search.js.map