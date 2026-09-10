import { isTauriEnv, invoke, tauriReady } from './tauri.js';
import {
  loadSettings,
  saveSettings,
  shouldUseCache,
  selectedArtworkSource,
  VALID_ARTWORK_SOURCES,
  DEFAULT_ARTWORK_SOURCE,
  clampFontSize,
  defaultFontSizeForViewport,
  FONT_SIZE_MIN,
} from './settings.js';
import {
  el,
  $$,
  hide,
  showError,
  showInfo,
  showSuccess,
  router,
  updateButtonStates,
  syncLspLogVisibility,
  syncFontSizeControls,
} from './dom.js';
import { renderSearchHistory } from './search-history.js';
import { loadSavedLyrics, initSongsControls, initSongsDockAutoHide } from './songs.js';
import {
  handleSearch,
  initInfiniteScroll,
  initSourceTabKeyboardNav,
  setPendingSaltRequest,
  setCurrentActiveTab,
  currentSearchData,
  renderResultList,
} from './search.js';
import {
  syncLspLogZoom,
  adjustLspLogZoom,
  syncLspSettings,
  setBackendLspLogging,
  appendAppLspLog,
  viewLspLogs,
} from './lsp.js';
import { confirmClearAllCaches, clearAllCaches } from './cache.js';
import { exportLyricsToFile, getExportData } from './export.js';
import { initBackButton, initBackGesture, handleBack } from './back-gesture.js';
import { initBottomMenu } from './bottom-menu.js';
import { initStarfield } from './starfield.js';
import { setLyricsFontSize } from './ruby.js';
import type { AppSettings } from './types.js';

// ==================== Salt Player Launch Flow ====================

async function checkSaltLaunchRequest(): Promise<void> {
  if (!isTauriEnv()) return;

  try {
    const request = await invoke<{ title?: string; artist?: string | null } | null>(
      'take_salt_launch_request',
    );
    if (!request || !request.title) return;

    setPendingSaltRequest(request);
    el<HTMLInputElement>('search-title').value = request.title || '';
    el<HTMLInputElement>('search-artist').value = request.artist || '';
    router.navigate('search', { resetScroll: true });
    void appendAppLspLog(
      'salt',
      `launch request received title="${request.title}" artist="${request.artist || ''}"`,
    );
    showInfo(
      `Salt Player から「${request.title}」を受け取りました。検索して候補を選ぶと、確認後にこの曲へ Ruby 表示を適用します。`,
    );
  } catch (err) {
    console.warn('Salt launch request check failed:', err);
  }
}

// ==================== Init Controls ====================

function initSearchForm(): void {
  const form = el<HTMLFormElement>('search-form');
  const titleInput = el<HTMLInputElement>('search-title');
  const artistInput = el<HTMLInputElement>('search-artist');
  const scrim = el<HTMLButtonElement>('search-scrim');

  const isNarrowSearch = (): boolean => window.matchMedia('(max-width: 768px)').matches;

  const setExpanded = (expanded: boolean): void => {
    const active = expanded && isNarrowSearch();
    form.dataset.expanded = String(active);
    form.setAttribute('aria-expanded', String(active));
    form.classList.toggle('is-expanded', active);
    scrim.classList.toggle('hidden', !active);
    scrim.setAttribute('aria-hidden', String(!active));
  };

  titleInput.addEventListener('focus', () => {
    if (isNarrowSearch()) setExpanded(true);
  });
  artistInput.addEventListener('focus', () => {
    if (isNarrowSearch()) setExpanded(true);
  });

  scrim.addEventListener('click', () => setExpanded(false));
  form.addEventListener('submit', () => {
    if (titleInput.value.trim()) setExpanded(false);
  });
  document.addEventListener('pointerdown', (event) => {
    if (form.dataset.expanded !== 'true') return;
    if (!form.contains(event.target as Node)) setExpanded(false);
  });
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && form.dataset.expanded === 'true') {
      event.preventDefault();
      setExpanded(false);
    }
  });

  window.addEventListener('resize', () => {
    if (!isNarrowSearch()) setExpanded(false);
  });
}

function initLyricsMenu(): void {
  const toggle = el<HTMLButtonElement>('lyrics-more-btn');
  const controls = el<HTMLElement>('lyrics-controls');
  const isNarrowLyrics = (): boolean => window.matchMedia('(max-width: 768px)').matches;

  let backdrop = document.querySelector<HTMLElement>('.lyrics-controls-backdrop');
  if (!backdrop) {
    backdrop = document.createElement('div');
    backdrop.className = 'lyrics-controls-backdrop';
    document.body.appendChild(backdrop);
    backdrop.addEventListener('click', () => setOpen(false));
  }

  const setOpen = (open: boolean): void => {
    const active = open && isNarrowLyrics();
    toggle.setAttribute('aria-expanded', String(active));
    toggle.classList.toggle('is-open', active);
    controls.classList.toggle('is-open', active);
    backdrop?.classList.toggle('is-open', active);
  };

  toggle.addEventListener('click', () => {
    setOpen(toggle.getAttribute('aria-expanded') !== 'true');
  });

  document.addEventListener('pointerdown', (event) => {
    if (toggle.getAttribute('aria-expanded') !== 'true') return;
    const target = event.target as Node;
    if (!toggle.contains(target) && !controls.contains(target)) setOpen(false);
  });

  document.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape' || toggle.getAttribute('aria-expanded') !== 'true') return;
    event.preventDefault();
    setOpen(false);
    toggle.focus();
  });

  window.addEventListener('resize', () => {
    if (!isNarrowLyrics()) setOpen(false);
  });
}

function initViewportGuards(): void {
  document.addEventListener(
    'touchmove',
    (event) => {
      if (event.touches.length > 1) event.preventDefault();
    },
    { passive: false },
  );
  document.addEventListener('gesturestart', (event) => event.preventDefault());
  window.addEventListener(
    'wheel',
    (event) => {
      if (event.ctrlKey || event.metaKey) event.preventDefault();
    },
    { passive: false },
  );
  window.addEventListener('keydown', (event) => {
    if (!(event.ctrlKey || event.metaKey)) return;
    if (['+', '-', '=', '0'].includes(event.key)) event.preventDefault();
  });
}

function applyFontSize(value: number): void {
  const fontSize = clampFontSize(value);
  setLyricsFontSize(fontSize);
  saveSettings({ fontSize });
  syncFontSizeControls(fontSize);
}

function initFontSizeControl(): void {
  const range = el<HTMLInputElement>('font-size-range');
  const valueInput = el<HTMLInputElement>('font-size-value');
  let isEditingNumber = false;
  syncFontSizeControls();

  range.addEventListener('input', () => {
    applyFontSize(Number(range.value));
  });

  const commitNumberInput = (): void => {
    isEditingNumber = false;
    const rawValue = valueInput.value.trim();
    const parsed = Number(rawValue);
    if (!rawValue || !Number.isFinite(parsed)) {
      syncFontSizeControls();
      return;
    }
    applyFontSize(parsed);
  };

  valueInput.addEventListener('beforeinput', (event) => {
    if (event.inputType.startsWith('insert') || event.inputType.startsWith('delete')) {
      isEditingNumber = true;
    }
  });
  valueInput.addEventListener('input', () => {
    const rawValue = valueInput.value.trim();
    const parsed = Number(rawValue);
    if (!rawValue || !Number.isFinite(parsed)) return;

    // Keep a short out-of-range draft intact while the user types the next digit.
    if (isEditingNumber && parsed < FONT_SIZE_MIN && rawValue.length < 3) return;
    isEditingNumber = false;
    applyFontSize(parsed);
  });
  valueInput.addEventListener('change', commitNumberInput);
  valueInput.addEventListener('blur', commitNumberInput);

  window.addEventListener('resize', () => {
    if (loadSettings().fontSize !== undefined) return;
    const fontSize = defaultFontSizeForViewport();
    setLyricsFontSize(fontSize);
    syncFontSizeControls(fontSize);
  });
}

function initControls(): void {
  // Cache checkbox
  el<HTMLInputElement>('setting-use-cache').checked = shouldUseCache();
  el<HTMLInputElement>('setting-use-cache').addEventListener('change', (event) => {
    saveSettings({ useCache: (event.target as HTMLInputElement).checked });
  });

  // Artwork source select
  el<HTMLSelectElement>('setting-artwork-source').value = selectedArtworkSource();
  el<HTMLSelectElement>('setting-artwork-source').addEventListener('change', (event) => {
    const val = (event.target as HTMLSelectElement).value;
    const artworkSource = VALID_ARTWORK_SOURCES.has(val as "auto" | "utaten" | "qq" | "netease") ? val : DEFAULT_ARTWORK_SOURCE;
    saveSettings({ artworkSource: artworkSource as AppSettings['artworkSource'] });
    if (router.current === 'songs') {
      void loadSavedLyrics({ refreshArtwork: true });
    }
  });

  // Clear cache button
  el<HTMLButtonElement>('setting-clear-cache').addEventListener('click', async () => {
    if (await confirmClearAllCaches()) {
      await clearAllCaches();
    }
  });

  // LSP log toggle
  el<HTMLInputElement>('setting-lsp-log').checked = loadSettings().lspLogEnabled === true;
  el<HTMLInputElement>('setting-lsp-log').addEventListener('change', (event) => {
    const enabled = (event.target as HTMLInputElement).checked;
    saveSettings({ lspLogEnabled: enabled });
    syncLspLogVisibility();
    void syncLspSettings();
  });
  syncLspLogVisibility();

  // Show popup toggle
  el<HTMLInputElement>('setting-show-popup').checked = loadSettings().showProofPopup !== false;
  el<HTMLInputElement>('setting-show-popup').addEventListener('change', (event) => {
    saveSettings({ showProofPopup: (event.target as HTMLInputElement).checked });
    void syncLspSettings();
  });

  // Auto launch toggle
  el<HTMLInputElement>('setting-auto-launch').checked = loadSettings().autoLaunchUtaBuild !== false;
  el<HTMLInputElement>('setting-auto-launch').addEventListener('change', (event) => {
    saveSettings({
      autoLaunchUtaBuild: (event.target as HTMLInputElement).checked,
    });
    void syncLspSettings();
  });

  // View LSP log button
  el<HTMLButtonElement>('setting-view-lsp-log').addEventListener('click', () => {
    router.navigate('lspLogs');
    void viewLspLogs();
  });

  // LSP log back
  el<HTMLButtonElement>('lsp-log-back-btn').addEventListener('click', () => handleBack());

  // LSP settings back
  el<HTMLButtonElement>('lsp-settings-back-btn').addEventListener('click', () => handleBack());

  // Goto LSP settings
  el<HTMLButtonElement>('setting-goto-lsp').addEventListener('click', () => {
    router.navigate('lspSettings', { resetScroll: true });
  });

  // About subpage
  el<HTMLButtonElement>('setting-goto-about').addEventListener('click', () => {
    router.navigate('settingsAbout', { animate: true, resetScroll: true });
  });
  el<HTMLButtonElement>('settings-about-back-btn').addEventListener('click', () => handleBack());

  // LSP log refresh
  el<HTMLButtonElement>('lsp-log-refresh-btn').addEventListener('click', () => {
    void viewLspLogs();
  });

  // LSP log zoom
  el<HTMLButtonElement>('lsp-log-zoom-out').addEventListener('click', () => adjustLspLogZoom(-0.1));
  el<HTMLButtonElement>('lsp-log-zoom-in').addEventListener('click', () => adjustLspLogZoom(0.1));

  syncLspLogZoom();

  // Font size controls
  initFontSizeControl();

  // Theme controls
  $$('.lyrics-controls [data-theme]').forEach((btn) => {
    const button = btn as HTMLElement;
    button.addEventListener('click', () => {
      const theme = button.dataset.theme as 'dark' | 'light' | 'mygo';
      if (!theme) return;
      document.body.setAttribute('data-theme', theme);
      saveSettings({ theme });
      updateButtonStates();
    });
  });

  // Export lyrics button
  const exportBtn = document.getElementById('export-lyrics-btn');
  if (exportBtn) {
    exportBtn.addEventListener('click', async () => {
      const data = getExportData();
      if (!data) return;
      const btn = exportBtn as HTMLButtonElement;
      btn.disabled = true;
      try {
        const exported = await exportLyricsToFile(data);
        if (exported) {
          showSuccess('エクスポートしました');
        } else {
          showInfo('エクスポートをキャンセルしました');
        }
      } catch (err) {
        showError(`エクスポートに失敗しました: ${err}`);
      } finally {
        btn.disabled = false;
      }
    });
  }

  // Source tab switching
  el<HTMLElement>('source-tabs').addEventListener('click', (event) => {
    const tab = (event.target as HTMLElement).closest('.source-tab') as HTMLElement | null;
    if (!tab) return;

    const source = tab.dataset.source;
    if (!source) return;

    setCurrentActiveTab(source);
    renderResultList(currentSearchData);
  });
}

// ==================== Init ====================

function init(): void {
  // Native form submission keeps mouse, keyboard and IME flows consistent.
  el<HTMLFormElement>('search-form').addEventListener('submit', (event) => {
    event.preventDefault();
    void handleSearch();
  });

  const submitOnEnter = (event: KeyboardEvent) => {
    if (event.key !== 'Enter' || event.isComposing) return;
    event.preventDefault();
    void handleSearch();
  };
  el<HTMLInputElement>('search-title').addEventListener('keydown', submitOnEnter);
  el<HTMLInputElement>('search-artist').addEventListener('keydown', submitOnEnter);

  // Back buttons
  el<HTMLButtonElement>('back-btn').addEventListener('click', () => {
    handleBack();
  });
  el<HTMLButtonElement>('back-to-results-btn').addEventListener('click', () => {
    handleBack();
  });

  // Error close
  el<HTMLButtonElement>('error-close').addEventListener('click', () => hide(el<HTMLElement>('error-toast')));

  // Controls
  initSearchForm();
  initLyricsMenu();
  initViewportGuards();
  initControls();
  initSongsControls();

  // Bottom menu
  initBottomMenu();
  initSongsDockAutoHide();

  // Search history
  renderSearchHistory();

  // Results infinite scroll
  initInfiniteScroll();
  initSourceTabKeyboardNav();

  // Android back button support
  initBackButton();
  initBackGesture();

  // Apply saved theme
  const settings = loadSettings();
  const effectiveTheme = settings.theme || 'dark';
  document.body.setAttribute('data-theme', effectiveTheme);
  if (!settings.theme) {
    saveSettings({ theme: 'dark' });
  }

  // Sync button states
  updateButtonStates();

  // Init starfield canvas
  initStarfield();

  void setBackendLspLogging(settings.lspLogEnabled === true);
  void syncLspSettings();
  void tauriReady.then(checkSaltLaunchRequest);
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') {
      void checkSaltLaunchRequest();
    }
  });

  // Browser mode notice
  if (!isTauriEnv()) {
    console.log('\u{1F98A} UtaBuild Browser Mode - 使用Mock数据调试UI');
  }
  console.log('UtaBuild initialized');
}

document.addEventListener('DOMContentLoaded', init);
