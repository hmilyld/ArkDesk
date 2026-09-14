<!--
  设置行：左「标题 + 说明」、右控件，视觉与系统设置页一致。
  控件宽度由调用方通过默认插槽里的 class 控制（w-40 / w-64 / w-72 等）。

    <SettingsRow title="开机自启" description="系统启动时自动运行本应用">
      <Switch :model-value="…" />
    </SettingsRow>

  需要自定义左侧内容时用 #label 插槽覆盖默认标题/说明。
-->
<script setup lang="ts">
import type { HTMLAttributes } from 'vue';
import { cn } from '@/lib/utils';

const props = defineProps<{
  /** 行标题（左侧主文本） */
  title?: string;
  /** 标题下方说明文字 */
  description?: string;
  /** 追加到整行的类 */
  class?: HTMLAttributes['class'];
}>();
</script>

<template>
  <div class="flex items-center justify-between gap-3 px-3 py-2.5" :class="cn(props.class)">
    <div class="min-w-0">
      <slot name="label">
        <p v-if="title" class="text-sm">{{ title }}</p>
        <p v-if="description" class="text-xs text-muted-foreground">{{ description }}</p>
      </slot>
    </div>
    <div class="flex shrink-0 items-center gap-2">
      <slot />
    </div>
  </div>
</template>
