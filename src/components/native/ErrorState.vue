<!--
  错误态（docs/design.md §4 三分法）：
  - 就地错误（表单/工具页内可定位）→ 用 `inline` 形态，或直接 `FormRow` 自带校验
  - 当前操作失败且有重试入口 → 用 `inline` + 重试按钮（默认）
  - 系统级/后台失败（无就地位置）→ 用 toast，不用本组件
-->
<script setup lang="ts">
import { CircleAlert } from '@lucide/vue';
import { Button } from '@/components/ui/button';

withDefaults(
  defineProps<{
    /** 错误信息（说明发生了什么 + 下一步怎么办，不道歉、不出现「出错了」） */
    message: string;
    /** 提供时显示重试按钮 */
    onRetry?: () => void;
    /** 单行紧凑形态（面板头部/行内） */
    compact?: boolean;
  }>(),
  { onRetry: undefined, compact: false }
);
</script>

<template>
  <div
    class="flex items-start justify-between gap-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive"
    role="alert"
  >
    <div class="flex min-w-0 items-start gap-1.5">
      <CircleAlert class="mt-0.5 size-3.5 shrink-0" />
      <span :class="compact ? 'line-clamp-2' : ''">{{ message }}</span>
    </div>
    <Button
      v-if="onRetry"
      variant="ghost"
      size="sm"
      class="h-6 shrink-0 px-2 text-destructive hover:bg-destructive/15 hover:text-destructive"
      @click="onRetry"
    >
      重试
    </Button>
  </div>
</template>
