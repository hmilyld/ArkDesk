<!--
  结果展示框：等宽文本 + 复制按钮，可显示加载/占位态。
-->
<script setup lang="ts">
import CopyButton from './CopyButton.vue';

withDefaults(
  defineProps<{
    value: string;
    label?: string;
    placeholder?: string;
    loading?: boolean;
    /** 结果较长时限制高度，默认 260px */
    maxHeight?: string;
    /** 语义色调（如验签通过/失败） */
    tone?: 'default' | 'success' | 'danger';
  }>(),
  {
    label: '结果',
    placeholder: '结果将显示在这里',
    loading: false,
    maxHeight: '260px',
    tone: 'default',
  }
);
</script>

<template>
  <div class="overflow-hidden rounded-lg border bg-card">
    <div class="flex items-center justify-between border-b px-4 py-2">
      <span class="text-xs font-medium text-muted-foreground">{{ label }}</span>
      <CopyButton :value="value" />
    </div>
    <div class="p-4">
      <div v-if="loading" class="flex items-center gap-2 py-4 text-muted-foreground">
        <div
          class="size-4 animate-spin rounded-full border-2 border-current border-t-transparent"
        />
        <span class="text-sm">计算中…</span>
      </div>
      <pre
        v-else-if="value"
        class="overflow-auto whitespace-pre-wrap break-all font-mono text-sm leading-relaxed"
        :class="tone === 'success' ? 'text-success' : tone === 'danger' ? 'text-destructive' : ''"
        :style="{ maxHeight }"
        >{{ value }}</pre>
      <p v-else class="py-4 text-center text-sm text-muted-foreground">{{ placeholder }}</p>
    </div>
  </div>
</template>
