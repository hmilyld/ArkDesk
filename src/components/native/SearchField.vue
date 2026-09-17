<!--
  搜索框（docs/design.md §3）：放大镜在左、输入即搜、有内容时显示清除按钮。
  事件与原生 input 一致（v-model + $attrs 透传，如 @keydown、ref），可直接替换普通 Input 用于过滤/查找。

    <SearchField v-model="query" placeholder="筛选工具" @keydown.down.prevent="…" />
-->
<script setup lang="ts">
import { Search, X } from '@lucide/vue';
import { cn } from '@/lib/utils';

const props = withDefaults(
  defineProps<{
    placeholder?: string;
    disabled?: boolean;
    class?: string;
  }>(),
  { placeholder: '搜索', disabled: false, class: undefined }
);

const model = defineModel<string>({ default: '' });

defineOptions({ inheritAttrs: false });
</script>

<template>
  <div
    :class="
      cn(
        'flex h-8 items-center gap-2 rounded-md border bg-transparent px-2.5 transition-colors',
        'focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/60',
        props.class
      )
    "
  >
    <Search class="size-3.5 shrink-0 text-muted-foreground" />
    <input
      v-model="model"
      :placeholder="placeholder"
      :disabled="disabled"
      type="search"
      class="min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed"
      v-bind="$attrs"
    />
    <button
      v-if="model"
      type="button"
      class="shrink-0 rounded-sm p-0.5 text-muted-foreground transition-colors hover:text-foreground"
      aria-label="清除"
      @click="model = ''"
    >
      <X class="size-3.5" />
    </button>
  </div>
</template>
