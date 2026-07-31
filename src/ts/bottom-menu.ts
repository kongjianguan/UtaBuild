import { $$, router } from './dom.js';
import { loadSavedLyrics } from './songs.js';

export function initBottomMenu(): void {
  $$('[data-app-tab]').forEach((button) => {
    const btn = button as HTMLElement;
    btn.addEventListener('click', () => {
      const tab = btn.dataset.appTab;
      if (!tab) return;
      if (tab !== router.current) {
        router.navigate(tab as 'search' | 'songs' | 'settings', { animate: true });
        if (tab === 'songs') {
          void loadSavedLyrics();
        }
      }
    });
  });
}
