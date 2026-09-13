<!--
  文件任务进度条 + 取消按钮。
  读取 core/tasks 的任务状态（由后端 task:// 事件驱动），运行中显示百分比并可取消。
-->
<script setup lang="ts">
import { computed } from 'vue';
import { Button } from '@/components/ui/button';
import { X } from '@lucide/vue';
import { cancelTask, tasksState } from '@/core/tasks';

const props = defineProps<{ taskId: string }>();

const task = computed(() => tasksState[props.taskId] ?? { running: false, done: 0, total: null });
const percent = computed(() => {
  const { done, total } = task.value;
  return total && total > 0 ? Math.min(100, Math.round((done / total) * 100)) : null;
});

function stop(): void {
  void cancelTask(props.taskId);
}
</script>

<template>
  <div v-if="task.running" class="flex items-center gap-3">
    <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-muted">
      <div
        class="h-full rounded-full bg-primary transition-[width] duration-150"
        :style="{ width: percent === null ? '100%' : `${percent}%` }"
      />
    </div>
    <span class="shrink-0 text-xs text-muted-foreground">
      {{ percent === null ? '处理中…' : `${percent}%` }}
    </span>
    <Button variant="ghost" size="sm" @click="stop">
      <X class="mr-1 size-3.5" />
      取消
    </Button>
  </div>
</template>
