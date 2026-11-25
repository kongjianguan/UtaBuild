<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { LyricSource, SearchItem } from '../types';
import { resolveSourceMeta, formatStructuredJson, copyTextToClipboard } from '../services/formatters';
import { useLyricsApp } from '../composables/useLyricsApp';
import { useRouter } from '../composables/useRouter';
import { useFeedback } from '../composables/useFeedback';
import AppIcon from '../components/AppIcon.vue';
import ContextMenu, { type ContextMenuItem } from '../components/ContextMenu.vue';

const {
  searchData,
  activeSourceTab,
  isSearching,
  isLoadingMoreResults,
  utatenPageState,
  filteredSearchResults,
  fetchLyricsForItem,
  selectSearchResult,
  openExportFromResult,
  loadNextUtatenPage,
} = useLyricsApp();

const { navigateBack } = useRouter();
const { showSuccess, showError } = useFeedback();

const scrollRef = ref<HTMLElement | null>(null);
const sentinelRef = ref<HTMLElement | null>(null);

const SOURCE_TABS: Array<{ id: 'all' | LyricSource; label: string }> = [
  { id: 'all', label: 'すべて' },
  { id: 'utaten', label: 'UtaTen' },
  { id: 'qq_music', label: 'QQ音楽' },
  { id: 'netease', label: 'NetEase' },
];

const queryLabel = computed(() => {
  const q = searchData.value?.query;
  if (!q) return '';
  return q.artist ? `${q.title} / ${q.artist}` : q.title;
});

function sourceCount(id: 'all' | LyricSource): number {
  const all = searchData.value?.allResults || [];
  if (id === 'all') return all.length;
  return all.filter((item) => item.source === id).length;
}

function isSourceLoading(id: 'all' | LyricSource): boolean {
  const sources = searchData.value?.sources;
  if (!sources) return false;
  if (id === 'all') return Object.values(sources).some((s) => s.loading);
  return Boolean(sources[id]?.loading);
}

const activeSourceError = computed(() => {
  if (activeSourceTab.value === 'all') return null;
  return searchData.value?.sources?.[activeSourceTab.value]?.error || null;
});

const showInitialSkeleton = computed(
  () => filteredSearchResults.value.length === 0 && isSourceLoading(activeSourceTab.value),
);

// Context menu state
const menuOpen = ref(false);
const menuPos = ref({ x: 0, y: 0 });
const menuTarget = ref<SearchItem | null>(null);
const menuItems: ContextMenuItem[] = [
  { id: 'export', label: 'エクスポート・プレビュー', icon: 'download', tone: 'primary' },
  { id: 'copy-json', label: 'JSON をコピー', icon: 'copy' },
];

function openMenuFor(item: SearchItem, event: MouseEvent): void {
  menuTarget.value = item;
  menuPos.value = { x: event.clientX, y: event.clientY };
  menuOpen.value = true;
}

function openMenuFromButton(item: SearchItem, event: MouseEvent): void {
  const btn = event.currentTarget as HTMLElement;
  const rect = btn.getBoundingClientRect();
  menuTarget.value = item;
  menuPos.value = { x: rect.right - 180, y: rect.bottom + 6 };
  menuOpen.value = true;
}

async function onMenuSelect(actionId: string): Promise<void> {
  const item = menuTarget.value;
  menuOpen.value = false;
  if (!item) return;

  if (actionId === 'export') {
    await openExportFromResult(item);
    return;
  }

  if (actionId === 'copy-json') {
    try {
      const result = await fetchLyricsForItem(item, { saveSaltBridge: false });
      if (!result || result.status !== 'success') {
        showError(result?.error || '歌詞の取得に失敗しました');
        return;
      }
      const json = formatStructuredJson({
        title: result.found_title,
        artist: result.found_artist,
        lyricsUrl: result.lyrics_url,
        rubyAnnotations: result.ruby_annotations,
        coverUrl: result.cover_url ?? null,
      });
      const copied = await copyTextToClipboard(json);
      if (copied) showSuccess('JSON をクリップボードにコピーしました');
      else showError('クリップボードへのコピーに失敗しました');
    } catch (err) {
      showError(`コピーに失敗しました: ${err}`);
    }
  }
}

let observer: IntersectionObserver | null = null;

function setupObserver(): void {
  observer?.disconnect();
  if (typeof IntersectionObserver !== 'function' || !sentinelRef.value || !scrollRef.value) return;
  observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        void loadNextUtatenPage();
      }
    },
    {
      root: scrollRef.value,
      rootMargin: '0px 0px 180px 0px',
    },
  );
  observer.observe(sentinelRef.value);
}

watch(
  () => [utatenPageState.hasNext, activeSourceTab.value, filteredSearchResults.value.length],
  () => setupObserver(),
);

onMounted(() => setupObserver());
onBeforeUnmount(() => observer?.disconnect());
</script>

<template>
  <div class="results-view">
    <header class="view-header">
      <button
        type="button"
        class="icon-button"
        aria-label="検索に戻る"
        title="戻る"
        @click="navigateBack"
      >
        <AppIcon name="chevron-left" :size="22" />
      </button>
      <div class="view-header__title">
        <h2>検索結果</h2>
        <p v-if="queryLabel">{{ queryLabel }}</p>
      </div>
      <div class="view-header__actions">
        <span v-if="isSearching" class="results-status-pill">
          <span class="inline-spinner" aria-hidden="true" />
          <span>検索中</span>
        </span>
      </div>
    </header>

    <div class="source-tabs-wrap">
      <div class="segmented-pill source-tabs" role="tablist" aria-label="検索ソース">
        <button
          v-for="tab in SOURCE_TABS"
          :key="tab.id"
          type="button"
          role="tab"
          :class="{ active: activeSourceTab === tab.id }"
          :aria-selected="activeSourceTab === tab.id"
          @click="activeSourceTab = tab.id"
        >
          <span>{{ tab.label }}</span>
          <span v-if="isSourceLoading(tab.id)" class="tab-badge tab-badge--loading">…</span>
          <span v-else class="tab-badge">{{ sourceCount(tab.id) }}</span>
        </button>
      </div>
    </div>

    <div ref="scrollRef" class="view-scroll">
      <div v-if="showInitialSkeleton" class="surface-group">
        <div v-for="n in 5" :key="n" class="result-skeleton">
          <div class="skeleton-line" style="width: 56px; height: 20px; border-radius: 999px" />
          <div class="skeleton-line" style="width: 62%; height: 16px" />
          <div class="skeleton-line" style="width: 38%; height: 12px" />
        </div>
      </div>

      <div v-else-if="filteredSearchResults.length === 0" class="empty-card">
        <span class="empty-card__icon">
          <AppIcon name="search" :size="20" />
        </span>
        <p class="empty-card__title">候補が見つかりませんでした</p>
        <p class="empty-card__desc">
          {{ activeSourceError || '曲名やアーティスト名を変えてもう一度検索してみてください。' }}
        </p>
      </div>

      <div v-else class="surface-group">
        <div
          v-for="(item, index) in filteredSearchResults"
          :key="`${item.source}-${item.url}-${index}`"
          class="result-row"
          :class="{ 'is-menu-open': menuOpen && menuTarget?.url === item.url }"
          @contextmenu.prevent="openMenuFor(item, $event)"
        >
          <button
            type="button"
            class="result-row__main"
            @click="void selectSearchResult(item)"
          >
            <span
              class="source-badge"
              :class="`source-badge--${resolveSourceMeta(item.source).key}`"
            >
              {{ resolveSourceMeta(item.source).label }}
            </span>
            <span class="result-row__title">{{ item.title }}</span>
            <span class="result-row__artist">{{ item.artist }}</span>
          </button>
          <button
            type="button"
            class="icon-button icon-button--sm result-row__more"
            aria-label="候補のメニューを開く"
            @click.stop="openMenuFromButton(item, $event)"
          >
            <AppIcon name="more-vertical" :size="18" />
          </button>
        </div>
      </div>

      <div
        v-if="utatenPageState.totalPages > 1 && (activeSourceTab === 'all' || activeSourceTab === 'utaten')"
        ref="sentinelRef"
        class="pagination-bar"
      >
        <span v-if="isLoadingMoreResults || utatenPageState.loadingMore" class="pagination-bar__status">
          <span class="inline-spinner" aria-hidden="true" />
          <span>続きを読み込み中 ({{ utatenPageState.loadedPages }}/{{ utatenPageState.totalPages }})</span>
        </span>
        <span v-else-if="utatenPageState.hasNext">
          スクロールで続きを読み込み ({{ utatenPageState.loadedPages }}/{{ utatenPageState.totalPages }})
        </span>
        <span v-else>
          全 {{ utatenPageState.totalPages }} ページを読み込み済み
        </span>
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
.results-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.results-status-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border-radius: var(--radius-pill);
  background: var(--primary-dim);
  color: var(--primary);
  font-size: var(--font-xs);
  font-weight: 600;
}

.source-tabs-wrap {
  flex: 0 0 auto;
  margin-bottom: 12px;
  overflow-x: auto;
  scrollbar-width: none;

  &::-webkit-scrollbar {
    display: none;
  }
}

.source-tabs {
  display: inline-flex;
  min-width: max-content;
}

.tab-badge {
  font-size: 0.7rem;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;

  &--loading {
    opacity: 0.7;
  }
}

.result-skeleton {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 16px 18px;
  border-bottom: 1px solid var(--border-subtle);

  &:last-child {
    border-bottom: 0;
  }
}

.result-row {
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
    padding: 14px 12px 14px 18px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
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

  &__artist {
    width: 100%;
    color: var(--text-secondary);
    font-size: var(--font-xs);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  &__more {
    margin-right: 10px;
    flex-shrink: 0;
  }
}

.pagination-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 18px 12px 6px;
  color: var(--text-muted);
  font-size: var(--font-xs);

  &__status {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
  }
}
</style>
