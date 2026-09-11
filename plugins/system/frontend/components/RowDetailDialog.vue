<!--
  行查看/编辑弹窗：长内容场景替代行内编辑。
  控件按列类型选择：数值类（INT/REAL/…）用 Input，其余（TEXT/空类型）用 Textarea，
  BLOB 与主键（rowid 别名）列一律只读；NULL 勾选优先。
  编辑模式仅提交相对原值发生变更的列，UPDATE 由父组件执行。
-->
<script setup lang="ts">
import { computed, reactive, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  isBlobColumn,
  isRowidAlias,
  parseCellValue,
  type ColumnInfo,
  type DataRow,
  type InsertEntry,
} from '../shared';

const props = defineProps<{
  open: boolean;
  mode: 'view' | 'edit';
  table: string;
  columns: ColumnInfo[];
  row: DataRow | null;
}>();

const emit = defineEmits<{
  'update:open': [open: boolean];
  save: [entries: InsertEntry[]];
}>();

/** 列名 → { 输入内容, 是否 NULL } */
const fields = reactive<Record<string, { raw: string; isNull: boolean }>>({});

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    for (const key of Object.keys(fields)) delete fields[key];
    for (const column of props.columns) {
      const value = props.row?.[column.name];
      fields[column.name] = {
        raw: value === null || value === undefined ? '' : String(value),
        isNull: value === null || value === undefined,
      };
    }
  }
);

const NUMERIC_TYPE_RE = /INT|REAL|FLOA|DOUB|DEC|NUM|BOOL/i;

function isNumeric(column: ColumnInfo): boolean {
  return NUMERIC_TYPE_RE.test(column.type);
}

/** 控件选择：数值类 Input，其余（TEXT/空声明类型）Textarea */
function controlKind(column: ColumnInfo): 'input' | 'textarea' {
  return isNumeric(column) ? 'input' : 'textarea';
}

/** 主键（rowid 别名）与 BLOB 列不提供修改，查看模式全部只读 */
function locked(column: ColumnInfo): boolean {
  return props.mode === 'view' || isRowidAlias(column) || isBlobColumn(column);
}

function disabled(column: ColumnInfo): boolean {
  return locked(column) || (fields[column.name]?.isNull ?? false);
}

/** 编辑提交：仅收集相对原值发生变化的列 */
const changedEntries = computed<InsertEntry[]>(() => {
  if (props.mode !== 'edit') return [];
  const result: InsertEntry[] = [];
  for (const column of props.columns) {
    if (locked(column)) continue;
    const field = fields[column.name];
    if (!field) continue;
    const original = props.row?.[column.name];
    const originalIsNull = original === null || original === undefined;
    const changed =
      field.isNull !== originalIsNull ||
      (!field.isNull && !originalIsNull && field.raw !== String(original));
    if (changed) {
      result.push({
        column: column.name,
        value: parseCellValue(field.raw, field.isNull, column),
      });
    }
  }
  return result;
});
</script>

<template>
  <Dialog :open="open" @update:open="(value) => emit('update:open', value)">
    <DialogContent class="sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>
          {{ mode === 'view' ? '查看行' : '编辑行' }}
          <span class="ml-1 font-mono text-sm font-normal text-muted-foreground">
            #{{ row?.__rid }}
          </span>
        </DialogTitle>
        <DialogDescription>
          表「{{ table }}」{{ mode === 'view' ? '的完整行内容' : '仅提交发生变更的列' }}
        </DialogDescription>
      </DialogHeader>

      <div class="max-h-[28rem] space-y-4 overflow-y-auto py-2 pr-1">
        <div v-for="column in columns" :key="column.name" class="space-y-1">
          <div class="flex items-center justify-between gap-2">
            <Label :for="`rowdlg-${column.name}`" class="font-mono text-xs">
              {{ column.name }}
              <span class="ml-1 font-sans text-[10px] font-normal text-muted-foreground">
                {{ column.type || 'ANY' }}
                {{ column.notnull === 1 ? '· NOT NULL' : '' }}
                {{ column.pk > 0 ? '· PK' : '' }}
              </span>
            </Label>
            <Label
              v-if="mode === 'edit' && !isRowidAlias(column) && !isBlobColumn(column)"
              class="flex items-center gap-1 text-[10px] text-muted-foreground"
            >
              <Checkbox
                :model-value="fields[column.name]?.isNull ?? false"
                @update:model-value="
                  fields[column.name] && (fields[column.name]!.isNull = $event === true)
                "
              />
              NULL
            </Label>
            <Badge
              v-else-if="fields[column.name]?.isNull"
              variant="outline"
              class="text-[10px] text-muted-foreground"
            >
              NULL
            </Badge>
            <Badge
              v-else-if="isBlobColumn(column)"
              variant="outline"
              class="text-[10px] text-muted-foreground"
            >
              BLOB
            </Badge>
          </div>

          <Textarea
            v-if="controlKind(column) === 'textarea'"
            :id="`rowdlg-${column.name}`"
            v-model="fields[column.name]!.raw"
            rows="3"
            class="font-mono text-xs"
            :disabled="disabled(column)"
          />
          <Input
            v-else
            :id="`rowdlg-${column.name}`"
            v-model="fields[column.name]!.raw"
            class="h-8 font-mono text-xs"
            :disabled="disabled(column)"
          />
        </div>
      </div>

      <DialogFooter>
        <Button variant="outline" size="sm" @click="emit('update:open', false)">
          {{ mode === 'view' ? '关闭' : '取消' }}
        </Button>
        <Button
          v-if="mode === 'edit'"
          size="sm"
          :disabled="changedEntries.length === 0"
          @click="emit('save', changedEntries)"
        >
          保存（{{ changedEntries.length }} 列变更）
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
