<!--
  数据表格：动态列渲染 + 分页 + 单元格内联编辑（双击）+ 删行。
  内联编辑为 Input + NULL 勾选的原位替换，Enter 提交 / Esc 取消；
  BLOB 列只读展示（Rust 通道以 base64 返回）。
-->
<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { Check, Eye, KeyRound, Pencil, Trash2, X } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { isBlobColumn, PAGE_SIZE, type ColumnInfo, type DataRow } from '../shared';

const props = defineProps<{
  columns: ColumnInfo[];
  rows: DataRow[];
  loading: boolean;
  readonly: boolean;
  page: number;
  pageCount: number;
  total: number;
}>();

const emit = defineEmits<{
  'edit-cell': [payload: { row: DataRow; column: ColumnInfo; raw: string; isNull: boolean }];
  'view-row': [row: DataRow];
  'edit-row': [row: DataRow];
  'delete-row': [row: DataRow];
  'page-change': [page: number];
}>();

// ── 编辑态（editingKey = `${__rid}:${列名}`，标识符白名单不含冒号，可安全拆分） ──
const editingKey = ref<string | null>(null);
const editRaw = ref('');
const editNull = ref(false);
const editInput = ref<{ $el: HTMLInputElement } | { $el: HTMLInputElement }[] | null>(null);

function cellText(row: DataRow, column: ColumnInfo): string {
  const value = row[column.name];
  if (value === null || value === undefined) return '';
  return typeof value === 'string' ? value : String(value);
}

function isNullCell(row: DataRow, column: ColumnInfo): boolean {
  return row[column.name] === null || row[column.name] === undefined;
}

function editable(row: DataRow, column: ColumnInfo): boolean {
  return !props.readonly && !isBlobColumn(column) && row.__rid !== undefined;
}

async function startEdit(row: DataRow, column: ColumnInfo): Promise<void> {
  if (!editable(row, column)) return;
  editingKey.value = `${String(row.__rid)}:${column.name}`;
  editRaw.value = cellText(row, column);
  editNull.value = isNullCell(row, column);
  await nextTick();
  const input = Array.isArray(editInput.value) ? editInput.value[0] : editInput.value;
  input?.$el?.focus();
  input?.$el?.select();
}

function cancelEdit(): void {
  editingKey.value = null;
  editRaw.value = '';
  editNull.value = false;
}

function commitEdit(): void {
  const key = editingKey.value;
  if (!key) return;
  const colonIndex = key.indexOf(':');
  const rid = key.slice(0, colonIndex);
  const name = key.slice(colonIndex + 1);
  const row = props.rows.find((item) => String(item.__rid) === rid);
  const column = props.columns.find((item) => item.name === name);
  const raw = editRaw.value;
  const isNull = editNull.value;
  cancelEdit();
  if (!row || !column) return;
  const original = row[column.name];
  if (isNull ? original === null : String(original ?? '') === raw) return;
  emit('edit-cell', { row, column, raw, isNull });
}

// ── 分页（页数多时只显示当前页前后各 2 页 + 首尾） ──────────
const pageNumbers = computed(() =>
  Array.from({ length: props.pageCount }, (_, i) => i + 1).filter(
    (n) => props.pageCount <= 5 || n === 1 || n === props.pageCount || Math.abs(n - props.page) <= 2
  )
);
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <p v-if="loading" class="flex flex-1 items-center justify-center text-xs text-muted-foreground">
      加载中…
    </p>

    <div
      v-else-if="rows.length === 0"
      class="grid flex-1 place-content-center rounded-lg border border-dashed"
    >
      <p class="text-sm text-muted-foreground">该表暂无数据</p>
    </div>

    <template v-else>
      <div class="min-h-0 flex-1 overflow-y-auto rounded-lg border bg-card">
        <Table class="table-fixed">
          <TableHeader>
            <TableRow class="hover:bg-transparent">
              <TableHead v-for="column in columns" :key="column.name">
                <span class="flex items-baseline gap-1.5">
                  <span class="truncate font-mono text-xs">{{ column.name }}</span>
                  <KeyRound
                    v-if="column.pk > 0"
                    class="size-3 shrink-0 self-center text-muted-foreground"
                  />
                  <span class="shrink-0 text-[10px] font-normal text-muted-foreground">
                    {{ column.type || 'ANY' }}
                  </span>
                </span>
              </TableHead>
              <TableHead class="w-24 pr-3 text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow
              v-for="(row, rowIndex) in rows"
              :key="String(row.__rid ?? rowIndex)"
              class="group hover:bg-accent/40"
            >
              <TableCell
                v-for="column in columns"
                :key="column.name"
                class="px-3 py-2"
                :class="{ 'cursor-text': editable(row, column) }"
                :title="cellText(row, column)"
                @dblclick="startEdit(row, column)"
              >
                <!-- 内联编辑态 -->
                <div
                  v-if="editingKey === `${String(row.__rid)}:${column.name}`"
                  class="flex items-center gap-2"
                  @dblclick.stop
                >
                  <Input
                    ref="editInput"
                    v-model="editRaw"
                    class="h-7 bg-background font-mono text-xs"
                    :disabled="editNull"
                    @keydown.enter.prevent="commitEdit"
                    @keydown.esc.prevent="cancelEdit"
                  />
                  <Label class="flex shrink-0 items-center gap-1 text-[10px] text-muted-foreground">
                    <Checkbox
                      :model-value="editNull"
                      @update:model-value="editNull = $event === true"
                    />
                    NULL
                  </Label>
                  <Button variant="ghost" size="icon-sm" aria-label="保存" @click="commitEdit">
                    <Check class="size-3.5 text-primary" />
                  </Button>
                  <Button variant="ghost" size="icon-sm" aria-label="取消" @click="cancelEdit">
                    <X class="size-3.5 text-muted-foreground" />
                  </Button>
                </div>

                <!-- 展示态 -->
                <template v-else>
                  <span
                    v-if="isNullCell(row, column)"
                    class="font-mono text-xs italic text-muted-foreground/60"
                  >
                    NULL
                  </span>
                  <span v-else-if="isBlobColumn(column)" class="flex items-center gap-1.5">
                    <Badge variant="outline" class="shrink-0 text-[10px] text-muted-foreground">
                      BLOB
                    </Badge>
                    <span class="truncate font-mono text-xs text-muted-foreground">
                      {{ cellText(row, column).slice(0, 24) }}
                    </span>
                  </span>
                  <span
                    v-else
                    class="block truncate font-mono text-xs"
                    :class="
                      typeof row[column.name] === 'number'
                        ? 'text-foreground'
                        : 'text-foreground/90'
                    "
                  >
                    {{ cellText(row, column) }}
                  </span>
                </template>
              </TableCell>

              <TableCell class="w-24 pr-3 text-right">
                <div
                  class="flex items-center justify-end gap-0.5 opacity-0 transition-opacity group-hover:opacity-100"
                >
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="查看行"
                    @click="emit('view-row', row)"
                  >
                    <Eye class="size-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="编辑行"
                    :disabled="readonly"
                    @click="emit('edit-row', row)"
                  >
                    <Pencil class="size-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="删除行"
                    :disabled="readonly"
                    @click="emit('delete-row', row)"
                  >
                    <Trash2 class="size-4 text-destructive" />
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </div>

      <div class="mt-3 flex shrink-0 items-center justify-center gap-1.5">
        <Button
          variant="outline"
          size="sm"
          :disabled="page === 1"
          @click="emit('page-change', page - 1)"
        >
          上一页
        </Button>
        <template v-for="(n, index) in pageNumbers" :key="n">
          <span
            v-if="index > 0 && n - pageNumbers[index - 1]! > 1"
            class="px-1 text-muted-foreground"
          >
            …
          </span>
          <Button
            variant="outline"
            size="sm"
            class="min-w-8 px-2 font-mono"
            :class="
              n === page ? 'border-primary/50 bg-primary/10 text-primary' : 'text-muted-foreground'
            "
            @click="emit('page-change', n)"
          >
            {{ n }}
          </Button>
        </template>
        <Button
          variant="outline"
          size="sm"
          :disabled="page === pageCount"
          @click="emit('page-change', page + 1)"
        >
          下一页
        </Button>
        <span class="ml-2 text-xs text-muted-foreground">
          共 {{ total }} 行 · 每页 {{ PAGE_SIZE }} 行
        </span>
      </div>
    </template>
  </div>
</template>
