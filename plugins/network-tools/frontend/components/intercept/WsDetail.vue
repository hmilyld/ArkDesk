<!-- WebSocket 连接详情（只读帧列表） -->
<script setup lang="ts">
import { Cable, Inbox } from '@lucide/vue';
import EmptyState from '@/components/native/EmptyState.vue';
import { formatClock, type WsRecord } from '../../lib/intercept-shared';

defineProps<{ record: WsRecord | null }>();
</script>

<template>
  <EmptyState
    v-if="!record"
    class="h-full"
    :icon="Cable"
    title="未选择连接"
    description="从左侧选择一条 WebSocket 连接查看帧"
  />
  <div v-else class="flex h-full min-h-0 flex-col gap-2">
    <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
      <span class="font-mono text-xs">{{ record.summary.url }}</span>
      <span class="text-xs text-muted-foreground">{{ record.summary.frameCount }} 帧</span>
      <span v-if="record.summary.truncated" class="text-xs text-warning">（仅保留部分帧）</span>
    </div>
    <div
      v-if="record.frames.length"
      class="min-h-0 flex-1 divide-y overflow-auto rounded-md border"
    >
      <div
        v-for="(frame, index) in record.frames"
        :key="index"
        class="flex items-start gap-3 px-3 py-1.5 text-xs"
      >
        <span class="w-16 shrink-0 text-muted-foreground">{{ formatClock(frame.at) }}</span>
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
        <span class="min-w-0 flex-1 font-mono whitespace-pre-wrap break-words">{{
          frame.text ?? ''
        }}</span>
      </div>
    </div>
    <EmptyState
      v-else
      class="flex-1"
      :icon="Inbox"
      title="暂无帧"
      description="该连接尚未捕获到任何帧"
    />
  </div>
</template>
