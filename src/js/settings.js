const STORAGE_KEY = 'utabuild-settings';
export const FONT_SIZE_MIN = 14;
export const FONT_SIZE_MAX = 32;
export const DEFAULT_FONT_SIZE = 17;
// 'mygo' is intentionally excluded: the theme is hidden until its quality is
// fixed. A stored 'mygo' value normalizes to 'dark' on the next load.
export const VALID_THEMES = new Set(['dark', 'light']);
export const VALID_ARTWORK_SOURCES = new Set(['auto', 'utaten', 'qq', 'netease']);
export const DEFAULT_USE_CACHE = true;
export const DEFAULT_ARTWORK_SOURCE = 'auto';
const LEGACY_FONT_SIZES = {
    small: FONT_SIZE_MIN,
    medium: DEFAULT_FONT_SIZE,
    large: 21,
};
export function clampFontSize(value) {
    if (!Number.isFinite(value))
        return DEFAULT_FONT_SIZE;
    return Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, Math.round(value)));
}
export function defaultFontSizeForViewport(viewportWidth = typeof window === 'undefined' ? 1024 : window.innerWidth) {
    if (viewportWidth <= 375)
        return FONT_SIZE_MIN;
    if (viewportWidth <= 768)
        return 15;
    return DEFAULT_FONT_SIZE;
}
function normalizeFontSize(rawFontSize) {
    if (typeof rawFontSize === 'string' && rawFontSize in LEGACY_FONT_SIZES) {
        return LEGACY_FONT_SIZES[rawFontSize];
    }
    if (typeof rawFontSize !== 'number' && typeof rawFontSize !== 'string')
        return undefined;
    const parsed = typeof rawFontSize === 'number' ? rawFontSize : Number(rawFontSize);
    return Number.isFinite(parsed) ? clampFontSize(parsed) : undefined;
}
function normalizeSettings(rawSettings = {}) {
    const settings = {};
    const fontSize = normalizeFontSize(rawSettings.fontSize);
    if (fontSize !== undefined)
        settings.fontSize = fontSize;
    if (VALID_THEMES.has(rawSettings.theme)) {
        settings.theme = rawSettings.theme;
    }
    else {
        settings.theme = 'dark';
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