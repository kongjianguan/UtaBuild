/**
 * UtaBuild - 前端主逻辑
 * 
 * 负责：搜索交互、结果列表、歌词渲染
 * 与Tauri后端通过 invoke() 通信
 */

// Tauri invoke（在Tauri环境中可用）
let invoke;
let isTauriEnv = false;

// 多种方式检测Tauri环境
async function initTauri() {
  // 方法1: 直接检测全局对象
  if (typeof window.__TAURI__ !== 'undefined') {
    if (window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      invoke = window.__TAURI__.core.invoke;
      isTauriEnv = true;
      console.log('🦊 Tauri v2 环境 (window.__TAURI__.core)');
      return;
    }
  }
  
  // 方法2: 尝试直接调用Tauri v2注入的全局invoke
  if (typeof window.__TAURI_INVOKE__ === 'function') {
    invoke = window.__TAURI_INVOKE__;
    isTauriEnv = true;
    console.log('🦊 Tauri v2 环境 (window.__TAURI_INVOKE__)');
    return;
  }
  
  // 方法3: 动态import（如果在Tauri中会成功）
  try {
    const mod = await import('@tauri-apps/api/core');
    if (mod && typeof mod.invoke === 'function') {
      invoke = mod.invoke;
      isTauriEnv = true;
      console.log('🦊 Tauri v2 环境 (dynamic import)');
      return;
    }
  } catch (e) {
    // 非Tauri环境，忽略
  }
  
  // 以上都失败 → 浏览器Mock模式
  console.log('🌐 浏览器Mock模式');
  invoke = async (cmd, args) => {
    console.log('[Mock]', cmd, args);
    
    // Mock搜索结果
    if (cmd === 'search_lyrics') {
      const page = Number(args.page || 1);
      return {
        status: 'select',
        query_title: args.title,
        query_artist: args.artist || null,
        page,
        pagination: {
          current_page: page,
          total_pages: 3,
          has_next: page < 3,
        },
        results: Array.from({ length: 3 }, (_, index) => ({
          index,
          title: `${args.title || '春日影'} P${page}-${index + 1}`,
          artist: index === 0 ? 'MyGO!!!!!' : index === 1 ? 'Ave Mujica' : 'Other',
          url: `/lyric/mock${page}-${index + 1}`,
          lyricist: index === 2 ? null : ['織田', 'CRYCHIC'][index],
          composer: index === 2 ? null : ['北澤', '祥子'][index],
        })),
      };
    }
    
    // Mock歌词
    if (cmd === 'get_lyrics') {
      return {
        status: 'success',
        found_title: '春日影',
        found_artist: 'MyGO!!!!!',
        lyrics_url: '/lyric/mock1',
        ruby_annotations: [
          { type: 'text', base: 'それでも' },
          { type: 'ruby', base: '不思議', ruby: 'ふしぎ' },
          { type: 'text', base: 'な' },
          { type: 'ruby', base: '時', ruby: 'とき' },
          { type: 'text', base: 'は' },
          { type: 'linebreak' },
          { type: 'text', base: 'ずっと' },
          { type: 'ruby', base: '続', ruby: 'つづ' },
          { type: 'text', base: 'いて' },
          { type: 'text', base: 'ほしかった' },
          { type: 'linebreak' },
          { type: 'ruby', base: '春', ruby: 'はる' },
          { type: 'ruby', base: '日', ruby: 'び' },
          { type: 'ruby', base: '影', ruby: 'かげ' },
          { type: 'text', base: 'の' },
          { type: 'ruby', base: '中', ruby: 'なか' },
          { type: 'linebreak' },
        ]
      };
    }

    if (cmd === 'clear_cache') {
      return null;
    }
    
    return null;
  };
}

// 立即初始化
initTauri();

// 调试：输出Tauri全局对象
console.log('window.__TAURI__ keys:', Object.keys(window.__TAURI__ || {}));
console.log('window.__TAURI__.core:', Object.keys((window.__TAURI__ || {}).core || {}));
console.log('window.__TAURI_INVOKE__:', typeof window.__TAURI_INVOKE__);

// ==================== DOM Elements ====================

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

const elements = {
  searchTitle: $('#search-title'),
  searchArtist: $('#search-artist'),
  searchBtn: $('#search-btn'),
  settingUseCache: $('#setting-use-cache'),
  settingClearCache: $('#setting-clear-cache'),
  resultList: $('#result-list'),
  resultsContainer: $('#results-container'),
  resultsSummary: $('#results-summary'),
  resultsPagination: $('#results-pagination'),
  paginationInfo: $('#pagination-info'),
  lyricsView: $('#lyrics-view'),
  lyricsTitle: $('#lyrics-title'),
  lyricsArtist: $('#lyrics-artist'),
  lyricsBody: $('#lyrics-body'),
  loading: $('#loading'),
  errorToast: $('#error-toast'),
  errorMessage: $('#error-message'),
  errorClose: $('#error-close'),
  backBtn: $('#back-btn'),
  backToResultsBtn: $('#back-to-results-btn'),
  searchHeader: $('#search-header'),
};

// ==================== State ====================

let currentSearchResults = null;
let currentLyrics = null;
let currentSearchQuery = null;
let currentSearchRunId = 0;
let isLoadingMoreResults = false;
let resultsScrollObserver = null;
let resultsScrollEventsInitialized = false;

// 当前视图状态：'search' | 'results' | 'lyrics'
let currentView = 'search';

// 导航标志：区分前进(用户操作)和后退(popstate)
let isNavigatingBack = false;

// ==================== UI Helpers ====================

function show(el) { el.classList.remove('hidden'); }
function hide(el) { el.classList.add('hidden'); }

function showLoading() { show(elements.loading); }
function hideLoading() { hide(elements.loading); }

function showError(msg) {
  elements.errorMessage.textContent = msg;
  show(elements.errorToast);
  setTimeout(() => hide(elements.errorToast), 5000);
}

// 内部切换视图（不pushState，用于返回按钮）
function switchToSearch() {
  show(elements.searchHeader);
  hide(elements.resultList);
  hide(elements.lyricsView);
  currentView = 'search';
}

function switchToResults() {
  hide(elements.searchHeader);
  show(elements.resultList);
  hide(elements.lyricsView);
  currentView = 'results';
}

function switchToLyrics() {
  hide(elements.searchHeader);
  hide(elements.resultList);
  show(elements.lyricsView);
  currentView = 'lyrics';
}

// 用户操作切换视图（pushState，用于前进导航）
function showSearch() {
  switchToSearch();
  if (!isNavigatingBack) {
    history.pushState({ view: 'search' }, '', '');
  }
}

function showResults() {
  switchToResults();
  if (!isNavigatingBack) {
    history.pushState({ view: 'results' }, '', '');
  }
}

function showLyrics() {
  switchToLyrics();
  if (!isNavigatingBack) {
    history.pushState({ view: 'lyrics' }, '', '');
  }
}

// ==================== Persistent Settings ====================

const STORAGE_KEY = 'utabuild-settings';
const VALID_FONT_SIZES = new Set(['small', 'medium', 'large']);
const VALID_DARK_MODES = new Set(['on', 'off']);
const DEFAULT_USE_CACHE = true;

function normalizeSettings(rawSettings = {}) {
  const settings = {};

  if (VALID_FONT_SIZES.has(rawSettings.fontSize)) {
    settings.fontSize = rawSettings.fontSize;
  }

  if (VALID_DARK_MODES.has(rawSettings.darkMode)) {
    settings.darkMode = rawSettings.darkMode;
  }

  if (typeof rawSettings.useCache === 'boolean') {
    settings.useCache = rawSettings.useCache;
  }

  return settings;
}

function loadSettings() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (!saved) {
      return {};
    }

    const parsed = JSON.parse(saved);
    const normalized = normalizeSettings(parsed);

    if (JSON.stringify(parsed) !== JSON.stringify(normalized)) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(normalized));
    }

    return normalized;
  } catch {
    return {};
  }
}

function saveSettings(settings) {
  try {
    const current = loadSettings();
    const merged = normalizeSettings({ ...current, ...settings });
    localStorage.setItem(STORAGE_KEY, JSON.stringify(merged));
  } catch (e) {
    console.warn('Failed to save settings:', e);
  }
}

function shouldUseCache() {
  const settings = loadSettings();
  return settings.useCache ?? DEFAULT_USE_CACHE;
}

async function clearAllCaches() {
  showLoading();

  try {
    await invoke('clear_cache');
    currentSearchResults = null;
    currentLyrics = null;
    currentSearchQuery = null;
    isLoadingMoreResults = false;
    showError('缓存已清除');
  } catch (err) {
    console.error('Clear cache error:', err);
    showError(`清除缓存失败: ${err}`);
  } finally {
    hideLoading();
  }
}

// ==================== Ruby Rendering (utaten CSS方案) ====================

/**
 * 将LyricElement数组渲染为带Ruby的DOM
 * 
 * 数据格式：
 * { type: "text", base: "文字" }
 * { type: "ruby", base: "漢字", ruby: "ふりがな" }
 * { type: "linebreak" }
 */
function renderLyrics(elements) {
  const wrapper = document.createElement('div');
  wrapper.className = 'lyricBody';
  
  const inner = document.createElement('div');
  // 从localStorage读取保存的字号，默认medium
  const settings = loadSettings();
  inner.className = settings.fontSize || 'medium';
  
  let currentLine = document.createElement('div');
  
  for (const el of elements) {
    switch (el.type) {
      case 'text': {
        currentLine.appendChild(document.createTextNode(el.base ?? ''));
        break;
      }
      
      case 'ruby': {
        // utaten结构: span.ruby > span.rb + span.rt
        const ruby = document.createElement('span');
        ruby.className = 'ruby';
        
        const rb = document.createElement('span');
        rb.className = 'rb';
        rb.textContent = el.base ?? '';
        
        const rt = document.createElement('span');
        rt.className = 'rt';
        rt.textContent = el.ruby ?? '';
        
        ruby.appendChild(rb);
        ruby.appendChild(rt);
        currentLine.appendChild(ruby);
        break;
      }
      
      case 'linebreak': {
        inner.appendChild(currentLine);
        currentLine = document.createElement('div');
        break;
      }
    }
  }
  
  // 添加最后一行
  if (currentLine.childNodes.length > 0) {
    inner.appendChild(currentLine);
  }
  
  wrapper.appendChild(inner);
  return wrapper;
}

// ==================== Event Handlers ====================

function getPaginationInfo(result) {
  const currentPage = result?.pagination?.current_page ?? result?.page ?? 1;
  const totalPages = result?.pagination?.total_pages ?? currentPage;
  const hasNext = result?.pagination?.has_next ?? (currentPage < totalPages);
  const loadedPages = result?.pagination?.loaded_pages ?? currentPage;
  const loadingMore = Boolean(result?.pagination?.loading_more);
  return { currentPage, totalPages, hasNext, loadedPages, loadingMore };
}

function updatePagination(result) {
  const { totalPages, loadedPages, hasNext, loadingMore } = getPaginationInfo(result);
  const showPagination = totalPages > 1;

  if (showPagination) {
    elements.paginationInfo.textContent = loadingMore
      ? `${loadedPages}/${totalPages}ページ読み込み済み · 続きを読み込み中...`
      : hasNext
        ? `${loadedPages}/${totalPages}ページ読み込み済み · 下にスクロールして続きを読み込む`
        : `${loadedPages}/${totalPages}ページを読み込み済み`;
  } else {
    elements.paginationInfo.textContent = '';
  }
  elements.resultsPagination.classList.toggle('hidden', !showPagination);
  syncInfiniteScrollObserver();
}

function updateResultsSummary(result) {
  const resultCount = result?.results?.length ?? 0;
  const { loadedPages, totalPages, loadingMore } = getPaginationInfo(result);
  const title = result?.query_title ?? currentSearchQuery?.title ?? '';
  const artist = result?.query_artist ?? currentSearchQuery?.artist;
  const queryLabel = artist ? `${title} / ${artist}` : title;

  if (!queryLabel) {
    hide(elements.resultsSummary);
    return;
  }

  const loadingSuffix = loadingMore ? '・続きを取得中' : '';
  elements.resultsSummary.textContent = `「${queryLabel}」の検索結果 ${resultCount}件（${loadedPages}/${totalPages}ページ読み込み済み${loadingSuffix}）`;
  show(elements.resultsSummary);
}

function getResultItemKey(item) {
  return item?.url || `${item?.title || ''}::${item?.artist || ''}`;
}

function mergeSearchResults(existingResult, nextResult, { loadingMore = false } = {}) {
  const existingItems = existingResult?.results ?? [];
  const nextItems = nextResult?.results ?? [];
  const mergedItems = [...existingItems];
  const seenKeys = new Set(existingItems.map(getResultItemKey));

  nextItems.forEach((item) => {
    const key = getResultItemKey(item);
    if (!seenKeys.has(key)) {
      seenKeys.add(key);
      mergedItems.push(item);
    }
  });

  const totalPages =
    nextResult?.pagination?.total_pages ??
    existingResult?.pagination?.total_pages ??
    nextResult?.page ??
    existingResult?.page ??
    1;
  const loadedPages = Math.max(
    existingResult?.pagination?.loaded_pages ?? existingResult?.pagination?.current_page ?? existingResult?.page ?? 1,
    nextResult?.pagination?.loaded_pages ?? nextResult?.pagination?.current_page ?? nextResult?.page ?? 1,
  );

  return {
    ...existingResult,
    ...nextResult,
    results: mergedItems,
    query_title: nextResult?.query_title ?? existingResult?.query_title,
    query_artist: nextResult?.query_artist ?? existingResult?.query_artist,
    pagination: {
      ...(existingResult?.pagination ?? {}),
      ...(nextResult?.pagination ?? {}),
      current_page: nextResult?.pagination?.current_page ?? nextResult?.page ?? loadedPages,
      total_pages: totalPages,
      has_next: nextResult?.pagination?.has_next ?? (loadedPages < totalPages),
      loaded_pages: loadedPages,
      loading_more: loadingMore,
    },
  };
}

async function loadRemainingSearchPages(searchRunId) {
  if (
    currentSearchRunId !== searchRunId ||
    !currentSearchQuery ||
    !currentSearchResults ||
    isLoadingMoreResults
  ) {
    return;
  }

  const { currentPage, totalPages, hasNext } = getPaginationInfo(currentSearchResults);
  if (!hasNext || currentPage >= totalPages) {
    currentSearchResults = mergeSearchResults(currentSearchResults, currentSearchResults, { loadingMore: false });
    renderResultList(currentSearchResults);
    return;
  }

  const nextPage = currentPage + 1;
  isLoadingMoreResults = true;
  currentSearchResults = mergeSearchResults(currentSearchResults, currentSearchResults, { loadingMore: true });
  renderResultList(currentSearchResults);

  try {
    const nextResult = await invoke('search_lyrics', {
      title: currentSearchQuery.title,
      artist: currentSearchQuery.artist ?? null,
      page: nextPage,
      useCache: shouldUseCache(),
    });

    if (currentSearchRunId !== searchRunId) {
      return;
    }

    if (nextResult.status !== 'select' || !Array.isArray(nextResult.results)) {
      throw new Error(nextResult.error || `ページ ${nextPage} の取得に失敗しました`);
    }

    currentSearchResults = mergeSearchResults(currentSearchResults, nextResult, { loadingMore: false });
    renderResultList(currentSearchResults);
  } catch (err) {
    if (currentSearchRunId !== searchRunId) {
      return;
    }

    currentSearchResults = mergeSearchResults(currentSearchResults, currentSearchResults, { loadingMore: false });
    renderResultList(currentSearchResults);
    console.error('Load more search results error:', err);
    const message = err instanceof Error ? err.message : String(err);
    showError(`続きの検索結果の取得に失敗しました: ${message}`);
  } finally {
    isLoadingMoreResults = false;
    maybeLoadMoreResults();
  }
}

function maybeLoadMoreResults() {
  if (
    currentView !== 'results' ||
    !currentSearchResults ||
    isLoadingMoreResults ||
    elements.resultsPagination.classList.contains('hidden')
  ) {
    return;
  }

  const rect = elements.resultsPagination.getBoundingClientRect();
  const viewportHeight = window.innerHeight || document.documentElement?.clientHeight || 0;

  if (rect.top <= viewportHeight + 160) {
    void loadRemainingSearchPages(currentSearchRunId);
  }
}

function syncInfiniteScrollObserver() {
  if (!resultsScrollObserver) {
    return;
  }

  resultsScrollObserver.disconnect();

  if (!elements.resultsPagination.classList.contains('hidden')) {
    resultsScrollObserver.observe(elements.resultsPagination);
  }
}

function initInfiniteScroll() {
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

async function performSearch(page = 1, searchRunId = currentSearchRunId) {
  const title = currentSearchQuery?.title;
  const artist = currentSearchQuery?.artist ?? null;
  
  if (!title) {
    showError('曲名を入力してください');
    return;
  }
  
  showLoading();
  
  console.log('🔍 搜索:', title, '| isTauriEnv:', isTauriEnv, '| invoke:', typeof invoke);
  
  try {
    const result = await invoke('search_lyrics', {
      title,
      artist,
      page,
      useCache: shouldUseCache(),
    });

    if (searchRunId !== currentSearchRunId) {
      return;
    }
    
    currentSearchResults = mergeSearchResults(null, result, { loadingMore: false });
    
    if (result.status === 'select' && result.results && result.results.length > 0) {
      renderResultList(currentSearchResults);
      if (currentView === 'results') {
        switchToResults();
      } else {
        showResults();
      }
      maybeLoadMoreResults();
    } else if (result.status === 'not_found') {
      showError('結果が見つかりませんでした');
    } else {
      showError(result.error || '検索に失敗しました');
    }
  } catch (err) {
    console.error('Search error:', err);
    showError(`検索エラー: ${err}`);
  } finally {
    hideLoading();
  }
}

// 搜索
async function handleSearch() {
  const title = elements.searchTitle.value.trim();
  const artist = elements.searchArtist.value.trim() || null;
  
  currentSearchQuery = { title, artist };
  currentSearchRunId += 1;
  isLoadingMoreResults = false;
  await performSearch(1, currentSearchRunId);
}

// 渲染搜索结果列表
function renderResultList(result) {
  elements.resultsContainer.innerHTML = '';
  updateResultsSummary(result);
  updatePagination(result);
  
  result.results.forEach((item, index) => {
    const div = document.createElement('div');
    div.className = 'result-item';
    div.innerHTML = `
      <div class="title">${escapeHtml(item.title)}</div>
      <div class="artist">${escapeHtml(item.artist)}</div>
    `;
    div.addEventListener('click', () => handleSelectResult(index));
    elements.resultsContainer.appendChild(div);
  });
}

// 选择搜索结果，获取歌词
async function handleSelectResult(index) {
  showLoading();
  
  try {
    // 按CLI逻辑：搜索 → 选择 → get_lyrics(传URL)
    const selectedItem = currentSearchResults.results[index];
    const result = await invoke('get_lyrics', {
      url: selectedItem.url,
      title: selectedItem.title,
      artist: selectedItem.artist || null,
      useCache: shouldUseCache(),
    });
    
    currentLyrics = result;
    
    if (result.status === 'success') {
      elements.lyricsTitle.textContent = result.found_title;
      elements.lyricsArtist.textContent = result.found_artist;
      
      // 渲染歌词
      elements.lyricsBody.innerHTML = '';
      const lyricsEl = renderLyrics(result.ruby_annotations);
      elements.lyricsBody.appendChild(lyricsEl);
      
      // 同步按钮状态
      updateButtonStates();
      
      showLyrics();
    } else {
      showError(result.error || '歌詞の取得に失敗しました');
    }
  } catch (err) {
    console.error('Select error:', err);
    showError(`エラー: ${err}`);
  } finally {
    hideLoading();
  }
}

// 更新按钮高亮状态
function updateButtonStates() {
  const settings = loadSettings();
  
  // 字号按钮
  const fontSize = settings.fontSize || 'medium';
  $$('[data-size]').forEach(b => b.classList.remove('active'));
  const sizeBtn = document.querySelector(`[data-size="${fontSize}"]`);
  if (sizeBtn) sizeBtn.classList.add('active');
  
  // 暗色模式按钮
  const darkMode = settings.darkMode || 'off';
  $$('[data-dark]').forEach(b => b.classList.remove('active'));
  const darkBtn = document.querySelector(`[data-dark="${darkMode}"]`);
  if (darkBtn) darkBtn.classList.add('active');
}

// ==================== Lyrics Controls ====================

function initControls() {
  if (elements.settingUseCache) {
    elements.settingUseCache.checked = shouldUseCache();
    elements.settingUseCache.addEventListener('change', (event) => {
      saveSettings({ useCache: event.target.checked });
    });
  }

  if (elements.settingClearCache) {
    elements.settingClearCache.addEventListener('click', () => {
      void clearAllCaches();
    });
  }

  // 字号控制
  $$('[data-size]').forEach(btn => {
    btn.addEventListener('click', () => {
      const size = btn.dataset.size;
      const body = elements.lyricsBody.querySelector('.lyricBody > div');
      if (body) {
        body.className = size;
      }
      // 保存到localStorage
      saveSettings({ fontSize: size });
      // 更新按钮状态
      updateButtonStates();
    });
  });
  
  // 暗色模式
  $$('[data-dark]').forEach(btn => {
    btn.addEventListener('click', () => {
      const mode = btn.dataset.dark;
      document.body.classList.toggle('dark-mode', mode === 'on');
      
      // 保存到localStorage
      saveSettings({ darkMode: mode });
      updateButtonStates();
    });
  });
}

// ==================== Android Back Button ====================

function initBackButton() {
  // 监听popstate（浏览器/Android返回按钮触发）
  window.addEventListener('popstate', (event) => {
    // 检查是否是我们管理的状态
    if (event.state && event.state.view) {
      isNavigatingBack = true;
      
      // 根据历史记录中的状态切换视图（不pushState）
      switch (event.state.view) {
        case 'search':
          switchToSearch();
          break;
        case 'results':
          switchToResults();
          break;
        case 'lyrics':
          switchToLyrics();
          break;
      }
      
      isNavigatingBack = false;
    } else {
      // event.state为null表示到达历史栈底部，让浏览器/系统处理退出
      // 在Android上这会退出应用
    }
  });
  
  // 初始push一个state
  history.replaceState({ view: 'search' }, '', '');
}

function handleBack() {
  // 手动触发返回（调用浏览器的history.back，触发popstate）
  history.back();
}

// ==================== Utils ====================

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// ==================== Init ====================

function init() {
  // 搜索按钮
  elements.searchBtn.addEventListener('click', handleSearch);
  
  // 回车搜索
  elements.searchTitle.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') handleSearch();
  });
  elements.searchArtist.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') handleSearch();
  });
  
  // 返回按钮（手动点击）
  elements.backBtn.addEventListener('click', () => {
    handleBack();
  });
  elements.backToResultsBtn.addEventListener('click', () => {
    handleBack();
  });
  
  // 错误关闭
  elements.errorClose.addEventListener('click', () => hide(elements.errorToast));
  
  // 控制按钮
  initControls();

  // 结果页无限滚动
  initInfiniteScroll();
  
  // Android返回按钮支持
  initBackButton();
  
  // 应用保存的暗色模式
  const settings = loadSettings();
  if (settings.darkMode === 'on') {
    document.body.classList.add('dark-mode');
  }
  
  // 同步按钮状态
  updateButtonStates();
  
  // 浏览器模式提示
  if (!isTauriEnv) {
    console.log('🦊 UtaBuild Browser Mode - 使用Mock数据调试UI');
  }
  console.log('UtaBuild initialized');
}

// 启动
document.addEventListener('DOMContentLoaded', init);
