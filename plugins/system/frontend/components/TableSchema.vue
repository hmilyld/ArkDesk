<!-- 表结构：pragma_table_info 的列清单（名称/类型/约束/默认值/主键位） -->
<script setup lang="ts">
import { KeyRound } from '@lucide/vue';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import type { TableSchemaInfo } from '../shared';

defineProps<{
  schema: TableSchemaInfo | null;
  loading: boolean;
}>();
</script>

<template>
  <p v-if="loading" class="py-16 text-center text-xs text-muted-foreground">加载中…</p>

  <div
    v-else-if="!schema || schema.columns.length === 0"
    class="rounded-lg border border-dashed py-16 text-center"
  >
    <p class="text-sm text-muted-foreground">未获取到表结构</p>
  </div>

  <div v-else class="overflow-hidden rounded-lg border bg-card">
    <Table>
      <TableHeader>
        <TableRow class="hover:bg-transparent">
          <TableHead class="w-12 pl-3">#</TableHead>
          <TableHead>列名</TableHead>
          <TableHead class="w-28">类型</TableHead>
          <TableHead class="w-20">NOT NULL</TableHead>
          <TableHead class="w-20">主键</TableHead>
          <TableHead class="pr-3">默认值</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow v-for="column in schema.columns" :key="column.name" class="hover:bg-accent/40">
          <TableCell class="pl-3 font-mono text-xs text-muted-foreground">
            {{ column.cid }}
          </TableCell>
          <TableCell class="font-mono text-xs">
            <span class="flex items-center gap-1.5">
              {{ column.name }}
              <KeyRound v-if="column.pk > 0" class="size-3 text-muted-foreground" />
            </span>
          </TableCell>
          <TableCell class="font-mono text-xs text-muted-foreground">
            {{ column.type || 'ANY' }}
          </TableCell>
          <TableCell>
            <Badge v-if="column.notnull === 1" variant="secondary">是</Badge>
            <span v-else class="text-xs text-muted-foreground">—</span>
          </TableCell>
          <TableCell class="font-mono text-xs text-muted-foreground">
            {{ column.pk > 0 ? column.pk : '—' }}
          </TableCell>
          <TableCell class="pr-3 font-mono text-xs text-muted-foreground">
            {{ column.dfltValue ?? '—' }}
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>
  </div>
</template>
