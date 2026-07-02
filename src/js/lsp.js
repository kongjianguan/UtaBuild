import { invoke, tauriReady } from './tauri.js';
import { loadSettings } from './settings.js';
import { el, showLoading, hideLoading } from './dom.js';
// ==================== LSP State ====================
export let lspLogZoom = 1;
// ==================== Backend LSP Settings ====================
export async function setBackendLspLogging(enabled) {
    try {
        await tauriReady;
        await invoke('set_lsp_logging_enabled', { enabled });
    }
    catch (err) {
        console.warn('Failed to sync lsp logging setting:', err);
    }
}
export async function syncLspSettings() {
    const settings = loadSettings();
    const lspSettings = {
        lspLogEnabled: settings.lspLogEnabled === true,
        showProofPopup: settings.showProofPopup !== false,
        autoLaunchUtaBuild: settings.autoLaunchUtaBuild !== false,
    };
    try {
        await tauriReady;
        await invoke('set_lsp_settings', { settings: lspSettings });
    }
    catch (err) {
        console.warn('Failed to sync lsp settings:', err);
    }
}
export async function appendAppLspLog(scope, message) {
    if (loadSettings().lspLogEnabled !== true)
        return;
    try {
        await tauriReady;
        await invoke('append_lsp_log', { scope, message });
    }
    catch (err) {
        console.warn('Failed to append lsp log:', err);
    }
}
// ==================== View LSP Logs ====================
export async function viewLspLogs() {
    showLoading();
    el('lsp-log-content').textContent = 'LSPログを読み込み中です...';
    void appendAppLspLog('settings', 'view lsp logs');
    try {
        await tauriReady;
        const logs = await invoke('get_lsp_logs');
        el('lsp-log-content').textContent =
            logs && String(logs).trim() ? String(logs) : 'LSPログはまだありません';
    }
    catch (err) {
        console.error('Read lsp logs error:', err);
        el('lsp-log-content').textContent = `LSPログの読み込みに失敗しました: ${err}`;
    }
    finally {
        hideLoading();
    }
}
// ==================== LSP Log Zoom ====================
export function syncLspLogZoom() {
    el('lsp-log-content').style.setProperty('--lsp-log-font-scale', String(lspLogZoom));
    el('lsp-log-zoom-label').textContent = `${Math.round(lspLogZoom * 100)}%`;
}
export function adjustLspLogZoom(delta) {
    lspLogZoom = Math.min(1.8, Math.max(0.7, Number((lspLogZoom + delta).toFixed(2))));
    syncLspLogZoom();
}
//# sourceMappingURL=lsp.js.map