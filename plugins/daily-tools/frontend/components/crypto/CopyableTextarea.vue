<!--
  可编辑 + 可复制的内容面板。
  用于「既是输入、又常作为生成结果」的内容（如密钥对）：
  视觉与 ResultBox 一致（面板 + 标题栏 + 右上角复制按钮），内容超出固定高度滚动。
-->
<script setup lang="ts">
import CopyButton from './CopyButton.vue';

const model = defineModel<string>({ required: true });

withDefaults(
  defineProps<{
    label: string;
    placeholder?: string;
  }>(),
  {
    placeholder: '',
  }
);
</script>

<template>
  <div class="overflow-hidden rounded-lg border bg-card">
    <div class="flex items-center justify-between gap-2 border-b px-4 py-2">
      <span class="text-xs font-medium text-muted-foreground">{{ label }}</span>
      <CopyButton :value="model" />
    </div>
    <textarea
      v-model="model"
      class="block h-40 w-full resize-y overflow-y-auto bg-transparent px-4 py-3 font-mono text-xs leading-relaxed outline-none placeholder:text-muted-foreground"
      :placeholder="placeholder"
    />
  </div>
</template>
