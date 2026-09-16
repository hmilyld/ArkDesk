<!--
  通用键值编辑器：启用开关 + 名称 + 值 + 删除，底部添加行。
  props 驱动，无业务状态，供 Params / Headers / Cookies / Form 复用。
-->
<script setup lang="ts">
import { Plus, Trash2 } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { kv, type HttpNameValue } from '../shared';

const props = withDefaults(
  defineProps<{
    modelValue: HttpNameValue[];
    addLabel?: string;
    keyPlaceholder?: string;
    valuePlaceholder?: string;
  }>(),
  { addLabel: '添加', keyPlaceholder: '名称', valuePlaceholder: '值' }
);

const emit = defineEmits<{ 'update:modelValue': [value: HttpNameValue[]] }>();

function update(rows: HttpNameValue[]): void {
  emit('update:modelValue', rows);
}
function patch(id: string, changes: Partial<HttpNameValue>): void {
  update(props.modelValue.map((row) => (row.id === id ? { ...row, ...changes } : row)));
}
function remove(id: string): void {
  update(props.modelValue.filter((row) => row.id !== id));
}
function add(): void {
  update([...props.modelValue, kv()]);
}
</script>

<template>
  <div class="space-y-1.5">
    <div v-for="row in modelValue" :key="row.id" class="flex items-center gap-2">
      <Checkbox
        :model-value="row.enabled"
        @update:model-value="(value) => patch(row.id, { enabled: value === true })"
      />
      <Input
        :model-value="row.name"
        :placeholder="keyPlaceholder"
        spellcheck="false"
        class="font-mono text-xs"
        @update:model-value="(value) => patch(row.id, { name: String(value) })"
      />
      <Input
        :model-value="row.value"
        :placeholder="valuePlaceholder"
        spellcheck="false"
        class="font-mono text-xs"
        @update:model-value="(value) => patch(row.id, { value: String(value) })"
      />
      <Button
        variant="ghost"
        size="icon-sm"
        type="button"
        title="删除"
        aria-label="删除"
        @click="remove(row.id)"
      >
        <Trash2 class="size-3.5" />
      </Button>
    </div>
    <Button variant="outline" size="sm" type="button" class="w-full" @click="add">
      <Plus class="mr-1 size-3.5" />
      {{ addLabel }}
    </Button>
  </div>
</template>
