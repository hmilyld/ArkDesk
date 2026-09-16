<!--
  工具页模块面板：统一「外框 + 头部条（标题 / 右上角动作）+ 正文」的视觉语言。

  工具页里每个功能模块（设置 / 输入 / 输出 / 结果 / 列表）都应当用它包裹，
  不要在页面里裸露模块，也不要在 Panel 内再嵌套卡片（预览、表格等组件自身不带外框）。

    <Panel title="输入" hint="JSON">
      <template #actions>
        <Button variant="secondary" size="sm">选择文件</Button>
      </template>
      …正文…
    </Panel>

  约定：
  - 头部标题 `text-xs font-medium text-muted-foreground`（与 `crypto/ResultBox` 等一致）；
  - 动作按钮 `size="sm"`、图标 `size-3.5`；
  - 正文默认 `space-y-3 p-4`，需要满幅（表格滚动区 / 控制台）时传 `body-class="p-0"` 等覆盖，
    类名经 `cn`（tailwind-merge）合并，可安全覆盖默认值；
  - `title` 插槽可替换标题内容（此时忽略 `title` prop），`header-class` 可微调头部样式。
-->
<script setup lang="ts">
import { cn } from '@/lib/utils';

withDefaults(
  defineProps<{
    /** 头部标题（省略且无动作时头部整行隐藏） */
    title?: string;
    /** 标题后的补充信息，如格式名或行列统计 */
    hint?: string;
    /** 正文附加类名（覆盖默认的 `space-y-3 p-4`） */
    bodyClass?: string;
    /** 头部附加类名 */
    headerClass?: string;
  }>(),
  { title: undefined, hint: undefined, bodyClass: undefined, headerClass: undefined }
);
</script>

<template>
  <div class="overflow-hidden rounded-lg border bg-card">
    <div
      v-if="title || hint || $slots.title || $slots.actions"
      :class="cn('flex items-center justify-between gap-2 border-b px-4 py-2', headerClass)"
    >
      <span class="min-w-0 truncate text-xs font-medium text-muted-foreground">
        <slot name="title">{{ title }}</slot>
        <span v-if="hint" class="ml-1 font-normal">· {{ hint }}</span>
      </span>
      <div v-if="$slots.actions" class="flex shrink-0 items-center gap-1">
        <slot name="actions" />
      </div>
    </div>

    <div :class="cn('space-y-3 p-4', bodyClass)">
      <slot />
    </div>
  </div>
</template>
