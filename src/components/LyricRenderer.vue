<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { LyricElement } from '../types';
import { splitLyricLines } from '../services/formatters';

const props = withDefaults(
  defineProps<{
    elements: LyricElement[];
    fontSize: number;
    showRuby?: boolean;
    scrollContainer?: HTMLElement | null;
  }>(),
  {
    showRuby: true,
    scrollContainer: null,
  },
);

const linesContainerRef = ref<HTMLElement | null>(null);
const lines = computed(() => splitLyricLines(props.elements));

// Preserve the reader's visible lyric line across font size changes
watch(
  () => props.fontSize,
  (_next, _prev, onCleanup) => {
    const container = props.scrollContainer;
    const inner = linesContainerRef.value;
    if (!container || !inner) return;

    const lineEls = Array.from(inner.children).filter(
      (child): child is HTMLElement => child instanceof HTMLElement,
    );
    if (lineEls.length === 0) return;

    const scrollTop = container.scrollTop;
    const maxScroll = Math.max(0, container.scrollHeight - container.clientHeight);
    const atBottom = maxScroll - scrollTop <= 2;
    let anchorIndex = lineEls.findIndex((line) => line.offsetTop + line.offsetHeight > scrollTop + 1);
    if (anchorIndex < 0) anchorIndex = lineEls.length - 1;
    const offset = lineEls[anchorIndex].offsetTop - scrollTop;

    const rafId = requestAnimationFrame(() => {
      const updatedMax = Math.max(0, container.scrollHeight - container.clientHeight);
      const target = lineEls[Math.min(anchorIndex, lineEls.length - 1)];
      if (!target) return;
      const nextTop = atBottom ? updatedMax : target.offsetTop - offset;
      container.scrollTop = Math.max(0, Math.min(updatedMax, nextTop));
    });

    onCleanup(() => cancelAnimationFrame(rafId));
  },
  { flush: 'pre' },
);
</script>

<template>
  <div class="lyricBody" :class="{ 'lyricBody--hide-ruby': !showRuby }">
    <div
      ref="linesContainerRef"
      class="lyricBody__lines"
      :data-font-size="fontSize"
      :style="{ '--lyric-font-size': `${fontSize}px` }"
    >
      <div
        v-for="(line, lineIdx) in lines"
        :key="lineIdx"
        class="lyricBody__line"
      >
        <template v-for="(el, elIdx) in line.elements" :key="elIdx">
          <ruby v-if="el.type === 'ruby'" class="ruby">
            {{ el.base ?? '' }}<rt class="rt"><span class="rt__text">{{ el.ruby ?? '' }}</span></rt>
          </ruby>
          <template v-else>{{ el.base ?? '' }}</template>
        </template>
      </div>
    </div>
  </div>
</template>
