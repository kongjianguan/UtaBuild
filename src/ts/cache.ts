import { invoke } from './tauri.js';
import {
  confirmDialog,
  showLoading,
  hideLoading,
  showError,
  showSuccess,
} from './dom.js';

// ==================== Clear All Caches ====================

export async function clearAllCaches(): Promise<void> {
  showLoading();

  try {
    await invoke('clear_cache');
    showSuccess('検索キャッシュを削除しました。保存済みの曲は削除されていません');
  } catch (err) {
    console.error('Clear cache error:', err);
    showError(`キャッシュの削除に失敗しました: ${err}`, {
      label: '再試行',
      onClick: () => void clearAllCaches(),
    });
  } finally {
    hideLoading();
  }
}

// Cache clearing is low-risk (saved songs are unaffected), so a single
// confirmation step is enough.
export async function confirmClearAllCaches(): Promise<boolean> {
  return confirmDialog({
    title: 'キャッシュを削除',
    message:
      '検索結果キャッシュとメモリキャッシュを削除します。保存済みの曲は削除されません。',
    confirmLabel: '削除',
  });
}
