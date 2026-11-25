<script setup lang="ts">
import { onMounted } from 'vue';
import { useLyricsApp } from '../composables/useLyricsApp';
import { useRouter } from '../composables/useRouter';
import AppIcon from '../components/AppIcon.vue';

const { lspLogs, isLoadingLspLogs, lspLogZoom, fetchLspLogs, adjustLspLogZoom } = useLyricsApp();
const { navigateBack } = useRouter();

onMounted(() => {
  if (!lspLogs.value) {
    void fetchLspLogs();
  }
});
</script>

<template>
  <div class="lsp-log-view">
    <header class="view-header">
      <button
        type="button"
        class="icon-button"
        aria-label="戻る"
        title="戻る"
        @click="navigateBack"
      >
        <AppIcon name="chevron-left" :size="22" />
      </button>
      <div class="view-header__title">
        <h2>LSP ログ</h2>
        <p>横・縦スクロールと文字拡大縮小に対応</p>
      </div>
      <div class="view-header__actions">
        <button
          type="button"
          class="icon-button"
          :disabled="isLoadingLspLogs"
          aria-label="更新"
          title="更新"
          @click="void fetchLspLogs()"
        >
          <span v-if="isLoadingLspLogs" class="inline-spinner" aria-hidden="true" />
          <AppIcon v-else name="refresh" :size="18" />
        </button>
      </div>
    </header>

    <div class="lsp-toolbar">
      <span class="lsp-toolbar__label">文字倍率</span>
      <div class="lsp-zoom">
        <button
          type="button"
          class="icon-button icon-button--sm"
          aria-label="ログを縮小"
          @click="adjustLspLogZoom(-0.1)"
        >
          <AppIcon name="minus" :size="15" />
        </button>
        <span class="lsp-zoom__val">{{ Math.round(lspLogZoom * 100) }}%</span>
        <button
          type="button"
          class="icon-button icon-button--sm"
          aria-label="ログを拡大"
          @click="adjustLspLogZoom(0.1)"
        >
          <AppIcon name="plus" :size="15" />
        </button>
      </div>
    </div>

    <div class="surface-group lsp-frame">
      <pre
        class="lsp-content"
        :style="{ '--lsp-log-font-scale': String(lspLogZoom) }"
      >{{ lspLogs }}</pre>
    </div>
  </div>
</template>

<style scoped lang="scss">
.lsp-log-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  padding-bottom: 16px;
}

.lsp-toolbar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;

  &__label {
    font-size: var(--font-xs);
    font-weight: 600;
    color: var(--text-muted);
  }
}

.lsp-zoom {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 6px;
  border-radius: var(--radius-md);
  background: var(--surface);
  border: 1px solid var(--border-subtle);

  &__val {
    min-width: 44px;
    text-align: center;
    font-size: var(--font-xs);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
}

.lsp-frame {
  flex: 1 1 auto;
  min-height: 0;
}

.lsp-content {
  width: 100%;
  height: 100%;
  margin: 0;
  padding: 16px;
  overflow: auto;
  white-space: pre;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace;
  font-size: calc(var(--lsp-log-font-scale, 1) * var(--font-xs));
  line-height: 1.5;
  color: var(--text);
  background: var(--glass-code-bg);
  touch-action: pan-x pan-y;
  overscroll-behavior: contain;
  scrollbar-width: thin;
}
</style>
