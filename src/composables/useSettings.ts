import { reactive } from 'vue';
import type { AppSettings, ArtworkSource, ThemeMode } from '../types';

const STORAGE_KEY = 'utabuild-settings';
export const FONT_SIZE_MIN = 14;
export const FONT_SIZE_MAX = 32;
export const DEFAULT_FONT_SIZE = 17;
export const VALID_THEMES = new Set<ThemeMode>(['dark', 'light']);
export const VALID_ARTWORK_SOURCES = new Set<ArtworkSource>(['auto', 'utaten', 'qq', 'netease']);
export const DEFAULT_USE_CACHE = true;
export const DEFAULT_ARTWORK_SOURCE: ArtworkSource = 'auto';

export function clampFontSize(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_FONT_SIZE;
  return Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, Math.round(value)));
}

export function defaultFontSizeForViewport(
  viewportWidth: number = typeof window === 'undefined' ? 1024 : window.innerWidth,
): number {
  if (viewportWidth <= 375) return FONT_SIZE_MIN;
  if (viewportWidth <= 768) return 15;
  return DEFAULT_FONT_SIZE;
}

function normalizeFontSize(rawFontSize: unknown): number | undefined {
  if (rawFontSize === 'small') return FONT_SIZE_MIN;
  if (rawFontSize === 'medium') return DEFAULT_FONT_SIZE;
  if (rawFontSize === 'large') return 21;

  if (typeof rawFontSize !== 'number' && typeof rawFontSize !== 'string') return undefined;
  const parsed = typeof rawFontSize === 'number' ? rawFontSize : Number(rawFontSize);
  return Number.isFinite(parsed) ? clampFontSize(parsed) : undefined;
}

function normalizeSettings(rawSettings: Partial<AppSettings> = {}): AppSettings {
  const settings: AppSettings = {};

  const fontSize = normalizeFontSize(rawSettings.fontSize);
  if (fontSize !== undefined) settings.fontSize = fontSize;

  settings.theme = VALID_THEMES.has(rawSettings.theme as ThemeMode)
    ? (rawSettings.theme as ThemeMode)
    : 'dark';

  if (typeof rawSettings.showRuby === 'boolean') {
    settings.showRuby = rawSettings.showRuby;
  }

  if (typeof rawSettings.useCache === 'boolean') {
    settings.useCache = rawSettings.useCache;
  }

  if (VALID_ARTWORK_SOURCES.has(rawSettings.artworkSource as ArtworkSource)) {
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

function readStoredSettings(): AppSettings {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (!saved) return normalizeSettings({});
    const parsed = JSON.parse(saved) as Partial<AppSettings>;
    const normalized = normalizeSettings(parsed);
    if (JSON.stringify(parsed) !== JSON.stringify(normalized)) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(normalized));
    }
    return normalized;
  } catch {
    return normalizeSettings({});
  }
}

const state = reactive<AppSettings>(readStoredSettings());

function applyThemeAttr(theme: ThemeMode): void {
  if (typeof document !== 'undefined' && document.body) {
    document.body.setAttribute('data-theme', theme);
  }
}

export function initSettingsTheme(): void {
  applyThemeAttr(state.theme || 'dark');
}

export function updateSettings(patch: Partial<AppSettings>): void {
  const merged = normalizeSettings({ ...state, ...patch });
  Object.assign(state, merged);
  if (merged.theme) {
    applyThemeAttr(merged.theme);
  }
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(merged));
  } catch (err) {
    console.warn('Failed to save settings:', err);
  }
}

export function effectiveFontSize(viewportWidth?: number): number {
  return state.fontSize ?? defaultFontSizeForViewport(viewportWidth);
}

export function effectiveUseCache(): boolean {
  return state.useCache ?? DEFAULT_USE_CACHE;
}

export function effectiveArtworkSource(): ArtworkSource {
  return state.artworkSource ?? DEFAULT_ARTWORK_SOURCE;
}

export function effectiveShowRuby(): boolean {
  return state.showRuby ?? true;
}

export function useSettings() {
  return {
    settings: state,
    updateSettings,
    effectiveFontSize,
    effectiveUseCache,
    effectiveArtworkSource,
    effectiveShowRuby,
  };
}
