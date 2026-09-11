<!--
  工具页标准壳：页头（标题 / 说明 / 右侧动作区）+ 全幅滚动内容区。

  页面布局不引入任何自定义概念：需要居中列 / 分栏时在页面内直接使用
  Tailwind 栅格（grid grid-cols-12 + col-start-* / col-span-*），例如：

    <div class="mx-auto grid w-full grid-cols-12">
      <form class="col-span-12 md:col-start-4 md:col-span-6">…</form>
    </div>

  满幅页面（如数据表格）无需任何布局类。禁止 mx-auto max-w-* 居中容器。
-->
<script setup lang="ts">
withDefaults(
  defineProps<{
    /** 页头标题（省略时页头整行隐藏，只剩动作区可改用自身布局） */
    title?: string;
    /** 标题下方的一行说明 */
    description?: string;
  }>(),
  { title: undefined, description: undefined }
);
</script>

<template>
  <div class="flex min-h-full flex-col">
    <header
      v-if="title || $slots.actions"
      class="sticky top-0 z-10 flex shrink-0 items-center justify-between gap-4 border-b bg-background/90 px-5 py-3 backdrop-blur-sm"
    >
      <div class="min-w-0">
        <h2 v-if="title" class="truncate text-base font-semibold tracking-tight">
          {{ title }}
        </h2>
        <p v-if="description" class="truncate text-xs text-muted-foreground">
          {{ description }}
        </p>
      </div>
      <div v-if="$slots.actions" class="flex shrink-0 items-center gap-2">
        <slot name="actions" />
      </div>
    </header>

    <div class="min-w-0 flex-1 p-5">
      <slot />
    </div>
  </div>
</template>
