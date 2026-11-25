<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { useFeedback } from '../composables/useFeedback';

const { confirmState, resolveConfirm } = useFeedback();
const cancelBtnRef = ref<HTMLButtonElement | null>(null);
const confirmBtnRef = ref<HTMLButtonElement | null>(null);
let prevFocused: HTMLElement | null = null;

function handleKeydown(event: KeyboardEvent): void {
  if (!confirmState.open) return;
  if (event.key === 'Escape') {
    event.preventDefault();
    resolveConfirm(false);
    return;
  }
  if (event.key === 'Tab') {
    const focusables = [cancelBtnRef.value, confirmBtnRef.value].filter(
      (btn): btn is HTMLButtonElement => Boolean(btn),
    );
    if (focusables.length === 0) return;
    event.preventDefault();
    const idx = focusables.indexOf(document.activeElement as HTMLButtonElement);
    const nextIdx = event.shiftKey
      ? (idx <= 0 ? focusables.length - 1 : idx - 1)
      : (idx + 1) % focusables.length;
    focusables[nextIdx].focus();
  }
}

watch(
  () => confirmState.open,
  (isOpen) => {
    if (isOpen) {
      prevFocused = document.activeElement as HTMLElement | null;
      window.addEventListener('keydown', handleKeydown, true);
      void nextTick(() => cancelBtnRef.value?.focus());
    } else {
      window.removeEventListener('keydown', handleKeydown, true);
      prevFocused?.focus?.();
      prevFocused = null;
    }
  },
);

onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown, true);
});
</script>

<template>
  <Transition name="dialog-fade">
    <div
      v-if="confirmState.open"
      class="confirm-dialog"
      @click.self="resolveConfirm(false)"
    >
      <div
        class="confirm-dialog__panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-dialog-title"
        aria-describedby="confirm-dialog-message"
      >
        <h2 id="confirm-dialog-title" class="confirm-dialog__title">{{ confirmState.title }}</h2>
        <p id="confirm-dialog-message" class="confirm-dialog__message">{{ confirmState.message }}</p>
        <div class="confirm-dialog__actions">
          <button
            ref="cancelBtnRef"
            type="button"
            class="secondary"
            @click="resolveConfirm(false)"
          >
            {{ confirmState.cancelLabel }}
          </button>
          <button
            ref="confirmBtnRef"
            type="button"
            class="confirm-dialog__confirm"
            :class="{ 'confirm-dialog__confirm--danger': confirmState.danger }"
            @click="resolveConfirm(true)"
          >
            {{ confirmState.confirmLabel }}
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>
