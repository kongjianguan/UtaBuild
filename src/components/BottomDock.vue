<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { PrimaryTab } from '../types';
import { useRouter } from '../composables/useRouter';
import { useLyricsApp } from '../composables/useLyricsApp';
import AppIcon, { type IconName } from './AppIcon.vue';

const { routerState, isFirstLevel, navigateTo } = useRouter();
const { loadSavedLyrics } = useLyricsApp();

const TABS: Array<{ id: PrimaryTab; label: string; icon: IconName }> = [
  { id: 'search', label: '検索', icon: 'search' },
  { id: 'songs', label: '保存曲', icon: 'music' },
  { id: 'settings', label: '設定', icon: 'sliders' },
];

const trackRef = ref<HTMLElement | null>(null);
const itemRefs = new Map<PrimaryTab, HTMLElement>();
const sliderStyle = ref<Record<string, string>>({ opacity: '0' });

function setItemRef(id: PrimaryTab, el: unknown): void {
  if (el instanceof HTMLElement) {
    itemRefs.set(id, el);
  }
}

function syncSlider(): void {
  const activeBtn = itemRefs.get(routerState.activeTab);
  if (!activeBtn) return;
  sliderStyle.value = {
    width: `${activeBtn.offsetWidth}px`,
    height: `${activeBtn.offsetHeight}px`,
    transform: `translate3d(${activeBtn.offsetLeft - 6}px, ${activeBtn.offsetTop - 6}px, 0)`,
    opacity: '1',
  };
}

function selectTab(tab: PrimaryTab): void {
  if (tab === routerState.current) return;
  navigateTo(tab, { replace: true });
  if (tab === 'songs') {
    void loadSavedLyrics();
  }
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
  event.preventDefault();
  const idx = TABS.findIndex((t) => t.id === routerState.activeTab);
  const delta = event.key === 'ArrowRight' ? 1 : -1;
  const next = TABS[(idx + delta + TABS.length) % TABS.length];
  if (!next) return;
  selectTab(next.id);
  void nextTick(() => itemRefs.get(next.id)?.focus());
}

watch(
  () => routerState.activeTab,
  () => {
    void nextTick(syncSlider);
  },
);

let resizeObserver: ResizeObserver | null = null;

onMounted(() => {
  void nextTick(syncSlider);
  if (typeof ResizeObserver === 'function' && trackRef.value) {
    resizeObserver = new ResizeObserver(() => syncSlider());
    resizeObserver.observe(trackRef.value);
  }
  window.addEventListener('resize', syncSlider);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  window.removeEventListener('resize', syncSlider);
});
</script>

<template>
  <nav
    class="bottom-menu"
    :class="{ 'is-hidden-mobile': !isFirstLevel }"
    aria-label="メインメニュー"
    @keydown="handleKeydown"
  >
    <div ref="trackRef" class="bottom-menu__track" role="tablist" aria-label="メインメニュー">
      <div class="bottom-menu__slider" :style="sliderStyle" aria-hidden="true" />
      <button
        v-for="tab in TABS"
        :key="tab.id"
        :ref="(el) => setItemRef(tab.id, el)"
        type="button"
        role="tab"
        class="bottom-menu__item"
        :class="{ active: routerState.activeTab === tab.id }"
        :aria-selected="routerState.activeTab === tab.id"
        :aria-current="routerState.activeTab === tab.id ? 'page' : undefined"
        :tabindex="routerState.activeTab === tab.id ? 0 : -1"
        @click="selectTab(tab.id)"
      >
        <AppIcon :name="tab.icon" :size="20" />
        <span>{{ tab.label }}</span>
      </button>
    </div>
  </nav>
</template>
