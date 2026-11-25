import { computed, reactive } from 'vue';
import type { PrimaryTab, ViewType } from '../types';

export const FIRST_LEVEL_VIEWS = new Set<ViewType>(['search', 'songs', 'settings']);

const VIEW_DEPTH: Record<ViewType, number> = {
  search: 0,
  songs: 0,
  settings: 0,
  results: 1,
  settingsAbout: 1,
  lspSettings: 1,
  lyrics: 2,
  lspLogs: 2,
  export: 3,
};

export interface RouterState {
  current: ViewType;
  stack: ViewType[];
  activeTab: PrimaryTab;
  transitionMode: 'push' | 'pop' | 'tab' | 'none';
  swipeActive: boolean;
  swipeOffsetPx: number;
  swipeUnderlay: ViewType | null;
}

const state = reactive<RouterState>({
  current: 'search',
  stack: ['search'],
  activeTab: 'search',
  transitionMode: 'none',
  swipeActive: false,
  swipeOffsetPx: 0,
  swipeUnderlay: null,
});

const scrollPositions = new Map<ViewType, number>();
let navigatingBackViaApi = false;

function tabOwnerForView(view: ViewType, fallback: PrimaryTab): PrimaryTab {
  if (view === 'search' || view === 'songs' || view === 'settings') return view;
  if (view === 'results') return 'search';
  if (view === 'settingsAbout' || view === 'lspSettings' || view === 'lspLogs') return 'settings';
  return fallback;
}

export function saveViewScroll(view: ViewType, scrollTop: number): void {
  scrollPositions.set(view, Math.max(0, Math.round(scrollTop)));
}

export function getSavedViewScroll(view: ViewType): number {
  return scrollPositions.get(view) ?? 0;
}

export function clearViewScroll(view: ViewType): void {
  scrollPositions.delete(view);
}

export function initRouterHistory(): void {
  if ('scrollRestoration' in history) {
    history.scrollRestoration = 'manual';
  }

  history.replaceState({ view: 'search', stack: ['search'], tab: 'search' }, '', '');

  window.addEventListener('popstate', (event: PopStateEvent) => {
    const historyState = event.state as {
      view?: ViewType;
      stack?: ViewType[];
      tab?: PrimaryTab;
    } | null;

    navigatingBackViaApi = false;

    if (!historyState?.view) return;

    const prevDepth = VIEW_DEPTH[state.current];
    const nextDepth = VIEW_DEPTH[historyState.view];

    if (FIRST_LEVEL_VIEWS.has(historyState.view) && FIRST_LEVEL_VIEWS.has(state.current)) {
      state.transitionMode = 'tab';
    } else if (nextDepth < prevDepth || state.stack.length > (historyState.stack?.length ?? 1)) {
      state.transitionMode = 'pop';
    } else {
      state.transitionMode = 'push';
    }

    state.current = historyState.view;
    state.stack = Array.isArray(historyState.stack) && historyState.stack.length > 0
      ? [...historyState.stack]
      : [historyState.view];
    state.activeTab = historyState.tab ?? tabOwnerForView(historyState.view, state.activeTab);
    state.swipeActive = false;
    state.swipeOffsetPx = 0;
    state.swipeUnderlay = null;
  });
}

export function navigateTo(
  view: ViewType,
  opts?: { replace?: boolean; resetScroll?: boolean },
): void {
  if (view === state.current) return;

  if (opts?.resetScroll) {
    scrollPositions.set(view, 0);
  }

  const isFirstLevelTarget = FIRST_LEVEL_VIEWS.has(view);
  let nextStack: ViewType[];

  if (isFirstLevelTarget) {
    nextStack = [view];
    state.transitionMode = FIRST_LEVEL_VIEWS.has(state.current) ? 'tab' : 'pop';
  } else {
    const existingIndex = state.stack.indexOf(view);
    if (existingIndex >= 0) {
      nextStack = state.stack.slice(0, existingIndex + 1);
      state.transitionMode = 'pop';
    } else {
      nextStack = [...state.stack, view];
      state.transitionMode = 'push';
    }
  }

  state.current = view;
  state.stack = nextStack;
  state.activeTab = tabOwnerForView(view, state.activeTab);

  const historyPayload = {
    view,
    stack: [...nextStack],
    tab: state.activeTab,
  };

  if (!navigatingBackViaApi) {
    if (opts?.replace) {
      history.replaceState(historyPayload, '', '');
    } else {
      history.pushState(historyPayload, '', '');
    }
  }
  navigatingBackViaApi = false;
}

export function navigateBack(): void {
  if (state.stack.length <= 1) return;

  navigatingBackViaApi = true;
  window.history.back();

  // Fallback in case history.back() does not trigger popstate synchronously/at all
  setTimeout(() => {
    if (navigatingBackViaApi) {
      navigatingBackViaApi = false;
      const nextStack = state.stack.slice(0, -1);
      const target = nextStack[nextStack.length - 1] ?? 'search';
      state.transitionMode = 'pop';
      state.stack = nextStack;
      state.current = target;
      state.activeTab = tabOwnerForView(target, state.activeTab);
    }
  }, 80);
}

export function canSwipeBack(): boolean {
  return !FIRST_LEVEL_VIEWS.has(state.current) && state.stack.length > 1;
}

export function getPreviousView(): ViewType | null {
  if (state.stack.length < 2) return null;
  return state.stack[state.stack.length - 2] ?? null;
}

export function useRouter() {
  const isFirstLevel = computed(() => FIRST_LEVEL_VIEWS.has(state.current));

  return {
    routerState: state,
    isFirstLevel,
    navigateTo,
    navigateBack,
    canSwipeBack,
    getPreviousView,
    saveViewScroll,
    getSavedViewScroll,
    clearViewScroll,
  };
}
