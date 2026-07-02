import { router } from './dom.js';
// ==================== Back Button / Popstate ====================
export function initBackButton() {
    if ('scrollRestoration' in history) {
        history.scrollRestoration = 'manual';
    }
    window.addEventListener('popstate', (event) => {
        router.handlePopstate(event);
    });
    history.replaceState({ view: 'search' }, '', '');
}
export function handleBack() {
    router.back();
}
export function canGestureBack() {
    return router.current !== 'search';
}
// ==================== Touch Swipe Gesture ====================
export function initBackGesture() {
    const edgeWidth = 28;
    const triggerDistance = 74;
    const maxVerticalDrift = 64;
    let gesture = null;
    window.addEventListener('touchstart', (event) => {
        if (!canGestureBack() || event.touches.length !== 1) {
            gesture = null;
            return;
        }
        const touch = event.touches[0];
        if (touch.clientX > edgeWidth) {
            gesture = null;
            return;
        }
        gesture = {
            startX: touch.clientX,
            startY: touch.clientY,
            active: true,
            triggered: false,
        };
    }, { passive: true });
    window.addEventListener('touchmove', (event) => {
        if (!gesture?.active || event.touches.length !== 1)
            return;
        const touch = event.touches[0];
        const dx = touch.clientX - gesture.startX;
        const dy = touch.clientY - gesture.startY;
        if (Math.abs(dy) > maxVerticalDrift && Math.abs(dy) > dx) {
            gesture.active = false;
            return;
        }
        if (!gesture.triggered &&
            dx >= triggerDistance &&
            Math.abs(dy) <= maxVerticalDrift) {
            gesture.triggered = true;
            gesture.active = false;
            handleBack();
        }
    }, { passive: true });
    window.addEventListener('touchend', () => {
        gesture = null;
    }, { passive: true });
    window.addEventListener('touchcancel', () => {
        gesture = null;
    }, { passive: true });
}
//# sourceMappingURL=back-gesture.js.map