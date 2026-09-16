<!-- 流量列表 -->
<script setup lang="ts">
import { Activity } from '@lucide/vue';
import { Badge } from '@/components/ui/badge';
import EmptyState from '@/components/native/EmptyState.vue';
import { formatBytes, httpMethodClass, statusBadgeVariant, urlPath } from '../../shared';
import { classifyFlow, formatClock, KIND_LABELS, type FlowSummary } from '../../intercept-shared';

defineProps<{ flows: FlowSummary[]; selectedId: number | null }>();
const emit = defineEmits<{ select: [id: number] }>();
</script>

<template>
  <div class="flex h-full flex-col divide-y overflow-auto">
    <button
      v-for="flow in flows"
      :key="flow.id"
      type="button"
      class="flex w-full items-center gap-2 px-3 py-1.5 text-left transition-colors hover:bg-accent focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60 focus-visible:ring-inset outline-none"
      :class="flow.id === selectedId ? 'bg-primary/10' : ''"
      :aria-current="flow.id === selectedId ? 'true' : undefined"
      @click="emit('select', flow.id)"
    >
      <span class="w-16 shrink-0 text-xs text-muted-foreground">{{
        formatClock(flow.startedAt)
      }}</span>
      <span class="w-16 shrink-0 text-xs font-semibold" :class="httpMethodClass(flow.method)">
        {{ flow.method }}
      </span>
      <Badge :variant="statusBadgeVariant(flow.status)" class="shrink-0">
        {{ flow.error ? 'ERR' : (flow.status ?? '…') }}
      </Badge>
      <span class="w-12 shrink-0 text-center text-xs text-muted-foreground">
        {{ KIND_LABELS[classifyFlow(flow)] }}
      </span>
      <span class="min-w-0 flex-1">
        <span class="block truncate text-xs">
          <span class="text-muted-foreground">{{ flow.host }}</span>
          <span class="font-mono">{{ urlPath(flow.url) }}</span>
        </span>
      </span>
      <span class="shrink-0 font-mono text-xs text-muted-foreground">
        {{ flow.durationMs === null ? '—' : `${flow.durationMs}ms` }}
      </span>
      <span class="w-16 shrink-0 text-right font-mono text-xs text-muted-foreground">
        {{ formatBytes(flow.resSize || flow.reqSize) }}
      </span>
    </button>
    <EmptyState
      v-if="flows.length === 0"
      class="my-auto"
      :icon="Activity"
      title="暂无流量"
      description="启动代理后，经过本地代理的请求会显示在这里"
    />
  </div>
</template>
