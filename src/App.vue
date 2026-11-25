<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, type Component } from 'vue';
import type { ViewType } from './types';
import { initSettingsTheme, useSettings } from './composables/useSettings';
import { FIRST_LEVEL_VIEWS, initRouterHistory, useRouter } from './composables/useRouter';
import { useLyricsApp } from './composables/useLyricsApp';
import BottomDock from './components/BottomDock.vue';
import AppToast from './components/AppToast.vue';
import ConfirmDialog from './components/ConfirmDialog.vue';
import SearchView from './views/SearchView.vue';
import ResultsView from './views/ResultsView.vue';
import SongsView from './views/SongsView.vue';
import LyricsView from './views/LyricsView.vue';
import ExportView from './views/ExportView.vue';
import SettingsView from './views/SettingsView.vue';
import SettingsAboutView from './views/SettingsAboutView.vue';
import LspSettingsView from './views/LspSettingsView.vue';
import LspLogView from './views/LspLogView.vue';

const VIEW_COMPONENTS: Record<ViewType, Component> = {
  search: SearchView,
  results: ResultsView,
  songs: SongsView,
  lyrics: LyricsView,
  export: ExportView,
  settings: SettingsView,
  settingsAbout: SettingsAboutView,
  lspSettings: LspSettingsView,
  lspLogs: LspLogView,
};

const { settings } = useSettings();
const {
  routerState,
  isFirstLevel,
  canSwipeBack,
  getPreviousView,
  navigateBack,
  saveViewScroll,
  getSavedViewScroll,
} = useRouter();
const { setBackendLspLogging, syncLspSettings, checkSaltLaunchRequest } = useLyricsApp();

const activePaneRef = ref<HTMLElement | null>(null);
const stackRef = ref<HTMLElement | null>(null);

const activeComponent = computed(() => VIEW_COMPONENTS[routerState.current]);
const underlayView = computed(() =>
  routerState.swipeActive ? getPreviousView() : null,
);
const underlayComponent = computed(() =>
  underlayView.value ? VIEW_COMPONENTS[underlayView.value] : null,
);

const transitionName = computed(() => {
  if (routerState.swipeActive) return '';
  if (routerState.transitionMode === 'push') return 'nav-push';
  if (routerState.transitionMode === 'pop') return 'nav-pop';
  if (routerState.transitionMode === 'tab') return 'nav-tab';
  return '';
});

const swipeRatio = computed(() => {
  const width = stackRef.value?.clientWidth || window.innerWidth || 375;
  return Math.min(1, Math.max(0, routerState.swipeOffsetPx / width));
});

const activeSwipeStyle = computed(() => {
  if (!routerState.swipeActive) return undefined;
  return {
    transform: `translate3d(${routerState.swipeOffsetPx}px, 0, 0)`,
    transition: 'none',
  };
});

const underlayStyle = computed(() => {
  if (!routerState.swipeActive) return undefined;
  const shift = -14 * (1 - swipeRatio.value);
  return {
    transform: `translate3d(${shift}%, 0, 0)`,
    transition: 'none',
  };
});

const scrimStyle = computed(() => ({
  opacity: String(0.28 * (1 - swipeRatio.value)),
}));

// Save/restore inner scroll container per view
watch(
  () => routerState.current,
  (nextView, prevView) => {
    if (prevView) {
      const scroller = activePaneRef.value?.querySelector<HTMLElement>('.view-scroll');
      if (scroller) saveViewScroll(prevView, scroller.scrollTop);
    }
    void nextTick(() => {
      const scroller = activePaneRef.value?.querySelector<HTMLElement>('.view-scroll');
      if (scroller) scroller.scrollTop = getSavedViewScroll(nextView);
    });
  },
);

// Interactive left-edge swipe back
let gesture: { startX: number; startY: number; tracking: boolean } | null = null;

function onTouchStart(event: TouchEvent): void {
  if (!canSwipeBack() || event.touches.length !== 1) {
    gesture = null;
    return;
  }
  const touch = event.touches[0];
  if (touch.clientX > 28) {
    gesture = null;
    return;
  }
  gesture = { startX: touch.clientX, startY: touch.clientY, tracking: false };
}

function onTouchMove(event: TouchEvent): void {
  if (!gesture || event.touches.length !== 1) return;
  const touch = event.touches[0];
  const dx = touch.clientX - gesture.startX;
  const dy = touch.clientY - gesture.startY;

  if (!gesture.tracking) {
    if (Math.abs(dy) > 18 && Math.abs(dy) > Math.abs(dx)) {
      gesture = null;
      return;
    }
    if (dx > 10) {
      gesture.tracking = true;
      routerState.swipeActive = true;
      routerState.swipeUnderlay = getPreviousView();
    }
  }

  if (gesture.tracking) {
    routerState.swipeOffsetPx = Math.max(0, dx);
  }
}

function onTouchEnd(): void {
  if (!gesture) return;
  const shouldPop = gesture.tracking && routerState.swipeOffsetPx >= 86;
  gesture = null;

  if (shouldPop) {
    routerState.swipeActive = false;
    routerState.swipeOffsetPx = 0;
    routerState.swipeUnderlay = null;
    navigateBack();
  } else {
    routerState.swipeActive = false;
    routerState.swipeOffsetPx = 0;
    routerState.swipeUnderlay = null;
  }
}

// Viewport zoom guards for native app shell feel
function preventPinch(event: TouchEvent): void {
  if (event.touches.length > 1) event.preventDefault();
}
function preventGesture(event: Event): void {
  event.preventDefault();
}
function preventWheelZoom(event: WheelEvent): void {
  if (event.ctrlKey || event.metaKey) event.preventDefault();
}
function preventKeyZoom(event: KeyboardEvent): void {
  if ((event.ctrlKey || event.metaKey) && ['+', '-', '=', '0'].includes(event.key)) {
    event.preventDefault();
  }
}
function onVisibilityChange(): void {
  if (document.visibilityState === 'visible') {
    void checkSaltLaunchRequest();
  }
}

onMounted(() => {
  initSettingsTheme();
  initRouterHistory();

  void setBackendLspLogging(settings.lspLogEnabled === true);
  void syncLspSettings();
  void checkSaltLaunchRequest();

  window.addEventListener('touchstart', onTouchStart, { passive: true });
  window.addEventListener('touchmove', onTouchMove, { passive: true });
  window.addEventListener('touchend', onTouchEnd, { passive: true });
  window.addEventListener('touchcancel', onTouchEnd, { passive: true });

  document.addEventListener('touchmove', preventPinch, { passive: false });
  document.addEventListener('gesturestart', preventGesture);
  window.addEventListener('wheel', preventWheelZoom, { passive: false });
  window.addEventListener('keydown', preventKeyZoom);
  document.addEventListener('visibilitychange', onVisibilityChange);
});

onBeforeUnmount(() => {
  window.removeEventListener('touchstart', onTouchStart);
  window.removeEventListener('touchmove', onTouchMove);
  window.removeEventListener('touchend', onTouchEnd);
  window.removeEventListener('touchcancel', onTouchEnd);

  document.removeEventListener('touchmove', preventPinch);
  document.removeEventListener('gesturestart', preventGesture);
  window.removeEventListener('wheel', preventWheelZoom);
  window.removeEventListener('keydown', preventKeyZoom);
  document.removeEventListener('visibilitychange', onVisibilityChange);
});
</script>

<template>
  <div class="app-shell">
    <main class="stage-viewport">
      <div ref="stackRef" class="view-stack">
        <div
          v-if="underlayComponent && underlayView"
          class="view-pane view-pane--underlay"
          :class="{ 'view-pane--with-dock': FIRST_LEVEL_VIEWS.has(underlayView) }"
          :style="underlayStyle"
          aria-hidden="true"
        >
          <component :is="underlayComponent" />
        </div>

        <div
          v-if="routerState.swipeActive"
          class="view-scrim"
          :style="scrimStyle"
          aria-hidden="true"
        />

        <Transition :name="transitionName">
          <section
            :key="routerState.current"
            ref="activePaneRef"
            class="view-pane view-pane--active"
            :class="{
              'view-pane--with-dock': isFirstLevel,
              'view-pane--swiping': routerState.swipeActive,
            }"
            :style="activeSwipeStyle"
          >
            <component :is="activeComponent" />
          </section>
        </Transition>
      </div>
    </main>

    <BottomDock />
    <AppToast />
    <ConfirmDialog />
  </div>
</template>
