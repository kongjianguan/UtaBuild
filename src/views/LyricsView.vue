<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import {
  clampFontSize,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  useSettings,
} from '../composables/useSettings';
import { copyTextToClipboard, formatPlainText, normalizeCoverUrl } from '../services/formatters';
import { useLyricsApp } from '../composables/useLyricsApp';
import { useRouter } from '../composables/useRouter';
import { useFeedback } from '../composables/useFeedback';
import AppIcon from '../components/AppIcon.vue';
import LyricRenderer from '../components/LyricRenderer.vue';

const { settings, updateSettings, effectiveFontSize, effectiveShowRuby } = useSettings();
const { currentLyrics, isLoadingLyrics, openExportForCurrentLyrics } = useLyricsApp();
const { navigateBack } = useRouter();
const { showSuccess, showError } = useFeedback();

const scrollRef = ref<HTMLElement | null>(null);
const settingsOpen = ref(false);
const viewportWidth = ref(typeof window !== 'undefined' ? window.innerWidth : 1024);

const fontSize = computed(() => effectiveFontSize(viewportWidth.value));
const showRuby = computed(() => effectiveShowRuby());
const activeTheme = computed(() => settings.theme || 'dark');
const coverUrl = computed(() => normalizeCoverUrl(currentLyrics.value?.cover_url));

const rangeProgress = computed(() => {
  return `${((fontSize.value - FONT_SIZE_MIN) / (FONT_SIZE_MAX - FONT_SIZE_MIN)) * 100}%`;
});

function setFontSize(next: number): void {
  updateSettings({ fontSize: clampFontSize(next) });
}

function stepFontSize(delta: number): void {
  setFontSize(fontSize.value + delta);
}

function setTheme(theme: 'dark' | 'light'): void {
  updateSettings({ theme });
}

function toggleRuby(): void {
  updateSettings({ showRuby: !showRuby.value });
}

async function copyPlainLyrics(): Promise<void> {
  const annotations = currentLyrics.value?.ruby_annotations;
  if (!annotations) return;
  const ok = await copyTextToClipboard(formatPlainText(annotations));
  if (ok) showSuccess('歌詞テキストをコピーしました');
  else showError('コピーに失敗しました');
  settingsOpen.value = false;
}

function openExportPage(): void {
  settingsOpen.value = false;
  openExportForCurrentLyrics();
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && settingsOpen.value) {
    event.preventDefault();
    settingsOpen.value = false;
  }
}

function onResize(): void {
  viewportWidth.value = window.innerWidth;
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown);
  window.addEventListener('resize', onResize);
});

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown);
  window.removeEventListener('resize', onResize);
});
</script>

<template>
  <div class="lyrics-view">
    <header class="view-header">
      <button
        type="button"
        class="icon-button"
        aria-label="戻る"
        title="戻る"
        @click="navigateBack"
      >
        <AppIcon name="chevron-left" :size="22" />
      </button>

      <div class="lyrics-heading">
        <span
          v-if="coverUrl"
          class="lyrics-heading__cover"
          :style="{ backgroundImage: `url(${coverUrl})` }"
          aria-hidden="true"
        />
        <div class="view-header__title">
          <h2>{{ currentLyrics?.found_title || '歌詞' }}</h2>
          <p>{{ currentLyrics?.found_artist || '' }}</p>
        </div>
      </div>

      <div class="view-header__actions">
        <button
          type="button"
          class="icon-button"
          :disabled="!currentLyrics || currentLyrics.status !== 'success'"
          aria-label="エクスポート"
          title="エクスポート"
          @click="openExportPage"
        >
          <AppIcon name="download" :size="20" />
        </button>
        <button
          type="button"
          class="icon-button"
          :class="{ 'is-active': settingsOpen }"
          :aria-expanded="settingsOpen"
          aria-label="表示設定"
          title="表示設定"
          @click="settingsOpen = !settingsOpen"
        >
          <AppIcon name="font-size" :size="20" />
        </button>
      </div>
    </header>

    <div ref="scrollRef" class="view-scroll">
      <div v-if="isLoadingLyrics" class="lyricBody lyric-skeleton">
        <div
          v-for="n in 9"
          :key="n"
          class="skeleton-line"
          :style="{ width: `${45 + ((n * 17) % 42)}%`, height: '18px' }"
        />
      </div>

      <LyricRenderer
        v-else-if="currentLyrics && currentLyrics.status === 'success'"
        :elements="currentLyrics.ruby_annotations"
        :font-size="fontSize"
        :show-ruby="showRuby"
        :scroll-container="scrollRef"
      />

      <div v-else class="empty-card">
        <span class="empty-card__icon">
          <AppIcon name="alert-circle" :size="20" />
        </span>
        <p class="empty-card__title">歌詞を表示できませんでした</p>
        <p class="empty-card__desc">前の画面に戻って別の候補をお試しください。</p>
      </div>
    </div>

    <Teleport to="body">
      <Transition name="sheet-fade">
        <div
          v-if="settingsOpen"
          class="reader-sheet-backdrop"
          @click.self="settingsOpen = false"
        >
          <div
            class="reader-sheet surface-group"
            role="dialog"
            aria-label="歌詞表示設定"
          >
            <div class="reader-sheet__header">
              <span>表示設定</span>
              <button
                type="button"
                class="icon-button icon-button--sm"
                aria-label="閉じる"
                @click="settingsOpen = false"
              >
                <AppIcon name="x" :size="16" />
              </button>
            </div>

            <div class="reader-sheet__row">
              <div class="reader-sheet__label">文字サイズ</div>
              <div class="font-stepper">
                <button
                  type="button"
                  class="icon-button icon-button--sm"
                  :disabled="fontSize <= FONT_SIZE_MIN"
                  aria-label="文字を小さく"
                  @click="stepFontSize(-1)"
                >
                  <AppIcon name="minus" :size="16" />
                </button>
                <input
                  class="font-stepper__range"
                  type="range"
                  :min="FONT_SIZE_MIN"
                  :max="FONT_SIZE_MAX"
                  step="1"
                  :value="fontSize"
                  :style="{ '--range-progress': rangeProgress }"
                  aria-label="文字サイズ"
                  @input="setFontSize(Number(($event.target as HTMLInputElement).value))"
                />
                <span class="font-stepper__value">{{ fontSize }}px</span>
                <button
                  type="button"
                  class="icon-button icon-button--sm"
                  :disabled="fontSize >= FONT_SIZE_MAX"
                  aria-label="文字を大きく"
                  @click="stepFontSize(1)"
                >
                  <AppIcon name="plus" :size="16" />
                </button>
              </div>
            </div>

            <div class="reader-sheet__row reader-sheet__row--inline">
              <span class="reader-sheet__label">テーマ</span>
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

            <div class="reader-sheet__row reader-sheet__row--inline">
              <span class="reader-sheet__label">ふりがな（ルビ）</span>
              <button
                type="button"
                class="ghost-pill"
                :class="{ 'is-active': showRuby }"
                :aria-pressed="showRuby"
                @click="toggleRuby"
              >
                <AppIcon :name="showRuby ? 'eye' : 'eye-off'" :size="15" />
                <span>{{ showRuby ? '表示中' : '非表示' }}</span>
              </button>
            </div>

            <div class="reader-sheet__actions">
              <button
                type="button"
                class="secondary"
                :disabled="!currentLyrics || currentLyrics.status !== 'success'"
                @click="void copyPlainLyrics()"
              >
                <AppIcon name="copy" :size="16" />
                <span>テキストをコピー</span>
              </button>
              <button
                type="button"
                class="primary"
                :disabled="!currentLyrics || currentLyrics.status !== 'success'"
                @click="openExportPage"
              >
                <AppIcon name="download" :size="16" />
                <span>エクスポートへ</span>
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped lang="scss">
.lyrics-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.lyrics-heading {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;

  &__cover {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background-size: cover;
    background-position: center;
    border: 1px solid var(--border);
    flex-shrink: 0;
  }
}

.lyric-skeleton {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 320px;
}

.reader-sheet-backdrop {
  position: fixed;
  inset: 0;
  z-index: var(--z-sheet-backdrop);
  background: rgba(0, 0, 0, 0.36);
  display: flex;
  align-items: flex-end;
  justify-content: center;
  padding: 16px;
}

.reader-sheet {
  width: min(100%, 480px);
  padding: 16px;
  background: var(--surface-elevated);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
  gap: 14px;
  margin-bottom: env(safe-area-inset-bottom);

  &__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: var(--font-md);
    font-weight: 700;
  }

  &__row {
    display: flex;
    flex-direction: column;
    gap: 8px;

    &--inline {
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
    }
  }

  &__label {
    font-size: var(--font-sm);
    font-weight: 600;
    color: var(--text-secondary);
  }

  &__actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    padding-top: 6px;
    border-top: 1px solid var(--border-subtle);
  }
}

.ghost-pill.is-active {
  background: var(--primary-dim);
  color: var(--primary);
}

.font-stepper {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 6px;
  border-radius: var(--radius-md);
  background: var(--bg-secondary);

  &__range {
    flex: 1 1 auto;
    min-width: 80px;
    height: 36px;
    appearance: none;
    -webkit-appearance: none;
    background: transparent;
    cursor: pointer;

    &::-webkit-slider-runnable-track {
      height: 4px;
      border-radius: 999px;
      background: linear-gradient(
        to right,
        var(--primary) 0%,
        var(--primary) var(--range-progress, 20%),
        var(--surface-hover) var(--range-progress, 20%),
        var(--surface-hover) 100%
      );
    }

    &::-webkit-slider-thumb {
      -webkit-appearance: none;
      width: 18px;
      height: 18px;
      margin-top: -7px;
      border-radius: 50%;
      background: var(--surface-elevated);
      border: 3px solid var(--primary);
      box-shadow: var(--shadow-sm);
    }
  }

  &__value {
    min-width: 44px;
    text-align: center;
    font-size: var(--font-xs);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }
}

@media (min-width: 768px) {
  .reader-sheet-backdrop {
    align-items: center;
  }
}

.sheet-fade-enter-active,
.sheet-fade-leave-active {
  transition: opacity 0.2s ease;

  .reader-sheet {
    transition: transform 0.26s var(--ease-spring), opacity 0.2s ease;
  }
}

.sheet-fade-enter-from,
.sheet-fade-leave-to {
  opacity: 0;

  .reader-sheet {
    transform: translateY(18px);
    opacity: 0;
  }
}
</style>
