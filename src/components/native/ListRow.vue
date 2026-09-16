<!--
  列表行（DESIGN.md §2.5 / appendix）：主文本 + 副文本 + 尾部控件/图标，可选选中态与键盘可达。
  与 `SettingsRow` 的区别：`SettingsRow` 用于「设置项（左标题 + 右控件）」，
  本组件用于**数据列表**（条目本身即内容，可点击/可选中）。

    <ListRow title="我的收藏" description="12 项" :selected="id === activeId" @select="…" />
-->
<script setup lang="ts">
import type { Component } from 'vue';
import { cn } from '@/lib/utils';

const props = withDefaults(
  defineProps<{
    title: string;
    description?: string;
    icon?: Component;
    /** 选中态（用于列表选择，不用指示线/边框表达） */
    selected?: boolean;
    /** 提供时该行可聚焦并响应 Enter/Space */
    interactive?: boolean;
    disabled?: boolean;
    /** 追加到整行的类（列表内可覆盖内边距，如 px-4 py-2.5） */
    class?: string;
  }>(),
  {
    description: undefined,
    icon: undefined,
    selected: false,
    interactive: false,
    disabled: false,
    class: undefined,
  }
);

const emit = defineEmits<{ select: [] }>();

function activate(): void {
  if (props.interactive && !props.disabled) emit('select');
}
</script>

<template>
  <div
    :class="
      cn(
        'flex min-w-0 items-center gap-2.5 rounded-md px-2 py-1.5 text-sm transition-colors',
        interactive && !disabled && 'cursor-pointer hover:bg-accent',
        selected && 'bg-primary/10 font-medium text-foreground [&>svg]:text-primary',
        disabled && 'pointer-events-none opacity-50',
        interactive &&
          'focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60 outline-none',
        props.class
      )
    "
    :tabindex="interactive && !disabled ? 0 : undefined"
    :role="interactive ? 'option' : undefined"
    :aria-selected="interactive ? selected : undefined"
    :aria-disabled="disabled || undefined"
    @click="activate"
    @keydown.enter.prevent="activate"
    @keydown.space.prevent="activate"
  >
    <slot name="leading" />
    <component :is="icon" v-if="icon" class="size-4 shrink-0 text-muted-foreground" />
    <div class="min-w-0 flex-1">
      <p class="truncate">{{ title }}</p>
      <p v-if="description" class="truncate text-xs text-muted-foreground">{{ description }}</p>
    </div>
    <div v-if="$slots.trailing" class="flex shrink-0 items-center gap-1">
      <slot name="trailing" />
    </div>
  </div>
</template>
