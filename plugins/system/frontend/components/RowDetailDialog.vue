<!--
  行查看/编辑弹窗：长内容场景替代行内编辑。
  控件按列类型选择：数值类（INT/REAL/…）用 Input，其余（TEXT/空类型）用 Textarea，
  BLOB 与主键（rowid 别名）列一律只读；NULL 勾选优先。
  标签 / 说明 / 行内校验统一交给 FormRow；字段多时正文内部滚动，父级错误就地展示。
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
import ErrorState from '@/components/native/ErrorState.vue';
import FormRow from '@/components/native/FormRow.vue';
import {
  isBlobColumn,
  isRowidAlias,
  parseCellValue,
  type ColumnInfo,
  type DataRow,
  type InsertEntry,
} from '../shared';

const props = withDefaults(
  defineProps<{
    open: boolean;
    mode: 'view' | 'edit';
    table: string;
    columns: ColumnInfo[];
    row: DataRow | null;
    /** 父级保存失败信息（就地展示，不用 toast） */
    error?: string;
  }>(),
  { error: undefined }
);

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

function isNumeric(column: ColumnInfo): boolean {
  return /INT|REAL|FLOA|DOUB|DEC|NUM|BOOL/i.test(column.type);
}

/** 字段标签下方的类型 / 约束说明 */
function metaText(column: ColumnInfo): string {
  const parts = [column.type || 'ANY'];
  if (column.notnull === 1) parts.push('NOT NULL');
  if (column.pk > 0) parts.push('PK');
  return parts.join(' · ');
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
          <span class="ml-1 font-mono text-sm font-normal text-muted-foreground tabular-nums">
            #{{ row?.__rid }}
          </span>
        </DialogTitle>
        <DialogDescription>
          表「{{ table }}」{{ mode === 'view' ? '的完整行内容' : '仅提交发生变更的列' }}
        </DialogDescription>
      </DialogHeader>

      <ErrorState v-if="error" :message="error" compact />

      <div class="max-h-[55vh] space-y-3 overflow-y-auto px-1 py-1">
        <FormRow
          v-for="column in columns"
          :key="column.name"
          :label="column.name"
          :description="metaText(column)"
        >
          <template #label-action>
            <Label
              v-if="mode === 'edit' && !isRowidAlias(column) && !isBlobColumn(column)"
              class="flex items-center gap-1 text-xs text-muted-foreground"
            >
              <Checkbox
                :model-value="fields[column.name]?.isNull ?? false"
                aria-label="该列写入 NULL"
                @update:model-value="
                  fields[column.name] && (fields[column.name]!.isNull = $event === true)
                "
              />
              NULL
            </Label>
            <Badge
              v-else-if="fields[column.name]?.isNull"
              variant="outline"
              class="text-muted-foreground"
            >
              NULL
            </Badge>
            <Badge v-else-if="isBlobColumn(column)" variant="outline" class="text-muted-foreground">
              BLOB
            </Badge>
          </template>

          <template #default="{ id, invalid, describedBy }">
            <Textarea
              v-if="controlKind(column) === 'textarea'"
              :id="id"
              v-model="fields[column.name]!.raw"
              class="field-sizing-fixed h-20 resize-none font-mono text-xs"
              :aria-invalid="invalid"
              :aria-describedby="describedBy"
              :disabled="disabled(column)"
            />
            <Input
              v-else
              :id="id"
              v-model="fields[column.name]!.raw"
              class="h-8 text-right font-mono text-xs tabular-nums"
              :aria-invalid="invalid"
              :aria-describedby="describedBy"
              :disabled="disabled(column)"
            />
          </template>
        </FormRow>
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
          保存（<span class="tabular-nums">{{ changedEntries.length }}</span> 列变更）
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
