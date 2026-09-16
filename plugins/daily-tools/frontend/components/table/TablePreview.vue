<!--
  表格预览：以网格形式展示中间表示（列 + 单元格）。

  不带外框：外框由调用方的 Panel（`rounded-lg border bg-card` + 头部条）提供，避免卡片套卡片。
-->
<script setup lang="ts">
import { computed } from 'vue';
import { formatCellText } from '../../table/cell-text';
import type { Cell, Table } from '../../table/model';
import type { TextFormatOptions } from '../../table/options';

const props = withDefaults(
  defineProps<{
    table: Table;
    options: TextFormatOptions;
    /** 最多展示行数（0 表示不限制） */
    limit?: number;
  }>(),
  { limit: 0 }
);

const rows = computed(() =>
  props.limit > 0 ? props.table.rows.slice(0, props.limit) : props.table.rows
);

function textOf(cell: Cell): string {
  return formatCellText(cell, props.options);
}
</script>

<template>
  <div class="max-h-[460px] overflow-auto">
    <table class="w-full border-collapse text-sm">
      <thead class="sticky top-0 bg-muted/80 backdrop-blur-sm">
        <tr>
          <th
            v-for="(column, index) in table.columns"
            :key="`${column}-${index}`"
            class="border-b px-3 py-2 text-left font-medium whitespace-nowrap"
          >
            {{ column }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(row, rowIndex) in rows" :key="rowIndex" class="even:bg-muted/30">
          <td
            v-for="(cell, cellIndex) in row"
            :key="cellIndex"
            class="border-b px-3 py-1.5 align-top"
            :class="cell.t === 'n' || cell.t === 'b' ? 'text-right tabular-nums' : ''"
          >
            <span class="line-clamp-3 break-all">{{ textOf(cell) }}</span>
          </td>
        </tr>
      </tbody>
    </table>
    <p v-if="table.columns.length === 0" class="p-6 text-center text-sm text-muted-foreground">
      没有可预览的列
    </p>
  </div>
</template>
