<script setup lang="ts">
import { useRouter } from '../composables/useRouter';
import AppIcon from '../components/AppIcon.vue';

const { navigateBack } = useRouter();
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
        <h2>このアプリについて</h2>
        <p>設定</p>
      </div>
    </header>

    <div class="view-scroll">
      <div class="surface-group about-card">
        <div class="about-card__mark">
          <AppIcon name="music" :size="26" />
        </div>
        <h3>UtaBuild</h3>
        <p class="about-card__subtitle">Ruby 歌詞ビューア</p>
        <p class="about-card__body">
          日本語の歌詞を検索し、ふりがな（ルビ）付きで読みやすく表示・保存・エクスポートできます。
        </p>

        <div class="about-features">
          <div class="about-feature">
            <AppIcon name="search" :size="17" />
            <span>UtaTen・QQ音楽・NetEase の横断検索</span>
          </div>
          <div class="about-feature">
            <AppIcon name="ruby-text" :size="17" />
            <span>漢字へのルビ自動アライメント表示</span>
          </div>
          <div class="about-feature">
            <AppIcon name="download" :size="17" />
            <span>HTML・ふりがなテキスト・JSON 形式のエクスポート</span>
          </div>
        </div>
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

.about-card {
  padding: 24px 20px;

  &__mark {
    width: 52px;
    height: 52px;
    border-radius: var(--radius-md);
    background: var(--primary-dim);
    color: var(--primary);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 14px;
  }

  h3 {
    font-size: 1.5rem;
    font-weight: 800;
  }

  &__subtitle {
    margin-top: 4px;
    color: var(--primary);
    font-size: var(--font-sm);
    font-weight: 700;
  }

  &__body {
    margin-top: 16px;
    color: var(--text-secondary);
    font-size: var(--font-sm);
    line-height: 1.7;
  }
}

.about-features {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border-subtle);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.about-feature {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: var(--font-xs);
  color: var(--text-secondary);

  :deep(.app-icon) {
    color: var(--primary);
  }
}
</style>
