<!--
  分段控件（DESIGN.md §3）：2–5 项互斥、需常显选中态的选项。
  ≥6 项或文字较长时改用下拉（PopUp）；窗口级视图切换用 tab 条，不用分段。
  键盘语义（§2.9）：←/→ 移动、Home/End 首尾（由 reka RadioGroup 提供）。
-->
<script setup lang="ts">
import type { Component } from 'vue';
import { RadioGroupItem, RadioGroupRoot } from 'reka-ui';
import { cn } from '@/lib/utils';

interface Segment {
  value: string;
  label: string;
  icon?: Component;
}

defineProps<{
  segments: Segment[];
  /** 紧凑档（工具栏/行内）：h-7；默认 h-8 */
  size?: 'sm' | 'default';
  disabled?: boolean;
}>();

const model = defineModel<string>({ required: true });
</script>

<template>
  <RadioGroupRoot
    v-model="model"
    :disabled="disabled"
    class="inline-flex shrink-0 items-center gap-0.5 rounded-md bg-muted p-0.5"
  >
    <RadioGroupItem
      v-for="segment in segments"
      :key="segment.value"
      :value="segment.value"
      :disabled="disabled"
      :class="
        cn(
          'inline-flex items-center gap-1.5 rounded-[5px] px-2.5 text-sm transition-colors outline-none',
          size === 'sm' ? 'h-6' : 'h-7',
          'focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60',
          model === segment.value
            ? 'bg-primary text-primary-foreground'
            : 'text-muted-foreground hover:text-foreground'
        )
      "
    >
      <component :is="segment.icon" v-if="segment.icon" class="size-3.5" />
      {{ segment.label }}
    </RadioGroupItem>
  </RadioGroupRoot>
</template>
