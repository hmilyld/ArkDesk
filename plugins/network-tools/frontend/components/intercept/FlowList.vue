<!-- 流量列表 -->
<script setup lang="ts">
import { Badge } from '@/components/ui/badge';
import { formatBytes, httpMethodClass, statusBadgeVariant, urlPath } from '../../shared';
import { classifyFlow, formatClock, KIND_LABELS, type FlowSummary } from '../../intercept-shared';

defineProps<{ flows: FlowSummary[]; selectedId: number | null }>();
const emit = defineEmits<{ select: [id: number] }>();
</script>

<template>
  <div class="h-full overflow-auto">
    <button
      v-for="flow in flows"
      :key="flow.id"
      type="button"
      class="flex w-full items-center gap-2 border-b border-border/50 px-2 py-1.5 text-left hover:bg-muted/60"
      :class="flow.id === selectedId ? 'bg-muted' : ''"
      @click="emit('select', flow.id)"
    >
      <span class="w-10 shrink-0 text-[10px] text-muted-foreground">{{
        formatClock(flow.startedAt)
      }}</span>
      <span class="w-11 shrink-0 text-[10px] font-semibold" :class="httpMethodClass(flow.method)">
        {{ flow.method }}
      </span>
      <Badge :variant="statusBadgeVariant(flow.status)" class="shrink-0">
        {{ flow.error ? 'ERR' : (flow.status ?? '…') }}
      </Badge>
      <span class="w-9 shrink-0 text-center text-[10px] text-muted-foreground">
        {{ KIND_LABELS[classifyFlow(flow)] }}
      </span>
      <span class="min-w-0 flex-1">
        <span class="block truncate text-xs">
          <span class="text-muted-foreground">{{ flow.host }}</span>
          <span class="font-mono">{{ urlPath(flow.url) }}</span>
        </span>
      </span>
      <span class="shrink-0 font-mono text-[10px] text-muted-foreground">
        {{ flow.durationMs === null ? '—' : `${flow.durationMs}ms` }}
      </span>
      <span class="w-14 shrink-0 text-right font-mono text-[10px] text-muted-foreground">
        {{ formatBytes(flow.resSize || flow.reqSize) }}
      </span>
    </button>
    <p v-if="flows.length === 0" class="px-3 py-8 text-center text-xs text-muted-foreground">
      暂无流量
    </p>
  </div>
</template>
