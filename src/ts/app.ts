import { isTauriEnv, invoke, tauriReady } from './tauri.js';
import {
  loadSettings,
  saveSettings,
  shouldUseCache,
  selectedArtworkSource,
  VALID_ARTWORK_SOURCES,
  DEFAULT_ARTWORK_SOURCE,
} from './settings.js';
import {
  el,
  $$,
  hide,
  showError,
  router,
  updateButtonStates,
  syncLspLogVisibility,
} from './dom.js';
import { renderSearchHistory } from './search-history.js';
import { loadSavedLyrics, initSongsControls, initSongsDockAutoHide } from './songs.js';
import {
  handleSearch,
  initInfiniteScroll,
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
    showError(
      `Salt Player から「${request.title}」を受け取りました。検索して候補を選ぶと、確認後にこの曲へ Ruby 表示を適用します。`,
    );
  } catch (err) {
    console.warn('Salt launch request check failed:', err);
  }
}

// ==================== Init Controls ====================

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
      void loadSavedLyrics();
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

  // LSP log refresh
  el<HTMLButtonElement>('lsp-log-refresh-btn').addEventListener('click', () => {
    void viewLspLogs();
  });

  // LSP log zoom
  el<HTMLButtonElement>('lsp-log-zoom-out').addEventListener('click', () => adjustLspLogZoom(-0.1));
  el<HTMLButtonElement>('lsp-log-zoom-in').addEventListener('click', () => adjustLspLogZoom(0.1));

  syncLspLogZoom();

  // Font size controls
  $$('[data-size]').forEach((btn) => {
    const button = btn as HTMLElement;
    button.addEventListener('click', () => {
      const size = button.dataset.size;
      if (!size) return;
      const body = el<HTMLElement>('lyrics-body').querySelector('.lyricBody > div') as HTMLElement | null;
      if (body) {
        body.className = size;
      }
      saveSettings({ fontSize: size as AppSettings['fontSize'] });
      updateButtonStates();
    });
  });

  // Dark mode controls
  $$('[data-dark]').forEach((btn) => {
    const button = btn as HTMLElement;
    button.addEventListener('click', () => {
      const mode = button.dataset.dark;
      if (!mode) return;
      document.body.classList.toggle('dark-mode', mode === 'on');
      saveSettings({ darkMode: mode as AppSettings['darkMode'] });
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
        await exportLyricsToFile(data);
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
  // Search button
  el<HTMLButtonElement>('search-btn').addEventListener('click', () => {
    void handleSearch();
  });

  // Enter key search
  el<HTMLInputElement>('search-title').addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Enter') void handleSearch();
  });
  el<HTMLInputElement>('search-artist').addEventListener('keydown', (e: KeyboardEvent) => {
    if (e.key === 'Enter') void handleSearch();
  });

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
  initControls();
  initSongsControls();

  // Bottom menu
  initBottomMenu();
  initSongsDockAutoHide();

  // Search history
  renderSearchHistory();

  // Results infinite scroll
  initInfiniteScroll();

  // Android back button support
  initBackButton();
  initBackGesture();

  // Apply saved dark mode
  const settings = loadSettings();
  const effectiveDarkMode = settings.darkMode || 'on';
  if (effectiveDarkMode === 'on') {
    document.body.classList.add('dark-mode');
  }
  if (!settings.darkMode) {
    saveSettings({ darkMode: 'on' });
  }

  // Sync button states
  updateButtonStates();

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
