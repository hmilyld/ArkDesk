<script setup lang="ts">
/**
 * 工具级错误边界：单个工具渲染崩溃不影响框架与导航。
 * 捕获后记录日志并显示占位页（return false 阻断全局重复处理）。
 */
import { ref, onErrorCaptured } from 'vue';
import { Button } from '@/components/ui/button';
import { logger } from '@/core/logger';
import { normalizeError } from '@/core/errors';

const crashed = ref(false);
const message = ref('');

onErrorCaptured((err) => {
  crashed.value = true;
  message.value = normalizeError(err).message;
  logger.error(`工具渲染异常: ${message.value}`);
  return false;
});

function reset(): void {
  crashed.value = false;
  message.value = '';
}
</script>

<template>
  <div v-if="!crashed" class="h-full">
    <slot />
  </div>
  <div v-else class="flex h-full flex-col items-center justify-center gap-3 p-8">
    <p class="text-base text-muted-foreground">此工具出现异常，不影响其他功能</p>
    <p class="max-w-md truncate font-mono text-xs text-muted-foreground/70">
      {{ message }}
    </p>
    <Button variant="outline" size="sm" @click="reset">重试</Button>
  </div>
</template>
