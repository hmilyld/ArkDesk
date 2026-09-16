<!--
  表结构：pragma_table_info 的列清单（名称 / 类型 / 约束 / 默认值 / 主键位）。
  自绘表格（不套 ui/Table 的横向滚动壳）：表头粘性需要滚动容器同时承担纵向滚动，
  直接由外层 TabsContent 充当滚动口。数字列右对齐 + tabular-nums。
-->
<script setup lang="ts">
import { KeyRound, Table2 } from '@lucide/vue';
import EmptyState from '@/components/native/EmptyState.vue';
import LoadingState from '@/components/native/LoadingState.vue';
import type { TableSchemaInfo } from '../shared';

defineProps<{
  schema: TableSchemaInfo | null;
  loading: boolean;
}>();
</script>

<template>
  <LoadingState v-if="loading" variant="spinner" label="读取表结构…" />
  <EmptyState
    v-else-if="!schema || schema.columns.length === 0"
    :icon="Table2"
    title="未获取到表结构"
    description="重新选择该表，或在页头点「刷新」后重试"
  />
  <table v-else class="w-full table-fixed caption-bottom text-base">
    <thead>
      <tr>
        <th
          class="sticky top-0 z-10 h-9 w-12 border-b bg-card pl-3 pr-2 text-right align-middle text-xs font-medium text-muted-foreground"
        >
          #
        </th>
        <th
          class="sticky top-0 z-10 h-9 border-b bg-card px-2 text-left align-middle text-xs font-medium text-muted-foreground"
        >
          表头
        </th>
        <th
          class="sticky top-0 z-10 h-9 w-28 border-b bg-card px-2 text-left align-middle text-xs font-medium text-muted-foreground"
        >
          类型
        </th>
        <th
          class="sticky top-0 z-10 h-9 w-24 border-b bg-card px-2 text-left align-middle text-xs font-medium text-muted-foreground"
        >
          NOT NULL
        </th>
        <th
          class="sticky top-0 z-10 h-9 w-16 border-b bg-card px-2 text-right align-middle text-xs font-medium text-muted-foreground"
        >
          主键
        </th>
        <th
          class="sticky top-0 z-10 h-9 border-b bg-card px-2 pr-3 text-left align-middle text-xs font-medium text-muted-foreground"
        >
          默认值
        </th>
      </tr>
    </thead>
    <tbody class="divide-y divide-border/60">
      <tr v-for="column in schema.columns" :key="column.name" class="hover:bg-accent">
        <td
          class="py-2 pl-3 pr-2 text-right align-middle font-mono text-xs tabular-nums text-muted-foreground"
        >
          {{ column.cid }}
        </td>
        <td class="px-2 py-2 align-middle">
          <span class="flex items-center gap-1.5 font-mono text-xs">
            <span class="truncate">{{ column.name }}</span>
            <KeyRound v-if="column.pk > 0" class="size-3.5 shrink-0 text-muted-foreground" />
          </span>
        </td>
        <td class="truncate px-2 py-2 align-middle font-mono text-xs text-muted-foreground">
          {{ column.type || 'ANY' }}
        </td>
        <td class="px-2 py-2 align-middle">
          <span v-if="column.notnull === 1" class="text-xs">是</span>
          <span v-else class="text-xs text-muted-foreground">—</span>
        </td>
        <td
          class="px-2 py-2 text-right align-middle font-mono text-xs tabular-nums text-muted-foreground"
        >
          {{ column.pk > 0 ? column.pk : '—' }}
        </td>
        <td class="truncate px-2 py-2 pr-3 align-middle font-mono text-xs text-muted-foreground">
          {{ column.dfltValue ?? '—' }}
        </td>
      </tr>
    </tbody>
  </table>
</template>
