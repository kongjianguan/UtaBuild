const STORAGE_KEY = 'utabuild-settings';
export const VALID_FONT_SIZES = new Set(['small', 'medium', 'large']);
export const VALID_DARK_MODES = new Set(['on', 'off']);
export const VALID_ARTWORK_SOURCES = new Set(['auto', 'utaten', 'qq', 'netease']);
export const DEFAULT_USE_CACHE = true;
export const DEFAULT_ARTWORK_SOURCE = 'auto';
function normalizeSettings(rawSettings = {}) {
    const settings = {};
    if (VALID_FONT_SIZES.has(rawSettings.fontSize)) {
        settings.fontSize = rawSettings.fontSize;
    }
    if (VALID_DARK_MODES.has(rawSettings.darkMode)) {
        settings.darkMode = rawSettings.darkMode;
    }
    else {
        settings.darkMode = 'on';
    }
    if (typeof rawSettings.useCache === 'boolean') {
        settings.useCache = rawSettings.useCache;
    }
    if (VALID_ARTWORK_SOURCES.has(rawSettings.artworkSource)) {
        settings.artworkSource = rawSettings.artworkSource;
    }
    if (typeof rawSettings.lspLogEnabled === 'boolean') {
        settings.lspLogEnabled = rawSettings.lspLogEnabled;
    }
    if (typeof rawSettings.showProofPopup === 'boolean') {
        settings.showProofPopup = rawSettings.showProofPopup;
    }
    if (typeof rawSettings.autoLaunchUtaBuild === 'boolean') {
        settings.autoLaunchUtaBuild = rawSettings.autoLaunchUtaBuild;
    }
    return settings;
}
export function loadSettings() {
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
    }
    catch {
        return {};
    }
}
export function saveSettings(settings) {
    try {
        const current = loadSettings();
        const merged = normalizeSettings({ ...current, ...settings });
        localStorage.setItem(STORAGE_KEY, JSON.stringify(merged));
    }
    catch (e) {
        console.warn('Failed to save settings:', e);
    }
}
export function shouldUseCache() {
    const settings = loadSettings();
    return settings.useCache ?? DEFAULT_USE_CACHE;
}
export function selectedArtworkSource() {
    const settings = loadSettings();
    return VALID_ARTWORK_SOURCES.has(settings.artworkSource)
        ? settings.artworkSource
        : DEFAULT_ARTWORK_SOURCE;
}
//# sourceMappingURL=settings.js.map