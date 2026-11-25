<script setup lang="ts">
import { useFeedback } from '../composables/useFeedback';
import { useRouter } from '../composables/useRouter';
import AppIcon from './AppIcon.vue';

const { toast, hideToast } = useFeedback();
const { isFirstLevel } = useRouter();

function handleAction(): void {
  const action = toast.action;
  hideToast();
  action?.onClick();
}
</script>

<template>
  <Transition name="toast-slide">
    <div
      v-if="toast.visible"
      class="app-toast"
      :class="[
        `app-toast--${toast.tone}`,
        { 'app-toast--above-dock': isFirstLevel },
      ]"
      :role="toast.tone === 'error' ? 'alert' : 'status'"
      :aria-live="toast.tone === 'error' ? 'assertive' : 'polite'"
    >
      <span class="app-toast__icon">
        <AppIcon
          :name="toast.tone === 'success' ? 'check-circle' : toast.tone === 'info' ? 'info' : 'alert-circle'"
          :size="18"
        />
      </span>
      <span class="app-toast__message">{{ toast.message }}</span>
      <button
        v-if="toast.action"
        type="button"
        class="app-toast__action"
        @click="handleAction"
      >
        {{ toast.action.label }}
      </button>
      <button
        type="button"
        class="app-toast__close"
        aria-label="閉じる"
        @click="hideToast"
      >
        <AppIcon name="x" :size="16" />
      </button>
    </div>
  </Transition>
</template>
