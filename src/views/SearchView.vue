<script setup lang="ts">
import { ref } from 'vue';
import type { HistoryEntry } from '../types';
import { useLyricsApp } from '../composables/useLyricsApp';
import AppIcon from '../components/AppIcon.vue';

const {
  searchTitle,
  searchArtist,
  searchHistory,
  isSearching,
  pendingSaltRequest,
  submitSearch,
  removeHistoryEntry,
  clearAllSearchHistory,
} = useLyricsApp();

const titleInputRef = ref<HTMLInputElement | null>(null);
const artistInputRef = ref<HTMLInputElement | null>(null);
const showArtistField = ref(Boolean(searchArtist.value));

function onSubmit(): void {
  void submitSearch();
}

function handleHistoryClick(entry: HistoryEntry): void {
  if (entry.artist) showArtistField.value = true;
  void submitSearch(entry.title, entry.artist);
}

function fillHistoryIntoInput(entry: HistoryEntry): void {
  searchTitle.value = entry.title;
  searchArtist.value = entry.artist || '';
  if (entry.artist) showArtistField.value = true;
  titleInputRef.value?.focus();
}

function toggleArtistField(): void {
  showArtistField.value = !showArtistField.value;
  if (!showArtistField.value) {
    searchArtist.value = '';
  } else {
    requestAnimationFrame(() => artistInputRef.value?.focus());
  }
}

function formatHistoryTime(timestamp: number): string {
  if (!Number.isFinite(timestamp)) return '';
  try {
    return new Date(timestamp).toLocaleString('ja-JP', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return '';
  }
}
</script>

<template>
  <div class="search-view">
    <header class="view-header view-header--top-level">
      <div class="view-header__title">
        <h1>歌詞検索</h1>
        <p>UtaTen・QQ音楽・NetEase からふりがな付き歌詞を検索</p>
      </div>
    </header>

    <div v-if="pendingSaltRequest" class="salt-banner" role="status">
      <AppIcon name="music" :size="16" />
      <span>
        Salt Player から「{{ pendingSaltRequest.title }}」の紐付け待機中
      </span>
      <button
        type="button"
        class="icon-button icon-button--sm"
        aria-label="紐付け待機をキャンセル"
        @click="pendingSaltRequest = null"
      >
        <AppIcon name="x" :size="15" />
      </button>
    </div>

    <form class="search-card surface-group" role="search" @submit.prevent="onSubmit">
      <div class="search-card__row">
        <span class="search-card__lead" aria-hidden="true">
          <AppIcon name="search" :size="18" />
        </span>
        <label class="visually-hidden" for="search-title">曲名</label>
        <input
          id="search-title"
          ref="titleInputRef"
          v-model="searchTitle"
          type="text"
          placeholder="曲名を入力..."
          autocomplete="off"
          enterkeyhint="search"
        />
        <button
          v-if="searchTitle"
          type="button"
          class="icon-button icon-button--sm search-card__clear"
          aria-label="曲名をクリア"
          @click="searchTitle = ''; titleInputRef?.focus()"
        >
          <AppIcon name="x" :size="15" />
        </button>
        <button
          type="button"
          class="ghost-pill search-card__artist-toggle"
          :class="{ 'is-active': showArtistField || Boolean(searchArtist) }"
          :aria-expanded="showArtistField || Boolean(searchArtist)"
          @click="toggleArtistField"
        >
          歌手名
        </button>
        <button
          type="submit"
          class="primary search-card__submit"
          :disabled="isSearching"
          aria-label="検索"
        >
          <span v-if="isSearching" class="inline-spinner" aria-hidden="true" />
          <AppIcon v-else name="search" :size="18" />
        </button>
      </div>

      <div v-if="showArtistField || searchArtist" class="search-card__subrow">
        <label class="visually-hidden" for="search-artist">アーティスト名（任意）</label>
        <input
          id="search-artist"
          ref="artistInputRef"
          v-model="searchArtist"
          type="text"
          placeholder="アーティスト名で絞り込み（任意）"
          autocomplete="off"
          enterkeyhint="search"
        />
        <button
          v-if="searchArtist"
          type="button"
          class="icon-button icon-button--sm search-card__clear"
          aria-label="アーティスト名をクリア"
          @click="searchArtist = ''; artistInputRef?.focus()"
        >
          <AppIcon name="x" :size="15" />
        </button>
      </div>
    </form>

    <section class="history-section" aria-labelledby="search-history-title">
      <div class="section-label">
        <span id="search-history-title">検索履歴</span>
        <button
          v-if="searchHistory.length > 0"
          type="button"
          class="history-clear-btn"
          @click="clearAllSearchHistory"
        >
          すべて削除
        </button>
      </div>

      <div class="view-scroll">
        <div v-if="searchHistory.length === 0" class="empty-card">
          <span class="empty-card__icon">
            <AppIcon name="search" :size="20" />
          </span>
          <p class="empty-card__title">まだ検索履歴はありません</p>
          <p class="empty-card__desc">曲名を検索すると、ここからワンタップで再検索できます。</p>
        </div>

        <div v-else class="surface-group history-list">
          <div
            v-for="entry in searchHistory"
            :key="`${entry.title}-${entry.artist ?? ''}-${entry.searchedAt}`"
            class="history-row"
          >
            <button
              type="button"
              class="history-row__main"
              @click="handleHistoryClick(entry)"
            >
              <span class="history-row__title">{{ entry.title }}</span>
              <span class="history-row__meta">
                <span>{{ entry.artist || 'アーティスト指定なし' }}</span>
                <span class="history-row__time">{{ formatHistoryTime(entry.searchedAt) }}</span>
              </span>
            </button>
            <div class="history-row__actions">
              <button
                type="button"
                class="icon-button icon-button--sm"
                aria-label="入力欄にコピーして編集"
                title="入力欄にコピーして編集"
                @click="fillHistoryIntoInput(entry)"
              >
                <AppIcon name="arrow-up-left" :size="15" />
              </button>
              <button
                type="button"
                class="icon-button icon-button--sm"
                aria-label="この履歴を削除"
                title="削除"
                @click="removeHistoryEntry(entry)"
              >
                <AppIcon name="x" :size="15" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped lang="scss">
.search-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.salt-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  margin-bottom: 12px;
  border-radius: var(--radius-md);
  background: var(--info-bg);
  color: var(--info);
  font-size: var(--font-xs);
  font-weight: 600;

  span {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
}

.search-card {
  flex: 0 0 auto;
  padding: 6px;
  margin-bottom: 20px;

  &__row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  &__lead {
    padding-left: 10px;
    color: var(--text-muted);
    display: inline-flex;
    flex-shrink: 0;
  }

  input {
    flex: 1 1 auto;
    min-width: 0;
    height: 44px;
    padding: 8px 6px;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: var(--font-md);
    font-weight: 500;

    &:focus {
      outline: none;
    }
  }

  &__clear {
    color: var(--text-muted);
  }

  &__artist-toggle {
    flex-shrink: 0;

    &.is-active {
      color: var(--primary);
      background: var(--primary-dim);
    }
  }

  &__submit {
    width: 44px;
    min-width: 44px;
    height: 44px;
    padding: 0;
    border-radius: var(--radius-md);
    flex-shrink: 0;
  }

  &__subrow {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    padding: 4px 4px 0 36px;
    border-top: 1px solid var(--border-subtle);

    input {
      height: 40px;
      font-size: var(--font-sm);
    }
  }
}

.history-section {
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.history-clear-btn {
  min-height: 28px;
  padding: 2px 8px;
  border-radius: var(--radius-pill);
  font-size: var(--font-xs);
  color: var(--text-muted);

  &:hover {
    color: var(--danger);
    background: var(--danger-bg);
  }
}

.history-list {
  display: flex;
  flex-direction: column;
}

.history-row {
  display: flex;
  align-items: center;
  border-bottom: 1px solid var(--border-subtle);
  transition: background var(--motion-fast);

  &:last-child {
    border-bottom: 0;
  }

  &:hover {
    background: var(--surface-hover);
  }

  &__main {
    flex: 1 1 auto;
    min-width: 0;
    min-height: 62px;
    padding: 10px 12px 10px 16px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: 3px;
    text-align: left;
    border-radius: 0;

    &:hover {
      background: transparent;
    }
  }

  &__title {
    width: 100%;
    color: var(--text);
    font-size: var(--font-md);
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__meta {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    color: var(--text-secondary);
    font-size: var(--font-xs);

    span:first-child {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }

  &__time {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  &__actions {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 8px;
    flex-shrink: 0;
  }
}
</style>
