import { isTauriEnv, invoke, tauriReady } from './tauri.js';
import { loadSettings, saveSettings, shouldUseCache, selectedArtworkSource, VALID_ARTWORK_SOURCES, DEFAULT_ARTWORK_SOURCE, } from './settings.js';
import { el, $$, hide, showError, router, updateButtonStates, syncLspLogVisibility, } from './dom.js';
import { renderSearchHistory } from './search-history.js';
import { loadSavedLyrics, initSongsControls, initSongsDockAutoHide } from './songs.js';
import { handleSearch, initInfiniteScroll, setPendingSaltRequest, setCurrentActiveTab, currentSearchData, renderResultList, } from './search.js';
import { syncLspLogZoom, adjustLspLogZoom, syncLspSettings, setBackendLspLogging, appendAppLspLog, viewLspLogs, } from './lsp.js';
import { confirmClearAllCaches, clearAllCaches } from './cache.js';
import { exportLyricsToFile, getExportData } from './export.js';
import { initBackButton, initBackGesture, handleBack } from './back-gesture.js';
import { initBottomMenu } from './bottom-menu.js';
import { initStarfield } from './starfield.js';
// ==================== Salt Player Launch Flow ====================
async function checkSaltLaunchRequest() {
    if (!isTauriEnv())
        return;
    try {
        const request = await invoke('take_salt_launch_request');
        if (!request || !request.title)
            return;
        setPendingSaltRequest(request);
        el('search-title').value = request.title || '';
        el('search-artist').value = request.artist || '';
        router.navigate('search', { resetScroll: true });
        void appendAppLspLog('salt', `launch request received title="${request.title}" artist="${request.artist || ''}"`);
        showError(`Salt Player から「${request.title}」を受け取りました。検索して候補を選ぶと、確認後にこの曲へ Ruby 表示を適用します。`);
    }
    catch (err) {
        console.warn('Salt launch request check failed:', err);
    }
}
// ==================== Init Controls ====================
function initControls() {
    // Cache checkbox
    el('setting-use-cache').checked = shouldUseCache();
    el('setting-use-cache').addEventListener('change', (event) => {
        saveSettings({ useCache: event.target.checked });
    });
    // Artwork source select
    el('setting-artwork-source').value = selectedArtworkSource();
    el('setting-artwork-source').addEventListener('change', (event) => {
        const val = event.target.value;
        const artworkSource = VALID_ARTWORK_SOURCES.has(val) ? val : DEFAULT_ARTWORK_SOURCE;
        saveSettings({ artworkSource: artworkSource });
        if (router.current === 'songs') {
            void loadSavedLyrics();
        }
    });
    // Clear cache button
    el('setting-clear-cache').addEventListener('click', async () => {
        if (await confirmClearAllCaches()) {
            await clearAllCaches();
        }
    });
    // LSP log toggle
    el('setting-lsp-log').checked = loadSettings().lspLogEnabled === true;
    el('setting-lsp-log').addEventListener('change', (event) => {
        const enabled = event.target.checked;
        saveSettings({ lspLogEnabled: enabled });
        syncLspLogVisibility();
        void syncLspSettings();
    });
    syncLspLogVisibility();
    // Show popup toggle
    el('setting-show-popup').checked = loadSettings().showProofPopup !== false;
    el('setting-show-popup').addEventListener('change', (event) => {
        saveSettings({ showProofPopup: event.target.checked });
        void syncLspSettings();
    });
    // Auto launch toggle
    el('setting-auto-launch').checked = loadSettings().autoLaunchUtaBuild !== false;
    el('setting-auto-launch').addEventListener('change', (event) => {
        saveSettings({
            autoLaunchUtaBuild: event.target.checked,
        });
        void syncLspSettings();
    });
    // View LSP log button
    el('setting-view-lsp-log').addEventListener('click', () => {
        router.navigate('lspLogs');
        void viewLspLogs();
    });
    // LSP log back
    el('lsp-log-back-btn').addEventListener('click', () => handleBack());
    // LSP settings back
    el('lsp-settings-back-btn').addEventListener('click', () => handleBack());
    // Goto LSP settings
    el('setting-goto-lsp').addEventListener('click', () => {
        router.navigate('lspSettings', { resetScroll: true });
    });
    // LSP log refresh
    el('lsp-log-refresh-btn').addEventListener('click', () => {
        void viewLspLogs();
    });
    // LSP log zoom
    el('lsp-log-zoom-out').addEventListener('click', () => adjustLspLogZoom(-0.1));
    el('lsp-log-zoom-in').addEventListener('click', () => adjustLspLogZoom(0.1));
    syncLspLogZoom();
    // Font size controls
    $$('[data-size]').forEach((btn) => {
        const button = btn;
        button.addEventListener('click', () => {
            const size = button.dataset.size;
            if (!size)
                return;
            const body = el('lyrics-body').querySelector('.lyricBody > div');
            if (body) {
                body.className = size;
            }
            saveSettings({ fontSize: size });
            updateButtonStates();
        });
    });
    // Theme controls
    $$('[data-theme]').forEach((btn) => {
        const button = btn;
        button.addEventListener('click', () => {
            const theme = button.dataset.theme;
            if (!theme)
                return;
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
            if (!data)
                return;
            const btn = exportBtn;
            btn.disabled = true;
            try {
                await exportLyricsToFile(data);
            }
            catch (err) {
                showError(`エクスポートに失敗しました: ${err}`);
            }
            finally {
                btn.disabled = false;
            }
        });
    }
    // Source tab switching
    el('source-tabs').addEventListener('click', (event) => {
        const tab = event.target.closest('.source-tab');
        if (!tab)
            return;
        const source = tab.dataset.source;
        if (!source)
            return;
        setCurrentActiveTab(source);
        renderResultList(currentSearchData);
    });
}
// ==================== Init ====================
function init() {
    // Search button
    el('search-btn').addEventListener('click', () => {
        void handleSearch();
    });
    // Enter key search
    el('search-title').addEventListener('keydown', (e) => {
        if (e.key === 'Enter')
            void handleSearch();
    });
    el('search-artist').addEventListener('keydown', (e) => {
        if (e.key === 'Enter')
            void handleSearch();
    });
    // Back buttons
    el('back-btn').addEventListener('click', () => {
        handleBack();
    });
    el('back-to-results-btn').addEventListener('click', () => {
        handleBack();
    });
    // Error close
    el('error-close').addEventListener('click', () => hide(el('error-toast')));
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
//# sourceMappingURL=app.js.map