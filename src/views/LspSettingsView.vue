<script setup lang="ts">
import { computed } from 'vue';
import { useSettings } from '../composables/useSettings';
import { useLyricsApp } from '../composables/useLyricsApp';
import { useRouter } from '../composables/useRouter';
import AppIcon from '../components/AppIcon.vue';

const { settings, updateSettings } = useSettings();
const { syncLspSettings, fetchLspLogs } = useLyricsApp();
const { navigateTo, navigateBack } = useRouter();

const lspLogEnabled = computed(() => settings.lspLogEnabled === true);
const showProofPopup = computed(() => settings.showProofPopup !== false);
const autoLaunchUtaBuild = computed(() => settings.autoLaunchUtaBuild !== false);

function onToggleLspLog(event: Event): void {
  const checked = (event.target as HTMLInputElement).checked;
  updateSettings({ lspLogEnabled: checked });
  void syncLspSettings();
}

function onTogglePopup(event: Event): void {
  updateSettings({ showProofPopup: (event.target as HTMLInputElement).checked });
  void syncLspSettings();
}

function onToggleAutoLaunch(event: Event): void {
  updateSettings({ autoLaunchUtaBuild: (event.target as HTMLInputElement).checked });
  void syncLspSettings();
}

function openLogs(): void {
  navigateTo('lspLogs', { resetScroll: true });
  void fetchLspLogs();
}
</script>

<template>
  <div class="subpage-view">
    <header class="view-header">
      <button
        type="button"
        class="icon-button"
        aria-label="設定に戻る"
        title="設定に戻る"
        @click="navigateBack"
      >
        <AppIcon name="chevron-left" :size="22" />
      </button>
      <div class="view-header__title">
        <h2>LSP 設定</h2>
        <p>LSPosed モジュールの動作設定</p>
      </div>
    </header>

    <div class="view-scroll">
      <div class="surface-group">
        <label class="pref-row">
          <div class="pref-row__text">
            <span class="pref-row__label">LSP ログ</span>
            <span class="pref-row__desc">有効にすると専用ページで LSP ログを確認できます。</span>
          </div>
          <input
            type="checkbox"
            class="miuix-switch"
            :checked="lspLogEnabled"
            @change="onToggleLspLog"
          />
        </label>

        <div v-if="lspLogEnabled" class="subpanel">
          <button type="button" class="secondary subpanel__btn" @click="openLogs">
            <AppIcon name="terminal" :size="16" />
            <span>LSP ログを見る</span>
          </button>
        </div>

        <label class="pref-row">
          <div class="pref-row__text">
            <span class="pref-row__label">Hi UtaBuild 通知</span>
            <span class="pref-row__desc">曲切替時に自動連携の確認通知を表示します。</span>
          </div>
          <input
            type="checkbox"
            class="miuix-switch"
            :checked="showProofPopup"
            @change="onTogglePopup"
          />
        </label>

        <label class="pref-row">
          <div class="pref-row__text">
            <span class="pref-row__label">UtaBuild に自動遷移</span>
            <span class="pref-row__desc">一致する歌詞がない場合、自動的に UtaBuild を開きます。</span>
          </div>
          <input
            type="checkbox"
            class="miuix-switch"
            :checked="autoLaunchUtaBuild"
            @change="onToggleAutoLaunch"
          />
        </label>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.subpage-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.pref-row {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 68px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border-subtle);
  cursor: pointer;

  &:last-child {
    border-bottom: 0;
  }

  &__text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  &__label {
    color: var(--text);
    font-size: var(--font-md);
    font-weight: 600;
  }

  &__desc {
    color: var(--text-secondary);
    font-size: var(--font-xs);
    line-height: 1.45;
  }
}

.subpanel {
  padding: 10px 16px;
  border-bottom: 1px solid var(--border-subtle);

  &__btn {
    width: 100%;
  }
}

.miuix-switch {
  appearance: none;
  -webkit-appearance: none;
  position: relative;
  width: 50px;
  height: 30px;
  flex: 0 0 50px;
  border: 0;
  border-radius: var(--radius-pill);
  background: var(--bg-secondary);
  box-shadow: inset 0 0 0 1px var(--border);
  cursor: pointer;
  transition: background var(--motion-base);

  &::before {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--text-secondary);
    box-shadow: var(--shadow-sm);
    transition: transform var(--motion-base) var(--ease-spring), background var(--motion-base);
  }

  &:checked {
    background: var(--primary);
    box-shadow: none;

    &::before {
      transform: translateX(20px);
      background: var(--primary-content);
    }
  }
}
</style>
