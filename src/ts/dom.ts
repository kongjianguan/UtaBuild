import type { ViewType } from './types.js';
import { loadSettings } from './settings.js';
// Cross-module imports for show* functions (circular but safe with ESM - used only in function bodies)
import { loadSavedLyrics } from './songs.js';
import { viewLspLogs } from './lsp.js';

// ==================== Element Accessors ====================

const _elCache = new Map<string, Element>();

export function el<T extends Element>(id: string): T {
  if (!_elCache.has(id)) {
    const node = document.querySelector(`#${id}`);
    if (!node) throw new Error(`Missing element: #${id}`);
    _elCache.set(id, node);
  }
  return _elCache.get(id) as T;
}

export function $$(sel: string): NodeListOf<Element> {
  return document.querySelectorAll(sel);
}

// ==================== State ====================

export const SONGS_FIRST_LEVEL_SCROLLBAR_DISABLED_CLASS =
  'songs-first-level-scrollbar-disabled';

export let currentView: ViewType = 'search';
export let isBottomMenuAutoHidden = false;
export const viewScrollPositions = new Map<ViewType, number>();

export function setCurrentView(view: ViewType): void {
  currentView = view;
  const isSongsFirstLevel = view === 'songs';
  document.documentElement.classList.toggle(
    SONGS_FIRST_LEVEL_SCROLLBAR_DISABLED_CLASS,
    isSongsFirstLevel,
  );
  document.body.classList.toggle(SONGS_FIRST_LEVEL_SCROLLBAR_DISABLED_CLASS, isSongsFirstLevel);
}

// ==================== UI Helpers ====================

export function show(el: Element | null): void {
  if (el) el.classList.remove('hidden');
}

export function hide(el: Element | null): void {
  if (el) el.classList.add('hidden');
}

export function currentPageScrollY(): number {
  return (
    window.scrollY || document.documentElement.scrollTop || document.body.scrollTop || 0
  );
}

export function scrollPageTo(y: number): void {
  const top = Math.max(0, Math.round(Number(y) || 0));
  document.documentElement.scrollTop = top;
  document.body.scrollTop = top;
  window.scrollTo({ top, left: 0, behavior: 'auto' });
}

export function scrollPageToTop(): void {
  scrollPageTo(0);
}

function repeatScrollTo(y: number): void {
  scrollPageTo(y);
  requestAnimationFrame(() => scrollPageTo(y));
  setTimeout(() => scrollPageTo(y), 80);
}

export function resetViewportToTop(): void {
  repeatScrollTo(0);
}

export function saveCurrentScrollPosition(): void {
  if (!currentView) return;
  viewScrollPositions.set(currentView, currentPageScrollY());
}

export function restoreViewScrollPosition(view: ViewType): void {
  repeatScrollTo(viewScrollPositions.get(view) || 0);
}

export function showLoading(): void {
  const el = getElements();
  show(el.loading);
}

export function hideLoading(): void {
  const el = getElements();
  hide(el.loading);
}

export function showError(msg: string): void {
  const el = getElements();
  el.errorMessage.textContent = msg;
  show(el.errorToast);
  setTimeout(() => hide(el.errorToast), 5000);
}

// ==================== Bottom Menu ====================

export function syncBottomMenu(activeTab: string): void {
  const el = getElements();
  if (!el.bottomMenu) return;

  el.bottomMenu.dataset.activeTab = activeTab;
  $$('[data-app-tab]').forEach((button) => {
    const btn = button as HTMLElement;
    const isActive = btn.dataset.appTab === activeTab;
    btn.classList.toggle('active', isActive);
    btn.setAttribute('aria-selected', String(isActive));
    if (isActive) {
      btn.setAttribute('aria-current', 'page');
    } else {
      btn.removeAttribute('aria-current');
    }
  });
}

export function setBottomMenuAutoHidden(isHidden: boolean): void {
  const el = getElements();
  if (!el.bottomMenu) return;

  isBottomMenuAutoHidden = Boolean(isHidden);
  el.bottomMenu.classList.toggle('is-auto-hidden', isBottomMenuAutoHidden);
}

export function setBottomMenuVisible(isVisible: boolean, activeTab?: string): void {
  const el = getElements();
  if (!el.bottomMenu) return;

  el.bottomMenu.classList.toggle('hidden', !isVisible);
  el.bottomMenu.setAttribute('aria-hidden', String(!isVisible));
  document.body.classList.toggle('has-bottom-menu', isVisible);

  if (!isVisible) {
    setBottomMenuAutoHidden(false);
    return;
  }

  syncBottomMenu(activeTab || 'search');
  setBottomMenuAutoHidden(false);
}

// ==================== View Helpers ====================

const FIRST_LEVEL_INDEX: Record<string, number> = {
  search: 0,
  songs: 1,
  settings: 2,
  lspSettings: 3,
  lspLogs: 3,
  results: 3,
  lyrics: 3,
};

export function firstLevelIndex(view: ViewType): number {
  return FIRST_LEVEL_INDEX[view] ?? 0;
}

export function firstLevelElement(view: ViewType): HTMLElement | null {
  const el = getElements();
  if (view === 'settings') return el.settingsView;
  if (view === 'songs') return el.songsView;
  return el.searchHeader;
}

let firstLevelAnimationTimer: ReturnType<typeof setTimeout> | null = null;

export function animateFirstLevelEntry(view: ViewType, direction: string): void {
  const target = firstLevelElement(view);
  if (!target || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

  if (firstLevelAnimationTimer) clearTimeout(firstLevelAnimationTimer);
  const el = getElements();
  [el.searchHeader, el.songsView, el.settingsView].forEach((element) => {
    element?.classList.remove('first-level-slide-from-left', 'first-level-slide-from-right');
  });

  const className =
    direction === 'back' ? 'first-level-slide-from-left' : 'first-level-slide-from-right';
  target.classList.add(className);

  firstLevelAnimationTimer = setTimeout(() => {
    target.classList.remove(className);
  }, 420);
}

// ==================== View Switching (internal, no pushState) ====================

export function switchToSearch(options?: { animate?: boolean; direction?: string }): void {
  const el = getElements();
  show(el.searchHeader);
  hide(el.songsView);
  hide(el.settingsView);
  hide(el.lspSettingsView);
  hide(el.lspLogView);
  hide(el.resultList);
  hide(el.lyricsView);
  setCurrentView('search');
  setBottomMenuVisible(true, 'search');
  if (options?.animate) animateFirstLevelEntry('search', options.direction || 'back');
}

export function switchToSettings(options?: { animate?: boolean; direction?: string }): void {
  const el = getElements();
  hide(el.searchHeader);
  hide(el.songsView);
  show(el.settingsView);
  hide(el.lspSettingsView);
  hide(el.lspLogView);
  hide(el.resultList);
  hide(el.lyricsView);
  setCurrentView('settings');
  setBottomMenuVisible(true, 'settings');
  if (options?.animate) animateFirstLevelEntry('settings', options.direction || 'forward');
}

export function switchToSongs(options?: { animate?: boolean; direction?: string }): void {
  const el = getElements();
  hide(el.searchHeader);
  show(el.songsView);
  hide(el.settingsView);
  hide(el.lspSettingsView);
  hide(el.lspLogView);
  hide(el.resultList);
  hide(el.lyricsView);
  setCurrentView('songs');
  setBottomMenuVisible(true, 'songs');
  setBottomMenuAutoHidden(false);
  if (options?.animate) animateFirstLevelEntry('songs', options.direction || 'forward');
}

export function switchToResults(): void {
  const el = getElements();
  hide(el.searchHeader);
  hide(el.songsView);
  hide(el.settingsView);
  hide(el.lspSettingsView);
  hide(el.lspLogView);
  show(el.resultList);
  hide(el.lyricsView);
  setCurrentView('results');
  setBottomMenuVisible(false);
}

export function switchToLyrics(options?: { resetScroll?: boolean }): void {
  const el = getElements();
  hide(el.searchHeader);
  hide(el.songsView);
  hide(el.settingsView);
  hide(el.lspSettingsView);
  hide(el.lspLogView);
  hide(el.resultList);
  show(el.lyricsView);
  setCurrentView('lyrics');
  setBottomMenuVisible(false);
  if (options?.resetScroll !== false) {
    resetViewportToTop();
  }
}

export function switchToLspLogs(): void {
  const el = getElements();
  hide(el.searchHeader);
  hide(el.songsView);
  hide(el.settingsView);
  hide(el.lspSettingsView);
  show(el.lspLogView);
  hide(el.resultList);
  hide(el.lyricsView);
  setCurrentView('lspLogs');
  setBottomMenuVisible(false);
}

export function switchToLspSettings(options?: { resetScroll?: boolean }): void {
  const el = getElements();
  hide(el.searchHeader);
  hide(el.songsView);
  hide(el.settingsView);
  show(el.lspSettingsView);
  hide(el.lspLogView);
  hide(el.resultList);
  hide(el.lyricsView);
  setCurrentView('lspSettings');
  setBottomMenuVisible(false);
  syncLspLogVisibility();
  if (options?.resetScroll !== false) {
    resetViewportToTop();
  }
}

// ==================== View Switching (user action, with pushState) ====================

let isNavigatingBack = false;

export function setIsNavigatingBack(v: boolean): void {
  isNavigatingBack = v;
}

export function isNavigatingBackFlag(): boolean {
  return isNavigatingBack;
}

export function showSearch(): void {
  saveCurrentScrollPosition();
  const previousView = currentView;
  const shouldAnimate =
    ['songs', 'settings'].includes(previousView) && !isNavigatingBack;
  switchToSearch({
    animate: shouldAnimate,
    direction:
      firstLevelIndex(previousView) > firstLevelIndex('search') ? 'back' : 'forward',
  });
  if (!isNavigatingBack) {
    history.pushState({ view: 'search' }, '', '');
  }
}

export function showSettings(): void {
  saveCurrentScrollPosition();
  const previousView = currentView;
  const shouldAnimate =
    ['search', 'songs'].includes(previousView) && !isNavigatingBack;
  switchToSettings({
    animate: shouldAnimate,
    direction:
      firstLevelIndex(previousView) < firstLevelIndex('settings') ? 'forward' : 'back',
  });
  if (!isNavigatingBack) {
    history.pushState({ view: 'settings' }, '', '');
  }
}

export function showSongs(): void {
  saveCurrentScrollPosition();
  const previousView = currentView;
  const shouldAnimate =
    ['search', 'settings'].includes(previousView) && !isNavigatingBack;
  switchToSongs({
    animate: shouldAnimate,
    direction:
      firstLevelIndex(previousView) < firstLevelIndex('songs') ? 'forward' : 'back',
  });
  if (!isNavigatingBack) {
    history.pushState({ view: 'songs' }, '', '');
  }
  void loadSavedLyrics();
}

export function showResults(): void {
  saveCurrentScrollPosition();
  switchToResults();
  if (!isNavigatingBack) {
    history.pushState({ view: 'results' }, '', '');
  }
}

export function showLyricsView(): void {
  saveCurrentScrollPosition();
  switchToLyrics({ resetScroll: true });
  if (!isNavigatingBack) {
    history.pushState({ view: 'lyrics' }, '', '');
  }
}

export function showLspLogsView(): void {
  saveCurrentScrollPosition();
  switchToLspLogs();
  if (!isNavigatingBack) {
    history.pushState({ view: 'lspLogs' }, '', '');
  }
  void viewLspLogs();
}

export function showLspSettingsView(): void {
  saveCurrentScrollPosition();
  switchToLspSettings({ resetScroll: true });
  if (!isNavigatingBack) {
    history.pushState({ view: 'lspSettings' }, '', '');
  }
}

// ==================== Button States ====================

export function updateButtonStates(): void {
  const settings = loadSettings();

  const fontSize = settings.fontSize || 'medium';
  $$('[data-size]').forEach((button) => {
    const btn = button as HTMLElement;
    const isActive = btn.dataset.size === fontSize;
    btn.classList.toggle('active', isActive);
    btn.setAttribute('aria-pressed', String(isActive));
  });
  const sizeBtn = document.querySelector(`[data-size="${fontSize}"]`);
  if (sizeBtn) sizeBtn.classList.add('active');

  const darkMode = settings.darkMode || 'off';
  $$('[data-dark]').forEach((button) => {
    const btn = button as HTMLElement;
    const isActive = btn.dataset.dark === darkMode;
    btn.classList.toggle('active', isActive);
    btn.setAttribute('aria-pressed', String(isActive));
  });
  const darkBtn = document.querySelector(`[data-dark="${darkMode}"]`);
  if (darkBtn) darkBtn.classList.add('active');
}

// ==================== LSP Log Visibility (DOM-side) ====================

export function syncLspLogVisibility(): void {
  const el = getElements();
  if (!el.settingLspLog || !el.lspLogPanel) return;

  const enabled = loadSettings().lspLogEnabled === true;
  el.settingLspLog.checked = enabled;
  el.lspLogPanel.classList.toggle('hidden', !enabled);
  if (!enabled && el.lspLogContent) {
    el.lspLogContent.textContent = '';
    if (currentView === 'lspLogs') {
      switchToSettings();
    }
  }
  if (!enabled && currentView === 'lspSettings') {
    el.lspLogPanel.classList.add('hidden');
  }
}
