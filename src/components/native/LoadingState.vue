<!--
  加载态（docs/design.md §4）：<1s 不显示任何指示；1–10s 用骨架屏；>10s 用进度 + 取消。
  本组件只负责「骨架屏」与「行内转圈」两种标准形态，长任务进度用 ProgressBar + core/tasks 取消。
-->
<script setup lang="ts">
import { Skeleton } from '@/components/ui/skeleton';

withDefaults(
  defineProps<{
    /** skeleton：结构与真实内容一致的骨架屏；spinner：行内转圈（仅用于无法预估结构的场景） */
    variant?: 'skeleton' | 'spinner';
    /** skeleton 的行数 */
    rows?: number;
    /** 文案（spinner 时显示） */
    label?: string;
  }>(),
  { variant: 'skeleton', rows: 3, label: '加载中…' }
);
</script>

<template>
  <div v-if="variant === 'skeleton'" class="space-y-2">
    <Skeleton
      v-for="row in rows"
      :key="row"
      class="h-4"
      :class="row % 3 === 0 ? 'w-1/2' : 'w-2/3'"
    />
  </div>
  <div v-else class="flex items-center gap-2 py-4 text-muted-foreground">
    <div class="size-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
    <span class="text-sm">{{ label }}</span>
  </div>
</template>
