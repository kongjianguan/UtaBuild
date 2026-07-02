import { $$, router } from './dom.js';
export function initBottomMenu() {
    $$('[data-app-tab]').forEach((button) => {
        const btn = button;
        btn.addEventListener('click', () => {
            const tab = btn.dataset.appTab;
            if (!tab)
                return;
            if (tab !== router.current) {
                router.navigate(tab, { animate: true });
            }
        });
    });
}
//# sourceMappingURL=bottom-menu.js.map