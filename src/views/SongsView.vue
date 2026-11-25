<script setup lang="ts">
import { onMounted, ref } from 'vue';
import type { SavedSong } from '../types';
import { normalizeCoverUrl, resolveSourceMeta } from '../services/formatters';
import { useLyricsApp } from '../composables/useLyricsApp';
import AppIcon from '../components/AppIcon.vue';
import ContextMenu, { type ContextMenuItem } from '../components/ContextMenu.vue';

const {
  savedSongs,
  filteredSongs,
  songsSortBy,
  songsFilterQuery,
  isLoadingSongs,
  hydratingUrls,
  loadSavedLyrics,
  openSavedLyrics,
  openExportFromSavedSong,
  refreshSavedSongArtwork,
  deleteSavedSong,
} = useLyricsApp();

const menuOpen = ref(false);
const menuPos = ref({ x: 0, y: 0 });
const menuTarget = ref<SavedSong | null>(null);
const menuItems: ContextMenuItem[] = [
  { id: 'refresh-art', label: 'ジャケット画像を更新', icon: 'refresh', tone: 'primary' },
  { id: 'delete', label: '削除', icon: 'trash', tone: 'danger' },
];

function setSort(sortBy: 'title' | 'artist'): void {
  if (songsSortBy.value === sortBy) return;
  songsSortBy.value = sortBy;
  void loadSavedLyrics();
}

function formatMeta(song: SavedSong): string {
  const artist = song.artist || 'アーティスト不明';
  return song.album ? `${artist} ・ ${song.album}` : artist;
}

function openMenuFor(song: SavedSong, event: MouseEvent): void {
  menuTarget.value = song;
  menuPos.value = { x: event.clientX, y: event.clientY };
  menuOpen.value = true;
}

function openMenuFromButton(song: SavedSong, event: MouseEvent): void {
  const btn = event.currentTarget as HTMLElement;
  const rect = btn.getBoundingClientRect();
  menuTarget.value = song;
  menuPos.value = { x: rect.right - 188, y: rect.bottom + 6 };
  menuOpen.value = true;
}

async function onMenuSelect(actionId: string): Promise<void> {
  const song = menuTarget.value;
  menuOpen.value = false;
  if (!song) return;

  if (actionId === 'refresh-art') {
    await refreshSavedSongArtwork(song);
  } else if (actionId === 'delete') {
    await deleteSavedSong(song);
  }
}

onMounted(() => {
  if (savedSongs.value.length === 0) {
    void loadSavedLyrics();
  }
});
</script>

<template>
  <div class="songs-view">
    <header class="view-header view-header--top-level">
      <div class="view-header__title">
        <h1>保存済みの曲</h1>
        <p>{{ savedSongs.length > 0 ? `${savedSongs.length} 曲をオフライン保存中` : '開いた歌詞は自動でここに保存されます' }}</p>
      </div>
      <div class="view-header__actions">
        <div class="segmented-pill" role="group" aria-label="並び替え">
          <button
            type="button"
            :class="{ active: songsSortBy === 'title' }"
            :aria-pressed="songsSortBy === 'title'"
            @click="setSort('title')"
          >
            曲名
          </button>
          <button
            type="button"
            :class="{ active: songsSortBy === 'artist' }"
            :aria-pressed="songsSortBy === 'artist'"
            @click="setSort('artist')"
          >
            歌手
          </button>
        </div>
      </div>
    </header>

    <div v-if="savedSongs.length > 0" class="songs-filter surface-group">
      <AppIcon name="search" :size="16" class="songs-filter__icon" />
      <label class="visually-hidden" for="songs-filter-input">保存済みの曲を絞り込む</label>
      <input
        id="songs-filter-input"
        v-model="songsFilterQuery"
        type="search"
        placeholder="曲名・アーティスト・アルバムで絞り込み..."
      />
      <button
        v-if="songsFilterQuery"
        type="button"
        class="icon-button icon-button--sm"
        aria-label="絞り込みをクリア"
        @click="songsFilterQuery = ''"
      >
        <AppIcon name="x" :size="15" />
      </button>
    </div>

    <div class="view-scroll">
      <div v-if="isLoadingSongs && savedSongs.length === 0" class="surface-group">
        <div v-for="n in 4" :key="n" class="song-skeleton">
          <div class="skeleton-line" style="width: 54px; height: 54px; border-radius: 12px" />
          <div class="song-skeleton__lines">
            <div class="skeleton-line" style="width: 55%; height: 15px" />
            <div class="skeleton-line" style="width: 38%; height: 12px" />
          </div>
        </div>
      </div>

      <div v-else-if="savedSongs.length === 0" class="empty-card">
        <span class="empty-card__icon">
          <AppIcon name="music" :size="20" />
        </span>
        <p class="empty-card__title">保存済みの歌詞はまだありません</p>
        <p class="empty-card__desc">検索して歌詞を開くと、自動的にオフライン保存されます。</p>
      </div>

      <div v-else-if="filteredSongs.length === 0" class="empty-card">
        <span class="empty-card__icon">
          <AppIcon name="search" :size="20" />
        </span>
        <p class="empty-card__title">一致する曲がありません</p>
        <p class="empty-card__desc">「{{ songsFilterQuery }}」に一致する保存済み曲はありませんでした。</p>
      </div>

      <div v-else class="surface-group">
        <div
          v-for="song in filteredSongs"
          :key="song.lyrics_url"
          class="song-row"
          :class="{ 'is-menu-open': menuOpen && menuTarget?.lyrics_url === song.lyrics_url }"
          @contextmenu.prevent="openMenuFor(song, $event)"
        >
          <button
            type="button"
            class="song-row__main"
            @click="void openSavedLyrics(song)"
          >
            <span
              class="song-row__cover"
              :class="{ 'has-cover': Boolean(normalizeCoverUrl(song.cover_url)) }"
              :style="normalizeCoverUrl(song.cover_url) ? { backgroundImage: `url(${normalizeCoverUrl(song.cover_url)})` } : undefined"
              aria-hidden="true"
            >
              <AppIcon v-if="!normalizeCoverUrl(song.cover_url)" name="music" :size="20" />
              <span
                v-if="hydratingUrls.has(song.lyrics_url)"
                class="song-row__cover-sync"
                aria-hidden="true"
              >
                <span class="inline-spinner" />
              </span>
            </span>

            <span class="song-row__body">
              <span
                class="source-badge"
                :class="`source-badge--${resolveSourceMeta(song.lyrics_url).key}`"
              >
                {{ resolveSourceMeta(song.lyrics_url).label }}
              </span>
              <span class="song-row__title">{{ song.title || 'タイトル未設定' }}</span>
              <span class="song-row__meta">{{ formatMeta(song) }}</span>
            </span>
          </button>

          <div class="song-row__actions">
            <button
              type="button"
              class="icon-button icon-button--sm"
              aria-label="エクスポート"
              title="エクスポート"
              @click.stop="void openExportFromSavedSong(song)"
            >
              <AppIcon name="download" :size="18" />
            </button>
            <button
              type="button"
              class="icon-button icon-button--sm"
              aria-label="曲のメニューを開く"
              title="その他"
              @click.stop="openMenuFromButton(song, $event)"
            >
              <AppIcon name="more-vertical" :size="18" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <ContextMenu
      :open="menuOpen"
      :x="menuPos.x"
      :y="menuPos.y"
      :items="menuItems"
      @close="menuOpen = false"
      @select="onMenuSelect"
    />
  </div>
</template>

<style scoped lang="scss">
.songs-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.songs-filter {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px 4px 14px;
  margin-bottom: 12px;

  &__icon {
    color: var(--text-muted);
  }

  input {
    flex: 1 1 auto;
    min-width: 0;
    height: 38px;
    border: 0;
    background: transparent;
    color: var(--text);
    font-size: var(--font-sm);

    &:focus {
      outline: none;
    }
  }
}

.song-skeleton {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border-subtle);

  &:last-child {
    border-bottom: 0;
  }

  &__lines {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
}

.song-row {
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

  &.is-menu-open {
    background: var(--primary-dim);
  }

  &__main {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 13px 8px 13px 16px;
    text-align: left;
    border-radius: 0;

    &:hover {
      background: transparent;
    }
  }

  &__cover {
    position: relative;
    width: 54px;
    height: 54px;
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    background-size: cover;
    background-position: center;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: 1px solid var(--border-subtle);

    &.has-cover {
      box-shadow: var(--shadow-sm);
      border-color: var(--border);
    }
  }

  &__cover-sync {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: rgba(0, 0, 0, 0.45);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  &__body {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
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
    color: var(--text-secondary);
    font-size: var(--font-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__actions {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 10px;
    flex-shrink: 0;
  }
}
</style>
