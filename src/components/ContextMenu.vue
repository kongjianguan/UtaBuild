<script setup lang="ts">
import { computed, onBeforeUnmount, watch } from 'vue';
import AppIcon, { type IconName } from './AppIcon.vue';

export interface ContextMenuItem {
  id: string;
  label: string;
  icon: IconName;
  tone?: 'default' | 'primary' | 'danger';
}

const props = defineProps<{
  open: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
}>();

const emit = defineEmits<{
  close: [];
  select: [id: string];
}>();

const menuStyle = computed(() => {
  const estimatedWidth = 196;
  const estimatedHeight = props.items.length * 46 + 16;
  const margin = 12;
  const maxX = typeof window !== 'undefined' ? window.innerWidth - estimatedWidth - margin : props.x;
  const maxY = typeof window !== 'undefined' ? window.innerHeight - estimatedHeight - margin : props.y;
  const left = Math.max(margin, Math.min(props.x, maxX));
  const top = Math.max(margin, Math.min(props.y, maxY));
  return {
    left: `${left}px`,
    top: `${top}px`,
  };
});

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.preventDefault();
    emit('close');
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      window.addEventListener('keydown', onKeydown);
    } else {
      window.removeEventListener('keydown', onKeydown);
    }
  },
);

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="context-menu-backdrop" @click="emit('close')" />
    <Transition name="menu-pop">
      <div
        v-if="open"
        class="context-menu"
        role="menu"
        :style="menuStyle"
      >
        <button
          v-for="item in items"
          :key="item.id"
          type="button"
          role="menuitem"
          class="context-menu__item"
          :class="item.tone ? `context-menu__item--${item.tone}` : undefined"
          @click="emit('select', item.id)"
        >
          <AppIcon :name="item.icon" :size="16" />
          <span>{{ item.label }}</span>
        </button>
      </div>
    </Transition>
  </Teleport>
</template>
