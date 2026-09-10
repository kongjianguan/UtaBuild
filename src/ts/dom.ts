import type { ViewType } from './types.js';
import {
  clampFontSize,
  defaultFontSizeForViewport,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  loadSettings,
} from './settings.js';

// ==================== Element Accessors ====================

const _elCache = new Map<string, Element>();
let errorToastTimer: ReturnType<typeof setTimeout> | null = null;
let toastActionHandler: (() => void) | null = null;
let toastActionBound = false;

type ToastTone = 'error' | 'success' | 'info';

export interface ToastAction {
  label: string;
  onClick: () => void;
}

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

// ==================== UI Helpers ====================

export function show(el: Element | null): void {
  if (el) el.classList.remove('hidden');
}

export function hide(el: Element | null): void {
  if (el) el.classList.add('hidden');
}

export function currentPageScrollY(): number {
  const scrollContainer = document.querySelector<HTMLElement>(
    `[data-view-scroll="${router._current}"]`,
  );
  return scrollContainer?.scrollTop || 0;
}

function _scrollViewTo(view: ViewType, y: number): void {
  const top = Math.max(0, Math.round(y || 0));
  const scrollContainer = document.querySelector<HTMLElement>(
    `[data-view-scroll="${view}"]`,
  );
  if (scrollContainer) {
    scrollContainer.scrollTop = top;
    return;
  }
  document.documentElement.scrollTop = 0;
  document.body.scrollTop = 0;
}

function _repeatScrollTo(view: ViewType, y: number): void {
  _scrollViewTo(view, y);
  requestAnimationFrame(() => _scrollViewTo(view, y));
  setTimeout(() => _scrollViewTo(view, y), 80);
}

export function showLoading(): void {
  show(el<HTMLElement>('loading'));
  el<HTMLElement>('app').setAttribute('aria-busy', 'true');
  document.body.classList.add('is-loading');
}

export function hideLoading(): void {
  hide(el<HTMLElement>('loading'));
  el<HTMLElement>('app').removeAttribute('aria-busy');
  document.body.classList.remove('is-loading');
}

export function showToast(msg: string, tone: ToastTone = 'error', action?: ToastAction): void {
  const toast = el<HTMLElement>('error-toast');
  const actionBtn = el<HTMLButtonElement>('error-action');

  if (!toastActionBound) {
    toastActionBound = true;
    actionBtn.addEventListener('click', () => {
      const handler = toastActionHandler;
      hide(toast);
      toastActionHandler = null;
      handler?.();
    });
  }

  toast.classList.remove('is-success', 'is-info');
  if (tone !== 'error') toast.classList.add(`is-${tone}`);
  toast.setAttribute('role', tone === 'error' ? 'alert' : 'status');
  toast.setAttribute('aria-live', tone === 'error' ? 'assertive' : 'polite');
  el<HTMLElement>('error-message').textContent = msg;

  if (action) {
    actionBtn.textContent = action.label;
    toastActionHandler = action.onClick;
    show(actionBtn);
  } else {
    toastActionHandler = null;
    hide(actionBtn);
  }

  show(toast);
  if (errorToastTimer) {
    clearTimeout(errorToastTimer);
    errorToastTimer = null;
  }
  // Errors stay until dismissed so the message and its recovery action
  // cannot be missed; success/info toasts still auto-dismiss.
  if (tone !== 'error') {
    errorToastTimer = setTimeout(() => {
      hide(toast);
      errorToastTimer = null;
    }, 5000);
  }
}

export function showError(msg: string, action?: ToastAction): void {
  showToast(msg, 'error', action);
}

export function showSuccess(msg: string): void {
  showToast(msg, 'success');
}

export function showInfo(msg: string): void {
  showToast(msg, 'info');
}

export function setBottomMenuAutoHidden(isHidden: boolean): void {
  el<HTMLElement>('bottom-menu').classList.toggle('is-auto-hidden', Boolean(isHidden));
}

// ==================== View Router ====================

const FIRST_LEVEL = new Set<ViewType>(['search', 'songs', 'settings']);

const VIEW_ORDER: Record<ViewType, number> = {
  search: 0,
  songs: 1,
  settings: 2,
  settingsAbout: 3,
  lspSettings: 4,
  lspLogs: 4,
  results: 4,
  lyrics: 4,
};

const _viewScroll = new Map<ViewType, number>();

function _saveScroll(): void {
  _viewScroll.set(router._current, currentPageScrollY());
}

function _restoreScroll(view: ViewType): void {
  _repeatScrollTo(view, _viewScroll.get(view) || 0);
}

function _syncBottomMenu(activeTab: string): void {
  const menu = el<HTMLElement>('bottom-menu');
  menu.dataset.activeTab = activeTab;
  let activeBtn: HTMLElement | null = null;
  $$('[data-app-tab]').forEach((button) => {
    const btn = button as HTMLElement;
    const isActive = btn.dataset.appTab === activeTab;
    btn.classList.toggle('active', isActive);
    btn.setAttribute('aria-selected', String(isActive));
    // Roving tabindex: only the active tab stays in the keyboard tab order.
    btn.tabIndex = isActive ? 0 : -1;
    if (isActive) {
      btn.setAttribute('aria-current', 'page');
      activeBtn = btn;
    } else {
      btn.removeAttribute('aria-current');
    }
  });

  const slider = menu.querySelector<HTMLElement>('.bottom-menu__slider');
  if (slider && activeBtn) {
    const btn = activeBtn as HTMLElement;
    const track = slider.parentElement;
    if (track) {
      const btnLeft = btn.offsetLeft;
      const btnTop = btn.offsetTop;
      const btnWidth = btn.offsetWidth;
      const btnHeight = btn.offsetHeight;
      slider.style.width = `${btnWidth}px`;
      slider.style.height = `${btnHeight}px`;
      slider.style.transform = `translate3d(${btnLeft - 6}px, ${btnTop - 6}px, 0)`;
      slider.classList.add('is-ready');
    }
  }
}

export function updateBottomMenuSlider(): void {
  const menu = document.getElementById('bottom-menu');
  if (!menu || menu.classList.contains('hidden')) return;

  const activeBtn = menu.querySelector<HTMLElement>('.bottom-menu__item.active');
  const slider = menu.querySelector<HTMLElement>('.bottom-menu__slider');
  if (!slider || !activeBtn) return;

  const btnLeft = activeBtn.offsetLeft;
  const btnTop = activeBtn.offsetTop;
  const btnWidth = activeBtn.offsetWidth;
  const btnHeight = activeBtn.offsetHeight;
  slider.style.width = `${btnWidth}px`;
  slider.style.height = `${btnHeight}px`;
  slider.style.transform = `translate3d(${btnLeft - 6}px, ${btnTop - 6}px, 0)`;
  slider.classList.add('is-ready');
}

function _setBottomMenu(visible: boolean, activeTab?: string): void {
  const menu = el<HTMLElement>('bottom-menu');
  menu.classList.toggle('hidden', !visible);
  menu.setAttribute('aria-hidden', String(!visible));
  document.body.classList.toggle('has-bottom-menu', visible);
  if (visible) _syncBottomMenu(activeTab || 'search');
}

function _toggleViewElements(view: ViewType): void {
  const ids = [
    'search-header', 'songs-view', 'settings-view', 'settings-about-view',
    'lsp-settings-view', 'lsp-log-view', 'result-list', 'lyrics-view',
  ];
  const viewIdMap: Record<ViewType, string> = {
    search: 'search-header',
    songs: 'songs-view',
    settings: 'settings-view',
    settingsAbout: 'settings-about-view',
    lspSettings: 'lsp-settings-view',
    lspLogs: 'lsp-log-view',
    results: 'result-list',
    lyrics: 'lyrics-view',
  };
  for (const id of ids) {
    const viewEl = el<HTMLElement>(id);
    const isActive = id === viewIdMap[view];
    viewEl.classList.toggle('hidden', !isActive);
    viewEl.setAttribute('aria-hidden', String(!isActive));
  }
}

function _animateEntry(view: ViewType, direction: 'forward' | 'back'): void {
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

  const viewElMap: Record<string, string> = {
    search: 'search-header',
    songs: 'songs-view',
    settings: 'settings-view',
    settingsAbout: 'settings-about-view',
    lspSettings: 'lsp-settings-view',
    lspLogs: 'lsp-log-view',
    results: 'result-list',
    lyrics: 'lyrics-view',
  };
  const targetId = viewElMap[view];
  if (!targetId) return;

  for (const id of Object.values(viewElMap)) {
    el<HTMLElement>(id).classList.remove('view-enter-from-left', 'view-enter-from-right');
  }

  const target = el<HTMLElement>(targetId);
  const className = direction === 'back' ? 'view-enter-from-left' : 'view-enter-from-right';
  target.classList.add(className);

  setTimeout(() => target.classList.remove(className), 260);
}

function _focusView(view: ViewType): void {
  const viewIdMap: Record<string, string> = {
    search: 'search-header',
    songs: 'songs-view',
    settings: 'settings-view',
    settingsAbout: 'settings-about-view',
    lspSettings: 'lsp-settings-view',
    lspLogs: 'lsp-log-view',
    results: 'result-list',
    lyrics: 'lyrics-view',
  };
  const viewEl = el<HTMLElement>(viewIdMap[view]);
  $$('h1, h2').forEach((heading) => heading.classList.remove('router-focus'));
  const heading = viewEl.querySelector('h1, h2') as HTMLElement | null;
  if (!heading) return;
  if (!heading.hasAttribute('tabindex')) heading.tabIndex = -1;
  heading.classList.add('router-focus');
  heading.focus({ preventScroll: true });
}

export class Router {
  _current: ViewType = 'search';
  private _navigatingBack = false;

  get current(): ViewType {
    return this._current;
  }

  navigate(view: ViewType, opts?: { animate?: boolean; resetScroll?: boolean; replace?: boolean }): void {
    if (view === this._current) return;

    _saveScroll();
    const prev = this._current;
    this._current = view;

    _toggleViewElements(view);
    _setBottomMenu(FIRST_LEVEL.has(view), view);

    if (opts?.resetScroll) {
      _repeatScrollTo(view, 0);
    }

    if (opts?.animate) {
      const dir = (VIEW_ORDER[view] > VIEW_ORDER[prev]) ? 'forward' : 'back';
      _animateEntry(view, dir);
    }

    _focusView(view);

    if (!this._navigatingBack) {
      // Switching between first-level tabs replaces the current entry so the
      // Android back button leaves the app instead of replaying tab history.
      if (opts?.replace) {
        history.replaceState({ view }, '', '');
      } else {
        history.pushState({ view }, '', '');
      }
    }
    this._navigatingBack = false;

    if (view === 'lspSettings') {
      syncLspLogVisibility();
    }
  }

  back(): void {
    this._navigatingBack = true;
    window.history.back();
    // If no popstate fires (e.g. already at the first history entry),
    // don't leave the flag stuck, which would break the next navigate().
    setTimeout(() => {
      this._navigatingBack = false;
    }, 0);
  }

  handlePopstate(event: PopStateEvent): void {
    this._navigatingBack = false;
    const state = event.state as { view?: ViewType } | null;
    if (state?.view) {
      _saveScroll();
      this._current = state.view;
      _toggleViewElements(state.view);
      _setBottomMenu(FIRST_LEVEL.has(state.view), state.view);
      _restoreScroll(state.view);
      _focusView(state.view);
    }
  }
}

export const router = new Router();

// ==================== Button States ====================

export function syncFontSizeControls(fontSize?: number): void {
  const range = document.getElementById('font-size-range') as HTMLInputElement | null;
  const valueInput = document.getElementById('font-size-value') as HTMLInputElement | null;
  if (!range || !valueInput) return;

  const currentFontSize = fontSize ?? loadSettings().fontSize ?? defaultFontSizeForViewport();
  const normalized = clampFontSize(currentFontSize);
  range.min = String(FONT_SIZE_MIN);
  range.max = String(FONT_SIZE_MAX);
  range.step = '1';
  range.value = String(normalized);
  range.style.setProperty(
    '--range-progress',
    `${((normalized - FONT_SIZE_MIN) / (FONT_SIZE_MAX - FONT_SIZE_MIN)) * 100}%`,
  );
  range.setAttribute('aria-valuetext', `${normalized}px`);

  valueInput.min = String(FONT_SIZE_MIN);
  valueInput.max = String(FONT_SIZE_MAX);
  valueInput.step = '1';
  valueInput.value = String(normalized);
}

export function updateButtonStates(): void {
  const settings = loadSettings();

  syncFontSizeControls(settings.fontSize);

  const theme = settings.theme || 'dark';
  $$('.lyrics-controls [data-theme]').forEach((button) => {
    const btn = button as HTMLElement;
    const isActive = btn.dataset.theme === theme;
    btn.classList.toggle('active', isActive);
    btn.setAttribute('aria-pressed', String(isActive));
  });
}

// ==================== Confirmation Dialog ====================

export interface ConfirmDialogOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
}

let confirmDialogActive = false;

/**
 * Modal confirmation dialog with focus trap, ESC handling and focus restore.
 * Resolves true on confirm, false on cancel/dismiss.
 */
export function confirmDialog(options: ConfirmDialogOptions): Promise<boolean> {
  if (confirmDialogActive) return Promise.resolve(false);
  confirmDialogActive = true;

  return new Promise<boolean>((resolve) => {
    const dialog = el<HTMLElement>('confirm-dialog');
    const confirmButton = el<HTMLButtonElement>('confirm-dialog-confirm');
    const cancelButton = el<HTMLButtonElement>('confirm-dialog-cancel');
    const previouslyFocused = document.activeElement as HTMLElement | null;
    let isResolved = false;

    const finish = (confirmed: boolean) => {
      if (isResolved) return;
      isResolved = true;
      document.removeEventListener('keydown', handleKeydown, true);
      confirmButton.removeEventListener('click', handleConfirm);
      cancelButton.removeEventListener('click', handleCancel);
      hide(dialog);
      confirmDialogActive = false;
      // Return focus to the element that opened the dialog.
      previouslyFocused?.focus?.();
      resolve(confirmed);
    };

    const handleConfirm = () => finish(true);
    const handleCancel = () => finish(false);
    const handleKeydown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        finish(false);
        return;
      }
      // Focus trap: keep Tab cycling between the two dialog buttons.
      if (event.key === 'Tab') {
        const focusables = [cancelButton, confirmButton];
        const currentIndex = focusables.indexOf(document.activeElement as HTMLButtonElement);
        event.preventDefault();
        const nextIndex = event.shiftKey
          ? (currentIndex <= 0 ? focusables.length - 1 : currentIndex - 1)
          : (currentIndex + 1) % focusables.length;
        focusables[nextIndex].focus();
      }
    };

    // The step indicator is a leftover of the old multi-step flow.
    hide(el<HTMLElement>('confirm-dialog-step'));
    el<HTMLElement>('confirm-dialog-title').textContent = options.title;
    el<HTMLElement>('confirm-dialog-message').textContent = options.message;
    confirmButton.textContent = options.confirmLabel || '確認';
    confirmButton.disabled = false;
    confirmButton.classList.toggle('confirm-dialog__confirm--danger', options.danger === true);

    confirmButton.addEventListener('click', handleConfirm);
    cancelButton.addEventListener('click', handleCancel);
    document.addEventListener('keydown', handleKeydown, true);

    show(dialog);
    // Safe default: focus Cancel so Enter does not trigger the destructive action.
    cancelButton.focus();
  });
}

// ==================== LSP Log Visibility (DOM-side) ====================

export function syncLspLogVisibility(): void {
  const settingLspLog = el<HTMLInputElement>('setting-lsp-log');
  if (!settingLspLog) return;

  const lspLogPanel = el<HTMLElement>('lsp-log-panel');
  if (!lspLogPanel) return;

  const enabled = loadSettings().lspLogEnabled === true;
  settingLspLog.checked = enabled;
  lspLogPanel.classList.toggle('hidden', !enabled);

  if (!enabled) {
    const lspLogContent = el<HTMLPreElement>('lsp-log-content');
    lspLogContent.textContent = '';
    if (router._current === 'lspLogs') {
      router.navigate('settings', { resetScroll: true });
    }
  }
  if (!enabled && router._current === 'lspSettings') {
    lspLogPanel.classList.add('hidden');
  }
}
