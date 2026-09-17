<!--
  空态（docs/design.md §4）：图标 + 主文案 + 说明 + 一个动作。
  - 放在 Panel 内时用默认样式（无虚线框，避免卡片套卡片）
  - 整页/整表为空时传 `framed` 显示虚线容器
-->
<script setup lang="ts">
import type { Component } from 'vue';
import { cn } from '@/lib/utils';

withDefaults(
  defineProps<{
    /** 图标组件（lucide），尺寸由组件统一为 20 */
    icon?: Component;
    /** 主文案（一句话说明「这里是什么」） */
    title: string;
    /** 补充说明：下一步该做什么 */
    description?: string;
    /** 整页/整区空态用虚线容器；面板内保持无框 */
    framed?: boolean;
  }>(),
  { icon: undefined, description: undefined, framed: false }
);
</script>

<template>
  <div
    :class="
      cn(
        'flex flex-col items-center justify-center gap-2 text-center',
        framed ? 'rounded-lg border border-dashed py-10' : 'py-8'
      )
    "
  >
    <component :is="icon" v-if="icon" class="size-5 text-muted-foreground" />
    <p class="text-sm font-medium">{{ title }}</p>
    <p v-if="description" class="max-w-prose text-xs text-muted-foreground">{{ description }}</p>
    <div v-if="$slots.default" class="mt-1 flex items-center gap-2">
      <slot />
    </div>
  </div>
</template>
