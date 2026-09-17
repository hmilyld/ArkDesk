<!--
  新增行对话框：按表结构动态生成表单。
  字段语义：勾选 NULL → 写入 NULL；留空 → 跳过该列（走表默认值）；
  填写 → 解析后写入（数值亲和列转数字）。INTEGER 主键留空自动赋值。
  标签 / 说明 / 行内校验统一交给 FormRow；字段多时正文内部滚动，父级错误就地展示。
-->
<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
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
import ErrorState from '@/components/native/ErrorState.vue';
import FormRow from '@/components/native/FormRow.vue';
import {
  assertColumnName,
  isRowidAlias,
  parseCellValue,
  type ColumnInfo,
  type InsertEntry,
} from '../shared';

const props = withDefaults(
  defineProps<{
    open: boolean;
    table: string;
    columns: ColumnInfo[];
    /** 父级写入失败信息（就地展示，不用 toast） */
    error?: string;
  }>(),
  { error: undefined }
);

const emit = defineEmits<{
  'update:open': [open: boolean];
  submit: [entries: InsertEntry[]];
}>();

/** 列名 → { 原始输入, 是否 NULL } */
const fields = reactive<Record<string, { raw: string; isNull: boolean }>>({});

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    for (const key of Object.keys(fields)) delete fields[key];
    for (const column of props.columns) {
      fields[column.name] = { raw: '', isNull: false };
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

const entries = computed<InsertEntry[]>(() => {
  const result: InsertEntry[] = [];
  for (const column of props.columns) {
    const field = fields[column.name];
    if (!field) continue;
    if (field.isNull) {
      result.push({ column: column.name, value: null });
      continue;
    }
    if (field.raw === '') continue;
    if (isRowidAlias(column)) continue;
    result.push({ column: column.name, value: parseCellValue(field.raw, false, column) });
  }
  return result;
});

const invalidColumns = computed(() => {
  const invalid: string[] = [];
  for (const column of props.columns) {
    try {
      assertColumnName(column.name);
    } catch {
      invalid.push(column.name);
    }
  }
  return invalid;
});

function columnError(column: ColumnInfo): string {
  return invalidColumns.value.includes(column.name) ? '列名不合法，无法写入该列' : '';
}

const submitting = ref(false);

function submit(): void {
  if (submitting.value || invalidColumns.value.length > 0) return;
  if (entries.value.length === 0) return;
  submitting.value = true;
  try {
    emit('submit', entries.value);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <Dialog :open="open" @update:open="(value) => emit('update:open', value)">
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>新增行</DialogTitle>
        <DialogDescription>
          写入表「{{ table }}」：勾选 NULL 写入 NULL，留空跳过该列（使用表默认值）
        </DialogDescription>
      </DialogHeader>

      <ErrorState v-if="error" :message="error" compact />

      <div class="max-h-[55vh] space-y-3 overflow-y-auto px-1 py-1">
        <FormRow
          v-for="column in columns"
          :key="column.name"
          :label="column.name"
          :description="metaText(column)"
          :error="columnError(column)"
        >
          <template #label-action>
            <Label class="flex items-center gap-1 text-xs text-muted-foreground">
              <Checkbox
                :model-value="fields[column.name]?.isNull ?? false"
                aria-label="该列写入 NULL"
                @update:model-value="
                  fields[column.name] && (fields[column.name]!.isNull = $event === true)
                "
              />
              NULL
            </Label>
          </template>
          <template #default="{ id, invalid, describedBy }">
            <Input
              :id="id"
              v-model="fields[column.name]!.raw"
              class="h-8 font-mono text-xs"
              :class="isNumeric(column) && 'text-right tabular-nums'"
              :disabled="fields[column.name]?.isNull"
              :aria-invalid="invalid"
              :aria-describedby="describedBy"
              :placeholder="
                isRowidAlias(column) ? '留空自动赋值' : isNumeric(column) ? '数字' : '文本'
              "
            />
          </template>
        </FormRow>
      </div>

      <p class="text-xs text-muted-foreground">
        将写入 <span class="tabular-nums">{{ entries.length }}</span> 列
      </p>

      <DialogFooter>
        <Button variant="outline" size="sm" @click="emit('update:open', false)">取消</Button>
        <Button
          size="sm"
          :disabled="entries.length === 0 || invalidColumns.length > 0 || submitting"
          @click="submit"
        >
          新增行
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
