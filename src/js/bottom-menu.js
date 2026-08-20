import { $$, router } from './dom.js';
import { loadSavedLyrics } from './songs.js';
export function initBottomMenu() {
    $$('[data-app-tab]').forEach((button) => {
        const btn = button;
        btn.addEventListener('click', () => {
            const tab = btn.dataset.appTab;
            if (!tab)
                return;
            if (tab !== router.current) {
                router.navigate(tab, { animate: true });
                if (tab === 'songs') {
                    void loadSavedLyrics();
                }
            }
        });
    });
}
//# sourceMappingURL=bottom-menu.js.map