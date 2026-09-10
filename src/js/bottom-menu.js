import { $$, el, router, updateBottomMenuSlider } from './dom.js';
import { loadSavedLyrics } from './songs.js';
const APP_TAB_ORDER = ['search', 'songs', 'settings'];
function activateTab(tab, moveFocus = false) {
    if (tab === router.current)
        return;
    // First-level tab switches replace the history entry so the Android back
    // button does not replay every tab visit before leaving the app.
    router.navigate(tab, { animate: true, replace: true });
    if (tab === 'songs') {
        void loadSavedLyrics();
    }
    if (moveFocus) {
        const btn = document.querySelector(`[data-app-tab="${tab}"]`);
        btn?.focus();
    }
}
export function initBottomMenu() {
    $$('[data-app-tab]').forEach((button) => {
        const btn = button;
        btn.addEventListener('click', () => {
            const tab = btn.dataset.appTab;
            if (!tab)
                return;
            activateTab(tab);
        });
    });
    // Arrow-key navigation for the tablist (roving tabindex lives in dom.ts).
    el('bottom-menu').addEventListener('keydown', (event) => {
        if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight')
            return;
        const tabs = APP_TAB_ORDER
            .map((tab) => document.querySelector(`[data-app-tab="${tab}"]`))
            .filter((tab) => Boolean(tab));
        if (tabs.length === 0)
            return;
        event.preventDefault();
        const currentIndex = tabs.findIndex((tab) => tab === document.activeElement);
        const delta = event.key === 'ArrowRight' ? 1 : -1;
        const nextIndex = ((currentIndex < 0 ? 0 : currentIndex) + delta + tabs.length) % tabs.length;
        const nextTab = tabs[nextIndex]?.dataset.appTab;
        if (!nextTab)
            return;
        activateTab(nextTab, true);
    });
    let resizeRafId = null;
    window.addEventListener('resize', () => {
        if (resizeRafId !== null)
            cancelAnimationFrame(resizeRafId);
        resizeRafId = requestAnimationFrame(() => {
            resizeRafId = null;
            updateBottomMenuSlider();
        });
    });
}
//# sourceMappingURL=bottom-menu.js.map