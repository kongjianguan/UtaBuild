import { loadSettings } from './settings.js';
// ==================== Element Accessors ====================
const _elCache = new Map();
export function el(id) {
    if (!_elCache.has(id)) {
        const node = document.querySelector(`#${id}`);
        if (!node)
            throw new Error(`Missing element: #${id}`);
        _elCache.set(id, node);
    }
    return _elCache.get(id);
}
export function $$(sel) {
    return document.querySelectorAll(sel);
}
// ==================== UI Helpers ====================
export function show(el) {
    if (el)
        el.classList.remove('hidden');
}
export function hide(el) {
    if (el)
        el.classList.add('hidden');
}
export function currentPageScrollY() {
    return window.scrollY || document.documentElement.scrollTop || document.body.scrollTop || 0;
}
function _scrollPageTo(y) {
    const top = Math.max(0, Math.round(y || 0));
    document.documentElement.scrollTop = top;
    document.body.scrollTop = top;
    window.scrollTo({ top, left: 0, behavior: 'auto' });
}
function _repeatScrollTo(y) {
    _scrollPageTo(y);
    requestAnimationFrame(() => _scrollPageTo(y));
    setTimeout(() => _scrollPageTo(y), 80);
}
export function showLoading() {
    show(el('loading'));
}
export function hideLoading() {
    hide(el('loading'));
}
export function showError(msg) {
    el('error-message').textContent = msg;
    show(el('error-toast'));
    setTimeout(() => hide(el('error-toast')), 5000);
}
export function setBottomMenuAutoHidden(isHidden) {
    el('bottom-menu').classList.toggle('is-auto-hidden', Boolean(isHidden));
}
// ==================== View Router ====================
const FIRST_LEVEL = new Set(['search', 'songs', 'settings']);
const VIEW_ORDER = {
    search: 0,
    songs: 1,
    settings: 2,
    lspSettings: 3,
    lspLogs: 3,
    results: 3,
    lyrics: 3,
};
const SONGS_SCROLLBAR_CLASS = 'songs-first-level-scrollbar-disabled';
const _viewScroll = new Map();
function _saveScroll() {
    _viewScroll.set(router._current, currentPageScrollY());
}
function _restoreScroll(view) {
    _repeatScrollTo(_viewScroll.get(view) || 0);
}
function _setFirstLevelScrollbar(view) {
    const isSongs = view === 'songs';
    document.documentElement.classList.toggle(SONGS_SCROLLBAR_CLASS, isSongs);
    document.body.classList.toggle(SONGS_SCROLLBAR_CLASS, isSongs);
}
function _syncBottomMenu(activeTab) {
    const menu = el('bottom-menu');
    menu.dataset.activeTab = activeTab;
    $$('[data-app-tab]').forEach((button) => {
        const btn = button;
        const isActive = btn.dataset.appTab === activeTab;
        btn.classList.toggle('active', isActive);
        btn.setAttribute('aria-selected', String(isActive));
        if (isActive) {
            btn.setAttribute('aria-current', 'page');
        }
        else {
            btn.removeAttribute('aria-current');
        }
    });
}
function _setBottomMenu(visible, activeTab) {
    const menu = el('bottom-menu');
    menu.classList.toggle('hidden', !visible);
    menu.setAttribute('aria-hidden', String(!visible));
    document.body.classList.toggle('has-bottom-menu', visible);
    if (visible)
        _syncBottomMenu(activeTab || 'search');
}
function _toggleViewElements(view) {
    const ids = [
        'search-header', 'songs-view', 'settings-view',
        'lsp-settings-view', 'lsp-log-view', 'result-list', 'lyrics-view',
    ];
    const viewIdMap = {
        search: 'search-header',
        songs: 'songs-view',
        settings: 'settings-view',
        lspSettings: 'lsp-settings-view',
        lspLogs: 'lsp-log-view',
        results: 'result-list',
        lyrics: 'lyrics-view',
    };
    for (const id of ids) {
        if (id === viewIdMap[view]) {
            el(id).classList.remove('hidden');
        }
        else {
            el(id).classList.add('hidden');
        }
    }
}
function _animateEntry(view, direction) {
    if (!FIRST_LEVEL.has(view))
        return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches)
        return;
    const viewElMap = {
        search: 'search-header',
        songs: 'songs-view',
        settings: 'settings-view',
    };
    const targetId = viewElMap[view];
    if (!targetId)
        return;
    for (const id of Object.values(viewElMap)) {
        el(id).classList.remove('first-level-slide-from-left', 'first-level-slide-from-right');
    }
    const target = el(targetId);
    const className = direction === 'back'
        ? 'first-level-slide-from-left'
        : 'first-level-slide-from-right';
    target.classList.add(className);
    setTimeout(() => target.classList.remove(className), 420);
}
export class Router {
    _current = 'search';
    _navigatingBack = false;
    get current() {
        return this._current;
    }
    navigate(view, opts) {
        if (view === this._current)
            return;
        _saveScroll();
        const prev = this._current;
        this._current = view;
        _toggleViewElements(view);
        _setFirstLevelScrollbar(view);
        _setBottomMenu(FIRST_LEVEL.has(view), view);
        if (opts?.resetScroll) {
            _repeatScrollTo(0);
        }
        if (opts?.animate) {
            const dir = (VIEW_ORDER[view] > VIEW_ORDER[prev]) ? 'forward' : 'back';
            _animateEntry(view, dir);
        }
        if (!this._navigatingBack) {
            history.pushState({ view }, '', '');
        }
        this._navigatingBack = false;
        if (view === 'lspSettings') {
            syncLspLogVisibility();
        }
    }
    back() {
        this._navigatingBack = true;
        window.history.back();
    }
    handlePopstate(event) {
        const state = event.state;
        if (state?.view) {
            this._navigatingBack = true;
            _saveScroll();
            this._current = state.view;
            _toggleViewElements(state.view);
            _setFirstLevelScrollbar(state.view);
            _setBottomMenu(FIRST_LEVEL.has(state.view), state.view);
            _restoreScroll(state.view);
        }
    }
}
export const router = new Router();
// ==================== Button States ====================
export function updateButtonStates() {
    const settings = loadSettings();
    const fontSize = settings.fontSize || 'medium';
    $$('[data-size]').forEach((button) => {
        const btn = button;
        const isActive = btn.dataset.size === fontSize;
        btn.classList.toggle('active', isActive);
        btn.setAttribute('aria-pressed', String(isActive));
    });
    const sizeBtn = document.querySelector(`[data-size="${fontSize}"]`);
    if (sizeBtn)
        sizeBtn.classList.add('active');
    const darkMode = settings.darkMode || 'off';
    $$('[data-dark]').forEach((button) => {
        const btn = button;
        const isActive = btn.dataset.dark === darkMode;
        btn.classList.toggle('active', isActive);
        btn.setAttribute('aria-pressed', String(isActive));
    });
    const darkBtn = document.querySelector(`[data-dark="${darkMode}"]`);
    if (darkBtn)
        darkBtn.classList.add('active');
}
// ==================== LSP Log Visibility (DOM-side) ====================
export function syncLspLogVisibility() {
    const settingLspLog = el('setting-lsp-log');
    if (!settingLspLog)
        return;
    const lspLogPanel = el('lsp-log-panel');
    if (!lspLogPanel)
        return;
    const enabled = loadSettings().lspLogEnabled === true;
    settingLspLog.checked = enabled;
    lspLogPanel.classList.toggle('hidden', !enabled);
    if (!enabled) {
        const lspLogContent = el('lsp-log-content');
        lspLogContent.textContent = '';
        if (router._current === 'lspLogs') {
            router.navigate('settings', { resetScroll: true });
        }
    }
    if (!enabled && router._current === 'lspSettings') {
        lspLogPanel.classList.add('hidden');
    }
}
//# sourceMappingURL=dom.js.map