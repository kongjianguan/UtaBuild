<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  copyTextToClipboard,
  formatPlainText,
  formatRubyBracketText,
  formatStructuredJson,
  normalizeCoverUrl,
  saveExportHtml,
  saveExportTextFile,
  splitLyricLines,
} from '../services/formatters';
import { useLyricsApp } from '../composables/useLyricsApp';
import { useRouter } from '../composables/useRouter';
import { useFeedback } from '../composables/useFeedback';
import { useSettings } from '../composables/useSettings';
import AppIcon from '../components/AppIcon.vue';

const { exportTarget } = useLyricsApp();
const { navigateBack } = useRouter();
const { showSuccess, showInfo, showError } = useFeedback();
const { settings } = useSettings();

const htmlPreviewTheme = ref<'dark' | 'light'>(settings.theme || 'dark');
const busyAction = ref<string | null>(null);
const copiedKey = ref<string | null>(null);

const lines = computed(() => splitLyricLines(exportTarget.value?.rubyAnnotations));
const plainText = computed(() => formatPlainText(exportTarget.value?.rubyAnnotations));
const rubyBracketText = computed(() => formatRubyBracketText(exportTarget.value?.rubyAnnotations));
const structuredJson = computed(() =>
  exportTarget.value ? formatStructuredJson(exportTarget.value) : '',
);
const coverUrl = computed(() => normalizeCoverUrl(exportTarget.value?.coverUrl));

function markCopied(key: string): void {
  copiedKey.value = key;
  setTimeout(() => {
    if (copiedKey.value === key) copiedKey.value = null;
  }, 1800);
}

async function handleCopy(key: string, text: string, label: string): Promise<void> {
  const ok = await copyTextToClipboard(text);
  if (ok) {
    markCopied(key);
    showSuccess(`${label}をクリップボードにコピーしました`);
  } else {
    showError('クリップボードへのコピーに失敗しました');
  }
}

async function handleSaveHtml(): Promise<void> {
  if (!exportTarget.value || busyAction.value) return;
  busyAction.value = 'html';
  try {
    const saved = await saveExportHtml(exportTarget.value);
    if (saved) showSuccess('HTML ファイルを保存しました');
    else showInfo('エクスポートをキャンセルしました');
  } catch (err) {
    showError(`HTML エクスポートに失敗しました: ${err}`);
  } finally {
    busyAction.value = null;
  }
}

async function handleSaveText(
  key: string,
  extension: 'txt' | 'json',
  content: string,
  label: string,
): Promise<void> {
  if (!exportTarget.value || busyAction.value) return;
  busyAction.value = key;
  try {
    const saved = await saveExportTextFile(exportTarget.value, extension, content);
    if (saved) showSuccess(`${label}を保存しました`);
    else showInfo('エクスポートをキャンセルしました');
  } catch (err) {
    showError(`保存に失敗しました: ${err}`);
  } finally {
    busyAction.value = null;
  }
}
</script>

<template>
  <div class="export-view">
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
        <h2>エクスポート</h2>
        <p v-if="exportTarget">
          {{ exportTarget.title }}{{ exportTarget.artist ? ` — ${exportTarget.artist}` : '' }}
        </p>
      </div>
    </header>

    <div class="view-scroll">
      <div v-if="!exportTarget" class="empty-card">
        <span class="empty-card__icon">
          <AppIcon name="download" :size="20" />
        </span>
        <p class="empty-card__title">エクスポートする曲が選択されていません</p>
        <p class="empty-card__desc">歌詞画面または保存済みリストから曲を選んでください。</p>
      </div>

      <div v-else class="export-column">
        <!-- 1. Standalone HTML Preview Card -->
        <article class="export-card surface-group">
          <div class="export-card__head">
            <div class="export-card__meta">
              <span class="export-card__badge">
                <AppIcon name="ruby-text" :size="15" />
                <span>HTML ページ</span>
              </span>
              <h3>ルビ付き Web ページ</h3>
              <p>ジャケット画像・テーマ切替・文字サイズ調整機能付きの単一 HTML ファイルとして書き出します。</p>
            </div>
          </div>

          <div class="export-card__toolbar">
            <span class="export-card__preview-label">外観プレビュー</span>
            <div class="segmented-pill">
              <button
                type="button"
                :class="{ active: htmlPreviewTheme === 'dark' }"
                @click="htmlPreviewTheme = 'dark'"
              >
                ダーク
              </button>
              <button
                type="button"
                :class="{ active: htmlPreviewTheme === 'light' }"
                @click="htmlPreviewTheme = 'light'"
              >
                ライト
              </button>
            </div>
          </div>

          <div
            class="html-mockup"
            :class="`html-mockup--${htmlPreviewTheme}`"
          >
            <div class="html-mockup__header">
              <span
                class="html-mockup__cover"
                :style="coverUrl ? { backgroundImage: `url(${coverUrl})` } : undefined"
              >
                <AppIcon v-if="!coverUrl" name="music" :size="18" />
              </span>
              <div class="html-mockup__title-block">
                <strong>{{ exportTarget.title }}</strong>
                <span>{{ exportTarget.artist || '不明なアーティスト' }}</span>
              </div>
            </div>
            <div class="html-mockup__body">
              <div
                v-for="(line, lIdx) in lines"
                :key="lIdx"
                class="html-mockup__line"
              >
                <template v-for="(el, eIdx) in line.elements" :key="eIdx">
                  <ruby v-if="el.type === 'ruby'">
                    {{ el.base ?? '' }}<rt>{{ el.ruby ?? '' }}</rt>
                  </ruby>
                  <template v-else>{{ el.base ?? '' }}</template>
                </template>
              </div>
            </div>
          </div>

          <div class="export-card__actions">
            <button
              type="button"
              class="primary"
              :disabled="busyAction !== null"
              @click="void handleSaveHtml()"
            >
              <span v-if="busyAction === 'html'" class="inline-spinner" aria-hidden="true" />
              <AppIcon v-else name="download" :size="16" />
              <span>HTML を保存 (.html)</span>
            </button>
          </div>
        </article>

        <!-- 2. Ruby Bracket Text Preview Card -->
        <article class="export-card surface-group">
          <div class="export-card__head">
            <div class="export-card__meta">
              <span class="export-card__badge">
                <AppIcon name="file-text" :size="15" />
                <span>ルビ括弧テキスト</span>
              </span>
              <h3>括弧ふりがな付きテキスト</h3>
              <p>漢字の直後に括弧で読みを添えたテキスト形式です。メモや学習ノートへの貼り付けに適しています。</p>
            </div>
          </div>

          <pre class="text-preview">{{ rubyBracketText }}</pre>

          <div class="export-card__actions">
            <button
              type="button"
              class="secondary"
              @click="void handleCopy('ruby-txt', rubyBracketText, '括弧ふりがなテキスト')"
            >
              <AppIcon :name="copiedKey === 'ruby-txt' ? 'check' : 'copy'" :size="16" />
              <span>{{ copiedKey === 'ruby-txt' ? 'コピー済み' : 'コピー' }}</span>
            </button>
            <button
              type="button"
              class="primary"
              :disabled="busyAction !== null"
              @click="void handleSaveText('ruby-txt', 'txt', rubyBracketText, '括弧ふりがなテキスト')"
            >
              <span v-if="busyAction === 'ruby-txt'" class="inline-spinner" aria-hidden="true" />
              <AppIcon v-else name="download" :size="16" />
              <span>保存 (.txt)</span>
            </button>
          </div>
        </article>

        <!-- 3. Plain Text Preview Card -->
        <article class="export-card surface-group">
          <div class="export-card__head">
            <div class="export-card__meta">
              <span class="export-card__badge">
                <AppIcon name="file-text" :size="15" />
                <span>プレーンテキスト</span>
              </span>
              <h3>歌詞のみ（ルビなし）</h3>
              <p>ふりがなを取り除いた純粋な歌詞本文です。</p>
            </div>
          </div>

          <pre class="text-preview">{{ plainText }}</pre>

          <div class="export-card__actions">
            <button
              type="button"
              class="secondary"
              @click="void handleCopy('plain-txt', plainText, '歌詞テキスト')"
            >
              <AppIcon :name="copiedKey === 'plain-txt' ? 'check' : 'copy'" :size="16" />
              <span>{{ copiedKey === 'plain-txt' ? 'コピー済み' : 'コピー' }}</span>
            </button>
            <button
              type="button"
              class="primary"
              :disabled="busyAction !== null"
              @click="void handleSaveText('plain-txt', 'txt', plainText, '歌詞テキスト')"
            >
              <span v-if="busyAction === 'plain-txt'" class="inline-spinner" aria-hidden="true" />
              <AppIcon v-else name="download" :size="16" />
              <span>保存 (.txt)</span>
            </button>
          </div>
        </article>

        <!-- 4. Structured JSON Preview Card -->
        <article class="export-card surface-group">
          <div class="export-card__head">
            <div class="export-card__meta">
              <span class="export-card__badge">
                <AppIcon name="code" :size="15" />
                <span>構造化 JSON</span>
              </span>
              <h3>UtaBuild JSON データ</h3>
              <p>行ごとのテキスト要素とルビ要素を保持した構造化データです。外部ツールや自動処理に利用できます。</p>
            </div>
          </div>

          <pre class="text-preview text-preview--code">{{ structuredJson }}</pre>

          <div class="export-card__actions">
            <button
              type="button"
              class="secondary"
              @click="void handleCopy('json', structuredJson, 'JSON')"
            >
              <AppIcon :name="copiedKey === 'json' ? 'check' : 'copy'" :size="16" />
              <span>{{ copiedKey === 'json' ? 'コピー済み' : 'コピー' }}</span>
            </button>
            <button
              type="button"
              class="primary"
              :disabled="busyAction !== null"
              @click="void handleSaveText('json', 'json', structuredJson, 'JSON')"
            >
              <span v-if="busyAction === 'json'" class="inline-spinner" aria-hidden="true" />
              <AppIcon v-else name="download" :size="16" />
              <span>保存 (.json)</span>
            </button>
          </div>
        </article>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.export-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.export-column {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.export-card {
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 14px;

  &__meta {
    h3 {
      margin-top: 6px;
      font-size: var(--font-lg);
      font-weight: 700;
    }

    p {
      margin-top: 4px;
      color: var(--text-secondary);
      font-size: var(--font-xs);
      line-height: 1.5;
    }
  }

  &__badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px;
    border-radius: var(--radius-pill);
    background: var(--primary-dim);
    color: var(--primary);
    font-size: var(--font-xs);
    font-weight: 700;
  }

  &__toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  &__preview-label {
    font-size: var(--font-xs);
    font-weight: 600;
    color: var(--text-muted);
  }

  &__actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;

    button {
      min-height: 40px;
      padding: 8px 14px;
      font-size: var(--font-xs);
    }
  }
}

.html-mockup {
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  padding: 16px;
  max-height: 240px;
  overflow-y: auto;
  scrollbar-width: thin;

  &--dark {
    background: #0d1117;
    color: #e6edf3;

    .html-mockup__title-block span,
    .html-mockup__body rt {
      color: #8b949e;
    }
  }

  &--light {
    background: #ffffff;
    color: #1f2328;

    .html-mockup__title-block span,
    .html-mockup__body rt {
      color: #656d76;
    }
  }

  &__header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding-bottom: 12px;
    margin-bottom: 12px;
    border-bottom: 1px solid rgba(128, 128, 128, 0.22);
  }

  &__cover {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    background-color: rgba(128, 128, 128, 0.16);
    background-size: cover;
    background-position: center;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  &__title-block {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;

    strong {
      font-size: 0.95rem;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    span {
      font-size: 0.75rem;
    }
  }

  &__body {
    font-size: 0.92rem;
    line-height: 2.15;

    rt {
      font-size: 0.62em;
    }
  }

  &__line {
    min-height: 0.8em;
  }
}

.text-preview {
  margin: 0;
  padding: 14px;
  max-height: 200px;
  overflow: auto;
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  border: 1px solid var(--border-subtle);
  color: var(--text);
  font-size: 0.82rem;
  line-height: 1.7;
  white-space: pre-wrap;
  word-break: break-word;
  scrollbar-width: thin;

  &--code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace;
    font-size: 0.75rem;
    line-height: 1.5;
    white-space: pre;
    word-break: normal;
  }
}
</style>
