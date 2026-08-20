import { invoke } from './tauri.js';
import { el, show, hide, showLoading, hideLoading, showError, showSuccess, } from './dom.js';
// ==================== Cache Clearing State ====================
export let clearCacheConfirmationActive = false;
let activeConfirmationCleanup = null;
const CLEAR_CACHE_CONFIRMATION_COOLDOWN_MS = 3000;
const CLEAR_CACHE_CONFIRMATION_STEPS = [
    {
        title: 'キャッシュを削除',
        message: '検索結果キャッシュとメモリキャッシュを削除します。保存済みの曲は削除されません。',
        confirmLabel: '1回目の確認',
    },
    {
        title: 'もう一度確認',
        message: 'この操作はすぐに実行され、削除したキャッシュは復元できません。',
        confirmLabel: '2回目の確認',
    },
    {
        title: '最後の確認',
        message: '本当にキャッシュを削除する場合のみ、最後の確認を押してください。',
        confirmLabel: '削除を実行',
    },
];
// ==================== Clear All Caches ====================
export async function clearAllCaches() {
    showLoading();
    try {
        await invoke('clear_cache');
        showSuccess('検索キャッシュを削除しました。保存済みの曲は削除されていません');
    }
    catch (err) {
        console.error('Clear cache error:', err);
        showError(`キャッシュの削除に失敗しました: ${err}`);
    }
    finally {
        hideLoading();
    }
}
// ==================== Confirmation Dialog ====================
export function closeConfirmationDialog() {
    if (activeConfirmationCleanup) {
        activeConfirmationCleanup();
        activeConfirmationCleanup = null;
    }
    hide(el('confirm-dialog'));
}
function requestConfirmationStep(step, stepIndex, totalSteps) {
    return new Promise((resolve) => {
        let remainingSeconds = Math.ceil(CLEAR_CACHE_CONFIRMATION_COOLDOWN_MS / 1000);
        let isResolved = false;
        const confirmDialog = el('confirm-dialog');
        const confirmButton = el('confirm-dialog-confirm');
        const cancelButton = el('confirm-dialog-cancel');
        const updateConfirmLabel = () => {
            confirmButton.textContent =
                remainingSeconds > 0
                    ? `${step.confirmLabel}（${remainingSeconds}）`
                    : step.confirmLabel;
        };
        const finish = (confirmed) => {
            if (isResolved)
                return;
            isResolved = true;
            clearInterval(intervalId);
            clearTimeout(timeoutId);
            document.removeEventListener('keydown', handleKeydown);
            confirmButton.removeEventListener('click', handleConfirm);
            cancelButton.removeEventListener('click', handleCancel);
            hide(confirmDialog);
            activeConfirmationCleanup = null;
            resolve(confirmed);
        };
        const handleConfirm = () => finish(true);
        const handleCancel = () => finish(false);
        const handleKeydown = (event) => {
            if (event.key === 'Escape') {
                finish(false);
            }
        };
        el('confirm-dialog-step').textContent = `${stepIndex + 1}/${totalSteps}`;
        el('confirm-dialog-title').textContent = step.title;
        el('confirm-dialog-message').textContent = step.message;
        confirmButton.disabled = true;
        updateConfirmLabel();
        show(confirmDialog);
        cancelButton.focus();
        const intervalId = setInterval(() => {
            remainingSeconds -= 1;
            updateConfirmLabel();
        }, 1000);
        const timeoutId = setTimeout(() => {
            remainingSeconds = 0;
            confirmButton.disabled = false;
            clearInterval(intervalId);
            updateConfirmLabel();
        }, CLEAR_CACHE_CONFIRMATION_COOLDOWN_MS);
        confirmButton.addEventListener('click', handleConfirm);
        cancelButton.addEventListener('click', handleCancel);
        document.addEventListener('keydown', handleKeydown);
        activeConfirmationCleanup = () => finish(false);
    });
}
export async function confirmClearAllCaches() {
    if (clearCacheConfirmationActive) {
        return false;
    }
    clearCacheConfirmationActive = true;
    try {
        for (let index = 0; index < CLEAR_CACHE_CONFIRMATION_STEPS.length; index += 1) {
            const confirmed = await requestConfirmationStep(CLEAR_CACHE_CONFIRMATION_STEPS[index], index, CLEAR_CACHE_CONFIRMATION_STEPS.length);
            if (!confirmed) {
                return false;
            }
        }
        return true;
    }
    finally {
        closeConfirmationDialog();
        clearCacheConfirmationActive = false;
    }
}
//# sourceMappingURL=cache.js.map