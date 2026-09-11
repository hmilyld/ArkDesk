<!--
  新增行对话框：按表结构动态生成表单。
  字段语义：勾选 NULL → 写入 NULL；留空 → 跳过该列（走表默认值）；
  填写 → 解析后写入（数值亲和列转数字）。INTEGER 主键留空自动赋值。
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
import {
  assertColumnName,
  isRowidAlias,
  parseCellValue,
  type ColumnInfo,
  type InsertEntry,
} from '../shared';

const props = defineProps<{
  open: boolean;
  table: string;
  columns: ColumnInfo[];
}>();

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

      <div class="max-h-[24rem] space-y-3 overflow-y-auto py-2 pr-1">
        <div v-for="column in columns" :key="column.name" class="space-y-1">
          <div class="flex items-center justify-between gap-2">
            <Label :for="`row-${column.name}`" class="font-mono text-xs">
              {{ column.name }}
              <span class="ml-1 font-sans text-[10px] font-normal text-muted-foreground">
                {{ column.type || 'ANY' }}
                {{ column.notnull === 1 ? '· NOT NULL' : '' }}
                {{ column.pk > 0 ? '· PK' : '' }}
              </span>
            </Label>
            <Label class="flex items-center gap-1 text-[10px] text-muted-foreground">
              <Checkbox
                :model-value="fields[column.name]?.isNull ?? false"
                @update:model-value="
                  fields[column.name] && (fields[column.name]!.isNull = $event === true)
                "
              />
              NULL
            </Label>
          </div>
          <Input
            :id="`row-${column.name}`"
            v-model="fields[column.name]!.raw"
            class="h-8 font-mono text-xs"
            :disabled="fields[column.name]?.isNull"
            :placeholder="
              isRowidAlias(column) ? '留空自动赋值' : isNumeric(column) ? '数字' : '文本'
            "
          />
        </div>
      </div>

      <p class="text-xs text-muted-foreground">将写入 {{ entries.length }} 列</p>

      <DialogFooter>
        <Button variant="outline" size="sm" @click="emit('update:open', false)">取消</Button>
        <Button
          size="sm"
          :disabled="entries.length === 0 || invalidColumns.length > 0 || submitting"
          @click="submit"
        >
          插入
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
