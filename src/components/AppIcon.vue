<script setup lang="ts">
export type IconName =
  | 'search'
  | 'music'
  | 'sliders'
  | 'chevron-left'
  | 'chevron-right'
  | 'more-vertical'
  | 'download'
  | 'copy'
  | 'check'
  | 'x'
  | 'trash'
  | 'refresh'
  | 'sun'
  | 'moon'
  | 'info'
  | 'terminal'
  | 'code'
  | 'file-text'
  | 'ruby-text'
  | 'alert-circle'
  | 'check-circle'
  | 'arrow-up-left'
  | 'plus'
  | 'minus'
  | 'eye'
  | 'eye-off'
  | 'font-size';

withDefaults(
  defineProps<{
    name: IconName;
    size?: number;
  }>(),
  {
    size: 20,
  },
);
</script>

<template>
  <svg
    class="app-icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.85"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
  >
    <template v-if="name === 'search'">
      <circle cx="11" cy="11" r="7" />
      <path d="m16.5 16.5 4.5 4.5" />
    </template>
    <template v-else-if="name === 'music'">
      <path d="M9 18V5l11-2v13" />
      <circle cx="6" cy="18" r="3" />
      <circle cx="17" cy="16" r="3" />
    </template>
    <template v-else-if="name === 'sliders'">
      <path d="M4 6h16M4 12h16M4 18h16" />
      <circle cx="9" cy="6" r="2" fill="var(--bg)" />
      <circle cx="15" cy="12" r="2" fill="var(--bg)" />
      <circle cx="8" cy="18" r="2" fill="var(--bg)" />
    </template>
    <template v-else-if="name === 'chevron-left'">
      <path d="m15 18-6-6 6-6" />
    </template>
    <template v-else-if="name === 'chevron-right'">
      <path d="m9 18 6-6-6-6" />
    </template>
    <template v-else-if="name === 'more-vertical'">
      <circle cx="12" cy="5" r="1.25" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="1.25" fill="currentColor" stroke="none" />
      <circle cx="12" cy="19" r="1.25" fill="currentColor" stroke="none" />
    </template>
    <template v-else-if="name === 'download'">
      <path d="M12 3v12" />
      <path d="m7 10 5 5 5-5" />
      <path d="M4 19h16" />
    </template>
    <template v-else-if="name === 'copy'">
      <rect x="9" y="9" width="11" height="11" rx="2.5" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </template>
    <template v-else-if="name === 'check'">
      <path d="M5 12.5 9.5 17 19 7.5" />
    </template>
    <template v-else-if="name === 'x'">
      <path d="M18 6 6 18M6 6l12 12" />
    </template>
    <template v-else-if="name === 'trash'">
      <path d="M3 6h18" />
      <path d="M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2" />
      <path d="M6 6l1 13a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-13" />
    </template>
    <template v-else-if="name === 'refresh'">
      <path d="M21 12a9 9 0 1 1-2.64-6.36" />
      <path d="M21 3v6h-6" />
    </template>
    <template v-else-if="name === 'sun'">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
    </template>
    <template v-else-if="name === 'moon'">
      <path d="M20.5 14.5A8.5 8.5 0 0 1 9.5 3.5a8.5 8.5 0 1 0 11 11z" />
    </template>
    <template v-else-if="name === 'info'">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 16v-4" />
      <circle cx="12" cy="8.5" r="0.8" fill="currentColor" stroke="none" />
    </template>
    <template v-else-if="name === 'terminal'">
      <rect x="3" y="4" width="18" height="16" rx="3" />
      <path d="m7 10 3 2.5L7 15" />
      <path d="M13 15h4" />
    </template>
    <template v-else-if="name === 'code'">
      <path d="m8 8-4 4 4 4" />
      <path d="m16 8 4 4-4 4" />
      <path d="m13.5 5-3 14" />
    </template>
    <template v-else-if="name === 'file-text'">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <path d="M14 2v6h6" />
      <path d="M8 13h8M8 17h5" />
    </template>
    <template v-else-if="name === 'ruby-text'">
      <path d="M4 18h16" />
      <path d="M7 14 12 6l5 8" />
      <path d="M9 11.5h6" />
    </template>
    <template v-else-if="name === 'alert-circle'">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 8v4.5" />
      <circle cx="12" cy="16" r="0.8" fill="currentColor" stroke="none" />
    </template>
    <template v-else-if="name === 'check-circle'">
      <circle cx="12" cy="12" r="9" />
      <path d="m8.5 12.2 2.4 2.4 4.8-5" />
    </template>
    <template v-else-if="name === 'arrow-up-left'">
      <path d="M17 17 7 7" />
      <path d="M15 7H7v8" />
    </template>
    <template v-else-if="name === 'plus'">
      <path d="M12 5v14M5 12h14" />
    </template>
    <template v-else-if="name === 'minus'">
      <path d="M5 12h14" />
    </template>
    <template v-else-if="name === 'eye'">
      <path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6-10-6-10-6z" />
      <circle cx="12" cy="12" r="2.5" />
    </template>
    <template v-else-if="name === 'eye-off'">
      <path d="M3 3l18 18" />
      <path d="M10.6 10.6a2.5 2.5 0 0 0 3.3 3.3" />
      <path d="M9.8 5.2A10.7 10.7 0 0 1 12 5c6.5 0 10 7 10 7a16.8 16.8 0 0 1-4.1 4.9" />
      <path d="M6.6 6.6C4 8.3 2 12 2 12s3.5 6 10 6c1.6 0 3-.4 4.2-1" />
    </template>
    <template v-else-if="name === 'font-size'">
      <path d="M4 18 9 6l5 12" />
      <path d="M5.7 14h6.6" />
      <path d="M16 18v-7" />
      <path d="M14.5 13h5" />
    </template>
  </svg>
</template>

<style scoped>
.app-icon {
  display: block;
  flex-shrink: 0;
}
</style>
