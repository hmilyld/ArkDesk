<!--
  设置字段：标题在上、控件在下的堆叠式字段，用于栅格排布的密集表单
  （如多列数值参数）。普通「左标题 / 右控件」设置项请用 SettingsRow。

    <div class="grid grid-cols-3 gap-3 p-3">
      <SettingsField label="正文最少字数">
        <Input v-model.number="…" type="number" />
      </SettingsField>
    </div>
-->
<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { cn } from '@/lib/utils';

const props = defineProps<{
  /** 字段标题 */
  label: string;
  /** 字段下方说明文字 */
  description?: string;
  /** 追加到外层容器的类 */
  class?: HTMLAttributes['class'];
}>();
</script>

<template>
  <div class="space-y-1.5" :class="cn(props.class)">
    <p class="text-sm">{{ label }}</p>
    <p v-if="description" class="text-xs text-muted-foreground">{{ description }}</p>
    <slot />
  </div>
</template>
