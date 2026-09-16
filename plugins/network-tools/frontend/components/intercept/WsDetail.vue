<!-- WebSocket 连接详情（只读帧列表） -->
<script setup lang="ts">
import { formatClock, type WsRecord } from '../../intercept-shared';

defineProps<{ record: WsRecord | null }>();
</script>

<template>
  <div v-if="!record" class="flex h-full items-center justify-center text-xs text-muted-foreground">
    选择左侧一个 WebSocket 连接查看帧
  </div>
  <div v-else class="flex h-full min-h-0 flex-col gap-2">
    <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
      <span class="font-mono text-xs">{{ record.summary.url }}</span>
      <span class="text-xs text-muted-foreground">{{ record.summary.frameCount }} 帧</span>
      <span v-if="record.summary.truncated" class="text-xs text-warning">（仅保留部分帧）</span>
    </div>
    <div class="min-h-0 flex-1 overflow-auto rounded-lg border border-border">
      <div
        v-for="(frame, index) in record.frames"
        :key="index"
        class="flex items-start gap-3 border-b border-border/50 px-3 py-1.5 text-xs"
      >
        <span class="w-10 shrink-0 text-muted-foreground">{{ formatClock(frame.at) }}</span>
        <span
          class="w-8 shrink-0 font-semibold"
          :class="frame.dir === 'up' ? 'text-info' : 'text-success'"
        >
          {{ frame.dir === 'up' ? '↑' : '↓' }}
        </span>
        <span class="w-14 shrink-0 text-muted-foreground">{{ frame.opcode }}</span>
        <span class="w-14 shrink-0 text-right font-mono text-muted-foreground"
          >{{ frame.len }}B</span
        >
        <span class="min-w-0 flex-1 break-all font-mono">{{ frame.text ?? '' }}</span>
      </div>
      <p
        v-if="record.frames.length === 0"
        class="px-3 py-6 text-center text-xs text-muted-foreground"
      >
        暂无帧
      </p>
    </div>
  </div>
</template>
