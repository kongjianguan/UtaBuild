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

    if (cmd === 'take_salt_launch_request') {
      return null;
    }

    if (cmd === 'bind_salt_song_lyrics') {
      console.log('[Mock] bind Salt song lyrics', args);
      return null;
    }

    if (cmd === 'list_saved_lyrics') {
      const songs = [
        { title: 'FIRE BIRD', artist: 'Roselia', album: 'Wahl', cover_url: 'https://placehold.co/160x160/2fd3ff/0b1220?text=FB', lyrics_url: '/lyric/mock1', annotation_count: 18 },
        { title: 'BLACK SHOUT', artist: 'Roselia', album: 'Für immer', cover_url: '', lyrics_url: '/lyric/mock2', annotation_count: 12 },
        { title: 'キズナミュージック♪', artist: "Poppin'Party", album: 'Breakthrough!', cover_url: 'https://placehold.co/160x160/f6c546/0b1220?text=KM', lyrics_url: '/lyric/mock3', annotation_count: 16 },
      ].sort((a, b) => String(a[args?.sortBy || 'title'] || '').localeCompare(String(b[args?.sortBy || 'title'] || ''), 'ja'));
      return { status: 'success', sort_by: args?.sortBy || 'title', songs };
    }

    if (cmd === 'hydrate_saved_lyrics_metadata') {
      return {
        status: 'success',
        lyrics_url: args.url,
        album: args.url === '/lyric/mock2' ? 'Für immer' : '',
        cover_url: args.url === '/lyric/mock2' ? 'https://placehold.co/160x160/111827/d1d5db?text=BS' : '',
      };
    }

    if (cmd === 'delete_saved_lyrics') {
      return true;
    }

    if (cmd === 'get_saved_lyrics') {
      return {
        status: 'success',
        found_title: 'FIRE BIRD',
        found_artist: 'Roselia',
        lyrics_url: args.url,
        ruby_annotations: [
          { type: 'text', base: 'Lala lalala lala lalala' },
          { type: 'linebreak' },
          { type: 'ruby', base: '飛', ruby: 'と' },
          { type: 'text', base: 'べ FIRE BIRD' },
        ],
      };
    }

    if (cmd === 'clear_cache') {
      return null;
    }

    if (cmd === 'set_lsp_logging_enabled') {
      console.log('[Mock] set lsp logging', args);
      return null;
    }

    if (cmd === 'append_lsp_log') {
      console.log('[Mock] lsp log', args);
      return null;
    }

    if (cmd === 'get_lsp_logs') {
      return '[Mock] 暂无lsp日志。';
    }
    
    return null;
  };
}

// 立即初始化
const tauriReady = initTauri();

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
  settingLspLog: $('#setting-lsp-log'),
  lspLogPanel: $('#lsp-log-panel'),
  settingViewLspLog: $('#setting-view-lsp-log'),
  lspLogView: $('#lsp-log-view'),
  lspLogBackBtn: $('#lsp-log-back-btn'),
  lspLogRefreshBtn: $('#lsp-log-refresh-btn'),
  lspLogZoomOut: $('#lsp-log-zoom-out'),
  lspLogZoomIn: $('#lsp-log-zoom-in'),
  lspLogZoomLabel: $('#lsp-log-zoom-label'),
  lspLogContent: $('#lsp-log-content'),
  settingsView: $('#settings-view'),
  songsView: $('#songs-view'),
  songsList: $('#songs-list'),
  songsEmpty: $('#songs-empty'),
  searchHistoryList: $('#search-history-list'),
  searchHistoryEmpty: $('#search-history-empty'),
  bottomMenu: $('#bottom-menu'),
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
let pendingSaltRequest = null;
let lspLogZoom = 1;
let firstLevelAnimationTimer = null;
let songsSortBy = 'title';
let songLongPressTimer = null;
let songLongPressTriggered = false;
let activeSongContextMenu = null;
let activeSongContextItem = null;
const hydratingSongMetadataUrls = new Set();
const viewScrollPositions = new Map();
let isBottomMenuAutoHidden = false;
let lastSongsScrollY = 0;

// 当前视图状态：'search' | 'songs' | 'settings' | 'lspLogs' | 'results' | 'lyrics'
let currentView = 'search';

// 导航标志：区分前进(用户操作)和后退(popstate)
let isNavigatingBack = false;

// ==================== UI Helpers ====================

function show(el) { el.classList.remove('hidden'); }
function hide(el) { el.classList.add('hidden'); }

function currentPageScrollY() {
  return window.scrollY || document.documentElement.scrollTop || document.body.scrollTop || 0;
}

function scrollPageTo(y) {
  const top = Math.max(0, Math.round(Number(y) || 0));
  document.documentElement.scrollTop = top;
  document.body.scrollTop = top;
  window.scrollTo({ top, left: 0, behavior: 'auto' });
}

function scrollPageToTop() {
  scrollPageTo(0);
}

function repeatScrollTo(y) {
  scrollPageTo(y);
  requestAnimationFrame(() => scrollPageTo(y));
  setTimeout(() => scrollPageTo(y), 80);
}

function resetViewportToTop() {
  repeatScrollTo(0);
}

function saveCurrentScrollPosition() {
  if (!currentView) {
    return;
  }
  viewScrollPositions.set(currentView, currentPageScrollY());
}

function restoreViewScrollPosition(view) {
  repeatScrollTo(viewScrollPositions.get(view) || 0);
}

function showLoading() { show(elements.loading); }
function hideLoading() { hide(elements.loading); }

function showError(msg) {
  elements.errorMessage.textContent = msg;
  show(elements.errorToast);
  setTimeout(() => hide(elements.errorToast), 5000);
}

function syncBottomMenu(activeTab) {
  if (!elements.bottomMenu) {
    return;
  }

  elements.bottomMenu.dataset.activeTab = activeTab;
  $$('[data-app-tab]').forEach((button) => {
    const isActive = button.dataset.appTab === activeTab;
    button.classList.toggle('active', isActive);
    button.setAttribute('aria-selected', String(isActive));
  });
}

function setBottomMenuAutoHidden(isHidden) {
  if (!elements.bottomMenu) {
    return;
  }

  isBottomMenuAutoHidden = Boolean(isHidden);
  elements.bottomMenu.classList.toggle('is-auto-hidden', isBottomMenuAutoHidden);
}

function setBottomMenuVisible(isVisible, activeTab = 'search') {
  if (!elements.bottomMenu) {
    return;
  }

  elements.bottomMenu.classList.toggle('hidden', !isVisible);
  elements.bottomMenu.setAttribute('aria-hidden', String(!isVisible));
  document.body.classList.toggle('has-bottom-menu', isVisible);

  if (!isVisible) {
    setBottomMenuAutoHidden(false);
    return;
  }

  syncBottomMenu(activeTab);
  if (activeTab !== 'songs') {
    setBottomMenuAutoHidden(false);
  }
}

function firstLevelIndex(view) {
  return { search: 0, songs: 1, settings: 2 }[view] ?? 0;
}

function firstLevelElement(view) {
  if (view === 'settings') return elements.settingsView;
  if (view === 'songs') return elements.songsView;
  return elements.searchHeader;
}

function animateFirstLevelEntry(view, direction) {
  const target = firstLevelElement(view);
  if (!target || window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    return;
  }

  clearTimeout(firstLevelAnimationTimer);
  [elements.searchHeader, elements.songsView, elements.settingsView].forEach((element) => {
    element?.classList.remove('first-level-slide-from-left', 'first-level-slide-from-right');
  });

  const className = direction === 'back'
    ? 'first-level-slide-from-left'
    : 'first-level-slide-from-right';
  target.classList.add(className);

  firstLevelAnimationTimer = setTimeout(() => {
    target.classList.remove(className);
  }, 420);
}

// 内部切换视图（不pushState，用于返回按钮）
function switchToSearch(options = {}) {
  show(elements.searchHeader);
  hide(elements.songsView);
  hide(elements.settingsView);
  hide(elements.lspLogView);
  hide(elements.resultList);
  hide(elements.lyricsView);
  currentView = 'search';
  setBottomMenuVisible(true, 'search');
  renderSearchHistory();
  if (options.animate) {
    animateFirstLevelEntry('search', options.direction);
  }
}

function switchToSettings(options = {}) {
  hide(elements.searchHeader);
  hide(elements.songsView);
  show(elements.settingsView);
  hide(elements.lspLogView);
  hide(elements.resultList);
  hide(elements.lyricsView);
  currentView = 'settings';
  setBottomMenuVisible(true, 'settings');
  if (options.animate) {
    animateFirstLevelEntry('settings', options.direction);
  }
}


function switchToSongs(options = {}) {
  hide(elements.searchHeader);
  show(elements.songsView);
  hide(elements.settingsView);
  hide(elements.lspLogView);
  hide(elements.resultList);
  hide(elements.lyricsView);
  currentView = 'songs';
  setBottomMenuVisible(true, 'songs');
  lastSongsScrollY = currentPageScrollY();
  if (lastSongsScrollY <= 24) {
    setBottomMenuAutoHidden(false);
  }
  if (options.animate) {
    animateFirstLevelEntry('songs', options.direction);
  }
}

function switchToResults() {
  hide(elements.searchHeader);
  hide(elements.songsView);
  hide(elements.settingsView);
  hide(elements.lspLogView);
  show(elements.resultList);
  hide(elements.lyricsView);
  currentView = 'results';
  setBottomMenuVisible(false);
}

function switchToLyrics(options = {}) {
  hide(elements.searchHeader);
  hide(elements.songsView);
  hide(elements.settingsView);
  hide(elements.lspLogView);
  hide(elements.resultList);
  show(elements.lyricsView);
  currentView = 'lyrics';
  setBottomMenuVisible(false);
  if (options.resetScroll !== false) {
    resetViewportToTop();
  }
}

function switchToLspLogs() {
  hide(elements.searchHeader);
  hide(elements.songsView);
  hide(elements.settingsView);
  show(elements.lspLogView);
  hide(elements.resultList);
  hide(elements.lyricsView);
  currentView = 'lspLogs';
  setBottomMenuVisible(false);
}

// 用户操作切换视图（pushState，用于前进导航）
function showSearch() {
  saveCurrentScrollPosition();
  const previousView = currentView;
  const shouldAnimate = ['songs', 'settings'].includes(previousView) && !isNavigatingBack;
  switchToSearch({ animate: shouldAnimate, direction: firstLevelIndex(previousView) > firstLevelIndex('search') ? 'back' : 'forward' });
  if (!isNavigatingBack) {
    history.pushState({ view: 'search' }, '', '');
  }
}

function showSettings() {
  saveCurrentScrollPosition();
  const previousView = currentView;
  const shouldAnimate = ['search', 'songs'].includes(previousView) && !isNavigatingBack;
  switchToSettings({ animate: shouldAnimate, direction: firstLevelIndex(previousView) < firstLevelIndex('settings') ? 'forward' : 'back' });
  if (!isNavigatingBack) {
    history.pushState({ view: 'settings' }, '', '');
  }
}


function showSongs() {
  saveCurrentScrollPosition();
  const previousView = currentView;
  const shouldAnimate = ['search', 'settings'].includes(previousView) && !isNavigatingBack;
  switchToSongs({ animate: shouldAnimate, direction: firstLevelIndex(previousView) < firstLevelIndex('songs') ? 'forward' : 'back' });
  if (!isNavigatingBack) {
    history.pushState({ view: 'songs' }, '', '');
  }
  void loadSavedLyrics();
}

function showLspLogs() {
  saveCurrentScrollPosition();
  switchToLspLogs();
  if (!isNavigatingBack) {
    history.pushState({ view: 'lspLogs' }, '', '');
  }
  void viewLspLogs();
}

function showResults() {
  saveCurrentScrollPosition();
  switchToResults();
  if (!isNavigatingBack) {
    history.pushState({ view: 'results' }, '', '');
  }
}

function showLyrics() {
  saveCurrentScrollPosition();
  switchToLyrics({ resetScroll: true });
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

  if (typeof rawSettings.lspLogEnabled === 'boolean') {
    settings.lspLogEnabled = rawSettings.lspLogEnabled;
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

// ==================== Search History ====================

const SEARCH_HISTORY_KEY = 'utabuild-search-history';
const SEARCH_HISTORY_LIMIT = 300;

function normalizeHistoryEntry(entry) {
  if (!entry || typeof entry.title !== 'string') {
    return null;
  }

  const title = entry.title.trim();
  if (!title) {
    return null;
  }

  const artist = typeof entry.artist === 'string' && entry.artist.trim()
    ? entry.artist.trim()
    : null;
  const searchedAt = Number.isFinite(entry.searchedAt)
    ? entry.searchedAt
    : Date.now();

  return { title, artist, searchedAt };
}

function loadSearchHistory() {
  try {
    const saved = localStorage.getItem(SEARCH_HISTORY_KEY);
    if (!saved) {
      return [];
    }

    const parsed = JSON.parse(saved);
    if (!Array.isArray(parsed)) {
      return [];
    }

    const normalized = parsed
      .map(normalizeHistoryEntry)
      .filter(Boolean)
      .sort((a, b) => b.searchedAt - a.searchedAt)
      .slice(0, SEARCH_HISTORY_LIMIT);

    if (normalized.length !== parsed.length) {
      saveSearchHistory(normalized);
    }

    return normalized;
  } catch {
    return [];
  }
}

function saveSearchHistory(historyItems) {
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

function historyKey(title, artist) {
  return `${title.trim().toLocaleLowerCase()}\u0000${(artist || '').trim().toLocaleLowerCase()}`;
}

function addSearchHistory(title, artist) {
  const entry = normalizeHistoryEntry({
    title,
    artist,
    searchedAt: Date.now(),
  });

  if (!entry) {
    return;
  }

  const newKey = historyKey(entry.title, entry.artist);
  const existing = loadSearchHistory().filter(
    (item) => historyKey(item.title, item.artist) !== newKey,
  );
  saveSearchHistory([entry, ...existing].slice(0, SEARCH_HISTORY_LIMIT));
  renderSearchHistory();
}

function formatHistoryTime(timestamp) {
  if (!Number.isFinite(timestamp)) {
    return '';
  }

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

function fillSearchFromHistory(entry) {
  elements.searchTitle.value = '';
  elements.searchArtist.value = '';
  elements.searchTitle.value = entry.title;
  elements.searchArtist.value = entry.artist || '';
  elements.searchTitle.focus();
}

function renderSearchHistory() {
  if (!elements.searchHistoryList || !elements.searchHistoryEmpty) {
    return;
  }

  const historyItems = loadSearchHistory();
  elements.searchHistoryList.innerHTML = '';
  elements.searchHistoryEmpty.classList.toggle('hidden', historyItems.length > 0);

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
    elements.searchHistoryList.appendChild(button);
  });
}


// ==================== Saved Songs ====================

function formatSongSubtitle(song) {
  const artist = song.artist || '未知歌手';
  return song.album ? `${artist} - ${song.album}` : artist;
}

function normalizeCoverUrl(value) {
  if (!value || typeof value !== 'string') {
    return '';
  }

  const trimmed = value.trim();
  if (!trimmed) {
    return '';
  }

  try {
    const url = new URL(trimmed, window.location.href);
    return ['http:', 'https:'].includes(url.protocol) ? url.href : '';
  } catch (_err) {
    return '';
  }
}

function applySongCoverArt(artEl, coverUrl) {
  const normalized = normalizeCoverUrl(coverUrl);
  artEl.classList.toggle('has-cover', Boolean(normalized));
  artEl.style.backgroundImage = normalized ? `url("${normalized}")` : '';
}

function buildSongItem(song) {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'song-item';
  button.dataset.lyricsUrl = song.lyrics_url || '';

  const art = document.createElement('span');
  art.className = 'song-item__art';
  art.setAttribute('aria-hidden', 'true');
  applySongCoverArt(art, song.cover_url);

  const body = document.createElement('span');
  body.className = 'song-item__body';

  const title = document.createElement('span');
  title.className = 'song-item__title';
  title.textContent = song.title || '未命名歌曲';

  const meta = document.createElement('span');
  meta.className = 'song-item__meta';
  meta.textContent = formatSongSubtitle(song);

  body.append(title, meta);
  button.append(art, body);
  return button;
}

function updateRenderedSongMetadata(metadata) {
  if (!metadata?.lyrics_url) {
    return;
  }

  const item = Array.from(elements.songsList?.querySelectorAll('.song-item') || [])
    .find((candidate) => candidate.dataset.lyricsUrl === metadata.lyrics_url);
  if (!item) {
    return;
  }

  const art = item.querySelector('.song-item__art');
  if (art) {
    applySongCoverArt(art, metadata.cover_url);
  }

  if (metadata.album) {
    const meta = item.querySelector('.song-item__meta');
    if (meta) {
      const artist = item.__songArtist || '未知歌手';
      meta.textContent = `${artist} - ${metadata.album}`;
    }
  }
}

async function hydrateMissingSongMetadata(songs) {
  const missing = songs.filter((song) => song.lyrics_url && !normalizeCoverUrl(song.cover_url));

  for (const song of missing) {
    if (hydratingSongMetadataUrls.has(song.lyrics_url)) {
      continue;
    }

    hydratingSongMetadataUrls.add(song.lyrics_url);
    try {
      const metadata = await invoke('hydrate_saved_lyrics_metadata', { url: song.lyrics_url });
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

function closeSongContextMenu() {
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

function positionSongContextMenu(menu, clientX, clientY) {
  const margin = 12;
  const rect = menu.getBoundingClientRect();
  const x = Math.min(Math.max(clientX, margin), window.innerWidth - rect.width - margin);
  const y = Math.min(Math.max(clientY, margin), window.innerHeight - rect.height - margin);
  menu.style.left = `${x}px`;
  menu.style.top = `${y}px`;
}

async function deleteSavedSong(song) {
  if (!song?.lyrics_url) {
    showError('已保存歌词缺少URL');
    return;
  }

  const label = song.title || '这首歌曲';
  if (!window.confirm(`删除「${label}」的已保存歌词？`)) {
    return;
  }

  showLoading();
  try {
    await invoke('delete_saved_lyrics', { url: song.lyrics_url });
    showError('已删除已保存歌词');
    await loadSavedLyrics();
  } catch (err) {
    console.error('Delete saved lyrics error:', err);
    showError(`删除失败: ${err}`);
  } finally {
    hideLoading();
  }
}

function showSongContextMenu(song, trigger, event) {
  closeSongContextMenu();

  const menu = document.createElement('div');
  menu.className = 'long-press-menu';
  menu.setAttribute('role', 'menu');
  menu.innerHTML = `
    <button class="long-press-menu__item long-press-menu__item--danger" type="button" role="menuitem">删除</button>
  `;

  menu.querySelector('button').addEventListener('click', async () => {
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

  requestAnimationFrame(() => menu.classList.add('is-visible'));
}

function attachSongLongPressMenu(button, song) {
  button.addEventListener('pointerdown', (event) => {
    if (event.pointerType === 'mouse' && event.button !== 0) {
      return;
    }

    closeSongContextMenu();
    songLongPressTriggered = false;
    songLongPressTimer = setTimeout(() => {
      songLongPressTriggered = true;
      if (event.pointerType !== 'mouse') {
        button.setPointerCapture?.(event.pointerId);
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

function renderSavedLyrics(songs) {
  if (!elements.songsList || !elements.songsEmpty) {
    return;
  }

  closeSongContextMenu();
  elements.songsList.innerHTML = '';
  elements.songsEmpty.classList.toggle('hidden', songs.length > 0);

  songs.forEach((song) => {
    const button = buildSongItem(song);
    button.__songArtist = song.artist || '未知歌手';
    attachSongLongPressMenu(button, song);
    button.addEventListener('click', () => {
      if (songLongPressTriggered) {
        songLongPressTriggered = false;
        return;
      }
      void openSavedLyrics(song.lyrics_url);
    });
    elements.songsList.appendChild(button);
  });

  void hydrateMissingSongMetadata(songs);
}

async function loadSavedLyrics() {
  if (!elements.songsList || !elements.songsEmpty) {
    return;
  }

  elements.songsList.innerHTML = '';
  elements.songsEmpty.textContent = '正在读取已保存歌词...';
  elements.songsEmpty.classList.remove('hidden');

  try {
    const result = await invoke('list_saved_lyrics', { sortBy: songsSortBy });
    const songs = Array.isArray(result?.songs) ? result.songs : [];
    elements.songsEmpty.textContent = '暂无已保存歌词。搜索并打开歌词后会永久保存到这里。';
    renderSavedLyrics(songs);
  } catch (err) {
    console.error('Load saved lyrics error:', err);
    elements.songsEmpty.textContent = `读取已保存歌词失败: ${err}`;
  }
}

async function openSavedLyrics(url) {
  if (!url) {
    showError('已保存歌词缺少URL');
    return;
  }

  showLoading();
  try {
    const result = await invoke('get_saved_lyrics', { url });
    currentLyrics = result;
    if (result.status !== 'success') {
      showError(result.error || '读取已保存歌词失败');
      return;
    }

    elements.lyricsTitle.textContent = result.found_title;
    elements.lyricsArtist.textContent = result.found_artist;
    elements.lyricsBody.innerHTML = '';
    elements.lyricsBody.appendChild(renderLyrics(result.ruby_annotations));
    updateButtonStates();
    showLyrics();
  } catch (err) {
    console.error('Open saved lyrics error:', err);
    showError(`读取已保存歌词失败: ${err}`);
  } finally {
    hideLoading();
  }
}

function initSongsControls() {
  $$('[data-song-sort]').forEach((button) => {
    button.addEventListener('click', () => {
      songsSortBy = button.dataset.songSort === 'artist' ? 'artist' : 'title';
      $$('[data-song-sort]').forEach((item) => {
        item.classList.toggle('active', item.dataset.songSort === songsSortBy);
      });
      void loadSavedLyrics();
    });
  });

  document.addEventListener('click', (event) => {
    if (activeSongContextMenu && !activeSongContextMenu.contains(event.target)) {
      closeSongContextMenu();
    }
  });
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') {
      closeSongContextMenu();
    }
  });
  window.addEventListener('scroll', closeSongContextMenu, { passive: true });
  window.addEventListener('resize', closeSongContextMenu);
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
      void appendAppLspLog('search', `search success results=${result.results.length}`);
      renderResultList(currentSearchResults);
      if (currentView === 'results') {
        switchToResults();
      } else {
        showResults();
      }
      maybeLoadMoreResults();
    } else if (result.status === 'not_found') {
      void appendAppLspLog('search', 'search not_found');
      showError('結果が見つかりませんでした');
    } else {
      void appendAppLspLog('search', `search failed ${result.error || 'unknown error'}`);
      showError(result.error || '検索に失敗しました');
    }
  } catch (err) {
    console.error('Search error:', err);
    void appendAppLspLog('search', `search error ${String(err)}`);
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
  addSearchHistory(title, artist);
  void appendAppLspLog('ui', `search requested title="${title}" artist="${artist || ''}"`);
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
  const selectedItem = currentSearchResults.results[index];
  const saltRequest = pendingSaltRequest;

  if (saltRequest) {
    const confirmed = window.confirm(
      `Salt Player の「${saltRequest.title || ''}」に UtaBuild の「${selectedItem.title}」を紐付け、今後 Ruby 表示に使用しますか？`
    );
    if (!confirmed) {
      void appendAppLspLog('salt', `binding cancelled selected="${selectedItem.title}"`);
      return;
    }
  }

  showLoading();
  
  try {
    // 按CLI逻辑：搜索 → 选择 → get_lyrics(传URL)
    const result = await invoke('get_lyrics', {
      url: selectedItem.url,
      title: selectedItem.title,
      artist: selectedItem.artist || null,
      useCache: shouldUseCache(),
      saveSaltBridge: !saltRequest,
    });
    
    currentLyrics = result;
    
    if (result.status === 'success') {
      if (saltRequest) {
        await invoke('bind_salt_song_lyrics', {
          saltTitle: saltRequest.title || selectedItem.title,
          saltArtist: saltRequest.artist || null,
          lyrics: result,
        });
        pendingSaltRequest = null;
        void appendAppLspLog(
          'salt',
          `binding saved salt="${saltRequest.title || ''}" selected="${selectedItem.title}"`,
        );
      }

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
      void appendAppLspLog('lyrics', `get lyrics failed selected="${selectedItem.title}"`);
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

function syncLspLogVisibility() {
  if (!elements.settingLspLog || !elements.lspLogPanel) {
    return;
  }

  const enabled = loadSettings().lspLogEnabled === true;
  elements.settingLspLog.checked = enabled;
  elements.lspLogPanel.classList.toggle('hidden', !enabled);
  if (!enabled && elements.lspLogContent) {
    elements.lspLogContent.textContent = '';
    if (currentView === 'lspLogs') {
      switchToSettings();
    }
  }
}

async function setBackendLspLogging(enabled) {
  try {
    await tauriReady;
    await invoke('set_lsp_logging_enabled', { enabled });
  } catch (err) {
    console.warn('Failed to sync lsp logging setting:', err);
  }
}

async function appendAppLspLog(scope, message) {
  if (loadSettings().lspLogEnabled !== true) {
    return;
  }

  try {
    await tauriReady;
    await invoke('append_lsp_log', {
      scope,
      message,
    });
  } catch (err) {
    console.warn('Failed to append lsp log:', err);
  }
}

async function viewLspLogs() {
  if (!elements.lspLogContent) {
    return;
  }

  showLoading();
  elements.lspLogContent.textContent = '正在读取lsp日志...';
  void appendAppLspLog('settings', 'view lsp logs');

  try {
    await tauriReady;
    const logs = await invoke('get_lsp_logs');
    elements.lspLogContent.textContent = logs && String(logs).trim()
      ? String(logs)
      : '暂无lsp日志';
  } catch (err) {
    console.error('Read lsp logs error:', err);
    elements.lspLogContent.textContent = `读取lsp日志失败: ${err}`;
  } finally {
    hideLoading();
  }
}

function syncLspLogZoom() {
  if (!elements.lspLogContent || !elements.lspLogZoomLabel) {
    return;
  }

  elements.lspLogContent.style.setProperty('--lsp-log-font-scale', String(lspLogZoom));
  elements.lspLogZoomLabel.textContent = `${Math.round(lspLogZoom * 100)}%`;
}

function adjustLspLogZoom(delta) {
  lspLogZoom = Math.min(1.8, Math.max(0.7, Number((lspLogZoom + delta).toFixed(2))));
  syncLspLogZoom();
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

  if (elements.settingLspLog) {
    elements.settingLspLog.checked = loadSettings().lspLogEnabled === true;
    elements.settingLspLog.addEventListener('change', (event) => {
      const enabled = event.target.checked;
      saveSettings({ lspLogEnabled: enabled });
      syncLspLogVisibility();
      void setBackendLspLogging(enabled);
    });
    syncLspLogVisibility();
  }

  if (elements.settingViewLspLog) {
    elements.settingViewLspLog.addEventListener('click', () => {
      showLspLogs();
    });
  }

  if (elements.lspLogBackBtn) {
    elements.lspLogBackBtn.addEventListener('click', () => handleBack());
  }

  if (elements.lspLogRefreshBtn) {
    elements.lspLogRefreshBtn.addEventListener('click', () => {
      void viewLspLogs();
    });
  }

  if (elements.lspLogZoomOut) {
    elements.lspLogZoomOut.addEventListener('click', () => adjustLspLogZoom(-0.1));
  }

  if (elements.lspLogZoomIn) {
    elements.lspLogZoomIn.addEventListener('click', () => adjustLspLogZoom(0.1));
  }

  syncLspLogZoom();

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
  if ('scrollRestoration' in history) {
    history.scrollRestoration = 'manual';
  }

  window.addEventListener('popstate', async (event) => {
    // 检查是否是我们管理的状态
    if (event.state && event.state.view) {
      saveCurrentScrollPosition();
      isNavigatingBack = true;
      const targetView = event.state.view;
      
      // 根据历史记录中的状态切换视图（不pushState）
      switch (targetView) {
        case 'search':
          switchToSearch();
          restoreViewScrollPosition(targetView);
          break;
        case 'settings':
          switchToSettings();
          restoreViewScrollPosition(targetView);
          break;
        case 'songs':
          switchToSongs();
          restoreViewScrollPosition(targetView);
          await loadSavedLyrics();
          restoreViewScrollPosition(targetView);
          break;
        case 'lspLogs':
          switchToLspLogs();
          restoreViewScrollPosition(targetView);
          await viewLspLogs();
          restoreViewScrollPosition(targetView);
          break;
        case 'results':
          switchToResults();
          restoreViewScrollPosition(targetView);
          break;
        case 'lyrics':
          switchToLyrics({ resetScroll: false });
          restoreViewScrollPosition(targetView);
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

function canGestureBack() {
  return currentView !== 'search';
}

function initBackGesture() {
  const edgeWidth = 28;
  const triggerDistance = 74;
  const maxVerticalDrift = 64;
  let gesture = null;

  window.addEventListener('touchstart', (event) => {
    if (!canGestureBack() || event.touches.length !== 1) {
      gesture = null;
      return;
    }

    const touch = event.touches[0];
    if (touch.clientX > edgeWidth) {
      gesture = null;
      return;
    }

    gesture = {
      startX: touch.clientX,
      startY: touch.clientY,
      active: true,
      triggered: false,
    };
  }, { passive: true });

  window.addEventListener('touchmove', (event) => {
    if (!gesture?.active || event.touches.length !== 1) {
      return;
    }

    const touch = event.touches[0];
    const dx = touch.clientX - gesture.startX;
    const dy = touch.clientY - gesture.startY;

    if (Math.abs(dy) > maxVerticalDrift && Math.abs(dy) > dx) {
      gesture.active = false;
      return;
    }

    if (!gesture.triggered && dx >= triggerDistance && Math.abs(dy) <= maxVerticalDrift) {
      gesture.triggered = true;
      gesture.active = false;
      handleBack();
    }
  }, { passive: true });

  window.addEventListener('touchend', () => {
    gesture = null;
  }, { passive: true });

  window.addEventListener('touchcancel', () => {
    gesture = null;
  }, { passive: true });
}


function handleSongsDockAutoHide() {
  if (currentView !== 'songs' || !elements.bottomMenu || elements.bottomMenu.classList.contains('hidden')) {
    setBottomMenuAutoHidden(false);
    return;
  }

  const scrollY = currentPageScrollY();
  const delta = scrollY - lastSongsScrollY;
  lastSongsScrollY = scrollY;

  if (scrollY <= 24) {
    setBottomMenuAutoHidden(false);
    return;
  }

  if (delta > 8) {
    setBottomMenuAutoHidden(true);
  } else if (delta < -8) {
    setBottomMenuAutoHidden(false);
  }
}

function initSongsDockAutoHide() {
  lastSongsScrollY = currentPageScrollY();
  window.addEventListener('scroll', handleSongsDockAutoHide, { passive: true });
  window.addEventListener('resize', handleSongsDockAutoHide, { passive: true });
}

function initBottomMenu() {
  $$('[data-app-tab]').forEach((button) => {
    button.addEventListener('click', () => {
      if (button.dataset.appTab === 'settings') {
        if (currentView !== 'settings') showSettings();
        return;
      }

      if (button.dataset.appTab === 'songs') {
        if (currentView !== 'songs') showSongs();
        return;
      }

      if (currentView !== 'search') {
        showSearch();
      }
    });
  });

  setBottomMenuVisible(
    ['search', 'songs', 'settings'].includes(currentView),
    ['search', 'songs', 'settings'].includes(currentView) ? currentView : 'search'
  );
}

// ==================== Utils ====================

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// ==================== Salt Player Launch Flow ====================

async function checkSaltLaunchRequest() {
  if (!isTauriEnv) return;

  try {
    const request = await invoke('take_salt_launch_request');
    if (!request || !request.title) return;

    pendingSaltRequest = request;
    elements.searchTitle.value = request.title || '';
    elements.searchArtist.value = request.artist || '';
    switchToSearch();
    void appendAppLspLog('salt', `launch request received title="${request.title}" artist="${request.artist || ''}"`);
    showError(`Salt Player から「${request.title}」を受け取りました。検索して候補を選ぶと、確認後にこの曲へ Ruby 表示を適用します。`);
  } catch (err) {
    console.warn('Salt launch request check failed:', err);
  }
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
  initSongsControls();

  // 底部菜单
  initBottomMenu();
  initSongsDockAutoHide();

  // 搜索历史
  renderSearchHistory();

  // 结果页无限滚动
  initInfiniteScroll();
  
  // Android返回按钮支持
  initBackButton();
  initBackGesture();
  
  // 应用保存的暗色模式
  const settings = loadSettings();
  if (settings.darkMode === 'on') {
    document.body.classList.add('dark-mode');
  }
  
  // 同步按钮状态
  updateButtonStates();

  void setBackendLspLogging(settings.lspLogEnabled === true);
  void tauriReady.then(checkSaltLaunchRequest);
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') {
      void checkSaltLaunchRequest();
    }
  });
  
  // 浏览器模式提示
  if (!isTauriEnv) {
    console.log('🦊 UtaBuild Browser Mode - 使用Mock数据调试UI');
  }
  console.log('UtaBuild initialized');
}

// 启动
document.addEventListener('DOMContentLoaded', init);
