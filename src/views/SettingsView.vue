<script setup lang="ts">
import { computed } from 'vue';
import type { ArtworkSource, ThemeMode } from '../types';
import {
  clampFontSize,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  useSettings,
} from '../composables/useSettings';
import { useLyricsApp } from '../composables/useLyricsApp';
import { useRouter } from '../composables/useRouter';
import AppIcon from '../components/AppIcon.vue';

const {
  settings,
  updateSettings,
  effectiveFontSize,
  effectiveUseCache,
  effectiveArtworkSource,
  effectiveShowRuby,
} = useSettings();
const { loadSavedLyrics, clearAllCaches } = useLyricsApp();
const { routerState, navigateTo } = useRouter();

const isAndroid = typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent);

const activeTheme = computed(() => settings.theme || 'dark');
const fontSize = computed(() => effectiveFontSize());
const useCache = computed(() => effectiveUseCache());
const artworkSource = computed(() => effectiveArtworkSource());
const showRuby = computed(() => effectiveShowRuby());

const ARTWORK_OPTIONS: Array<{ value: ArtworkSource; label: string }> = [
  { value: 'auto', label: '自動' },
  { value: 'utaten', label: 'UtaTen' },
  { value: 'qq', label: 'QQ音楽' },
  { value: 'netease', label: 'NetEase' },
];

function setTheme(theme: ThemeMode): void {
  updateSettings({ theme });
}

function stepDefaultFontSize(delta: number): void {
  updateSettings({ fontSize: clampFontSize(fontSize.value + delta) });
}

function toggleRubyDefault(): void {
  updateSettings({ showRuby: !showRuby.value });
}

function toggleUseCache(event: Event): void {
  updateSettings({ useCache: (event.target as HTMLInputElement).checked });
}

function setArtworkSource(source: ArtworkSource): void {
  updateSettings({ artworkSource: source });
  if (routerState.current === 'songs') {
    void loadSavedLyrics({ refreshArtwork: true });
  }
}
</script>

<template>
  <div class="settings-view">
    <header class="view-header view-header--top-level">
      <div class="view-header__title">
        <h1>設定</h1>
        <p>表示・検索・アプリ動作のカスタマイズ</p>
      </div>
    </header>

    <div class="view-scroll settings-groups">
      <section aria-labelledby="group-appearance">
        <div id="group-appearance" class="section-label">外観・表示</div>
        <div class="surface-group">
          <div class="pref-row pref-row--static">
            <div class="pref-row__text">
              <span class="pref-row__label">テーマ</span>
              <span class="pref-row__desc">画面全体と歌詞ビューの配色を切り替えます。</span>
            </div>
            <div class="segmented-pill">
              <button
                type="button"
                :class="{ active: activeTheme === 'dark' }"
                @click="setTheme('dark')"
              >
                <AppIcon name="moon" :size="14" />
                <span>ダーク</span>
              </button>
              <button
                type="button"
                :class="{ active: activeTheme === 'light' }"
                @click="setTheme('light')"
              >
                <AppIcon name="sun" :size="14" />
                <span>ライト</span>
              </button>
            </div>
          </div>

          <div class="pref-row pref-row--static">
            <div class="pref-row__text">
              <span class="pref-row__label">既定の文字サイズ</span>
              <span class="pref-row__desc">歌詞ページの標準フォントサイズ（{{ FONT_SIZE_MIN }}–{{ FONT_SIZE_MAX }}px）。</span>
            </div>
            <div class="size-stepper">
              <button
                type="button"
                class="icon-button icon-button--sm"
                :disabled="fontSize <= FONT_SIZE_MIN"
                aria-label="文字を小さく"
                @click="stepDefaultFontSize(-1)"
              >
                <AppIcon name="minus" :size="15" />
              </button>
              <span class="size-stepper__val">{{ fontSize }}px</span>
              <button
                type="button"
                class="icon-button icon-button--sm"
                :disabled="fontSize >= FONT_SIZE_MAX"
                aria-label="文字を大きく"
                @click="stepDefaultFontSize(1)"
              >
                <AppIcon name="plus" :size="15" />
              </button>
            </div>
          </div>

          <label class="pref-row">
            <div class="pref-row__text">
              <span class="pref-row__label">ふりがな（ルビ）を表示</span>
              <span class="pref-row__desc">オフにすると漢字のみの歌詞表示に切り替わります。</span>
            </div>
            <input
              type="checkbox"
              class="miuix-switch"
              :checked="showRuby"
              @change="toggleRubyDefault"
            />
          </label>
        </div>
      </section>

      <section aria-labelledby="group-data">
        <div id="group-data" class="section-label">検索・キャッシュ</div>
        <div class="surface-group">
          <label class="pref-row">
            <div class="pref-row__text">
              <span class="pref-row__label">キャッシュを使う</span>
              <span class="pref-row__desc">1日以内にヒットした検索・歌詞キャッシュをそのまま利用します。</span>
            </div>
            <input
              type="checkbox"
              class="miuix-switch"
              :checked="useCache"
              @change="toggleUseCache"
            />
          </label>

          <div class="pref-row pref-row--stacked">
            <div class="pref-row__text">
              <span class="pref-row__label">ジャケット画像ソース</span>
              <span class="pref-row__desc">自動では UtaTen → QQ音楽 → NetEase の順で取得します。</span>
            </div>
            <div class="segmented-pill artwork-pills" role="group" aria-label="ジャケット画像ソース">
              <button
                v-for="opt in ARTWORK_OPTIONS"
                :key="opt.value"
                type="button"
                :class="{ active: artworkSource === opt.value }"
                @click="setArtworkSource(opt.value)"
              >
                {{ opt.label }}
              </button>
            </div>
          </div>

          <button
            type="button"
            class="pref-row pref-row--button"
            @click="void clearAllCaches()"
          >
            <div class="pref-row__text">
              <span class="pref-row__label pref-row__label--danger">キャッシュを削除</span>
              <span class="pref-row__desc">検索結果とメモリキャッシュのみ削除します。保存済みの曲は残ります。</span>
            </div>
            <AppIcon name="trash" :size="18" class="pref-row__icon-danger" />
          </button>
        </div>
      </section>

      <section aria-labelledby="group-system">
        <div id="group-system" class="section-label">アプリ情報</div>
        <div class="surface-group">
          <button
            v-if="isAndroid"
            type="button"
            class="pref-row pref-row--button"
            @click="navigateTo('lspSettings', { resetScroll: true })"
          >
            <div class="pref-row__text">
              <span class="pref-row__label">LSP 設定</span>
              <span class="pref-row__desc">LSP ログ、Hi UtaBuild 通知、自動遷移の設定。</span>
            </div>
            <AppIcon name="chevron-right" :size="18" class="pref-row__chevron" />
          </button>

          <button
            type="button"
            class="pref-row pref-row--button"
            @click="navigateTo('settingsAbout', { resetScroll: true })"
          >
            <div class="pref-row__text">
              <span class="pref-row__label">このアプリについて</span>
              <span class="pref-row__desc">UtaBuild の概要と主な機能。</span>
            </div>
            <AppIcon name="chevron-right" :size="18" class="pref-row__chevron" />
          </button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped lang="scss">
.settings-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.settings-groups {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.pref-row {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 68px;
  padding: 14px 16px;
  border: 0;
  border-bottom: 1px solid var(--border-subtle);
  border-radius: 0;
  background: transparent;
  text-align: left;
  cursor: pointer;

  &:last-child {
    border-bottom: 0;
  }

  &--static {
    cursor: default;
  }

  &--stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
    cursor: default;
  }

  &--button:hover {
    background: var(--surface-hover);
  }

  &__text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  &__label {
    color: var(--text);
    font-size: var(--font-md);
    font-weight: 600;
    line-height: 1.35;

    &--danger {
      color: var(--danger);
    }
  }

  &__desc {
    color: var(--text-secondary);
    font-size: var(--font-xs);
    line-height: 1.45;
  }

  &__chevron {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  &__icon-danger {
    color: var(--danger);
    flex-shrink: 0;
  }
}

.size-stepper {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px;
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  border: 1px solid var(--border-subtle);
  flex-shrink: 0;

  &__val {
    min-width: 44px;
    text-align: center;
    font-size: var(--font-xs);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
}

.artwork-pills {
  width: 100%;
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

.miuix-switch {
  appearance: none;
  -webkit-appearance: none;
  position: relative;
  width: 50px;
  height: 30px;
  flex: 0 0 50px;
  border: 0;
  border-radius: var(--radius-pill);
  background: var(--bg-secondary);
  box-shadow: inset 0 0 0 1px var(--border);
  cursor: pointer;
  transition: background var(--motion-base), box-shadow var(--motion-base);

  &::before {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--text-secondary);
    box-shadow: var(--shadow-sm);
    transition: transform var(--motion-base) var(--ease-spring), background var(--motion-base);
  }

  &:checked {
    background: var(--primary);
    box-shadow: none;

    &::before {
      transform: translateX(20px);
      background: var(--primary-content);
    }
  }

  &:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }
}
</style>
