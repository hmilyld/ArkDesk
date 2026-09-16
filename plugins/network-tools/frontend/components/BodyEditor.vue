<!--
  请求体编辑器：none / raw / form-urlencoded / multipart / binary。
-->
<script setup lang="ts">
import { computed } from 'vue';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { FilePlus2, Plus, Trash2 } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import Segmented from '@/components/native/Segmented.vue';
import KeyValueEditor from './KeyValueEditor.vue';
import {
  multipartRow,
  type HttpBody,
  type HttpBodyType,
  type HttpMultipartRow,
  type HttpRawLang,
} from '../shared';

const props = defineProps<{ modelValue: HttpBody }>();
const emit = defineEmits<{ 'update:modelValue': [value: HttpBody] }>();

const BODY_TYPES: { value: HttpBodyType; label: string }[] = [
  { value: 'none', label: '无' },
  { value: 'raw', label: 'Raw' },
  { value: 'form', label: '表单 (x-www-form-urlencoded)' },
  { value: 'multipart', label: '表单 (multipart / 文件)' },
  { value: 'binary', label: '二进制文件' },
];

const RAW_LANGS: { value: HttpRawLang; label: string }[] = [
  { value: 'json', label: 'JSON' },
  { value: 'text', label: 'Text' },
  { value: 'xml', label: 'XML' },
  { value: 'html', label: 'HTML' },
];

const body = computed(() => props.modelValue);

function patch(changes: Partial<HttpBody>): void {
  emit('update:modelValue', { ...props.modelValue, ...changes });
}
function setType(value: unknown): void {
  patch({ type: value as HttpBodyType });
}
function setRawLang(value: unknown): void {
  patch({ rawLang: value as HttpRawLang });
}
function patchMultipart(id: string, changes: Partial<HttpMultipartRow>): void {
  patch({
    multipart: props.modelValue.multipart.map((row) =>
      row.id === id ? { ...row, ...changes } : row
    ),
  });
}
function addMultipart(): void {
  patch({ multipart: [...props.modelValue.multipart, multipartRow()] });
}
function removeMultipart(id: string): void {
  patch({ multipart: props.modelValue.multipart.filter((row) => row.id !== id) });
}
async function pickFile(row: HttpMultipartRow): Promise<void> {
  const selected = await openFileDialog({ multiple: false });
  if (typeof selected === 'string') patchMultipart(row.id, { filePath: selected });
}
async function pickBinary(): Promise<void> {
  const selected = await openFileDialog({ multiple: false });
  if (typeof selected === 'string') patch({ binaryPath: selected });
}
</script>

<template>
  <div class="space-y-3">
    <div class="flex flex-wrap items-center gap-2">
      <Label class="text-xs text-muted-foreground">类型</Label>
      <Select :model-value="body.type" @update:model-value="setType">
        <SelectTrigger class="w-64">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem v-for="item in BODY_TYPES" :key="item.value" :value="item.value">
            {{ item.label }}
          </SelectItem>
        </SelectContent>
      </Select>
      <template v-if="body.type === 'raw'">
        <Segmented
          :model-value="body.rawLang"
          size="sm"
          :segments="RAW_LANGS"
          @update:model-value="setRawLang"
        />
      </template>
    </div>

    <Textarea
      v-if="body.type === 'raw'"
      :model-value="body.raw"
      placeholder="请求体内容，支持 {{变量}}"
      spellcheck="false"
      class="min-h-40 font-mono text-xs"
      @update:model-value="(value) => patch({ raw: String(value) })"
    />

    <KeyValueEditor
      v-else-if="body.type === 'form'"
      :model-value="body.form"
      add-label="添加字段"
      key-placeholder="字段名"
      value-placeholder="字段值"
      @update:model-value="(value) => patch({ form: value })"
    />

    <div v-else-if="body.type === 'multipart'" class="space-y-1.5">
      <div v-for="row in body.multipart" :key="row.id" class="flex items-center gap-2">
        <Checkbox
          :model-value="row.enabled"
          @update:model-value="(value) => patchMultipart(row.id, { enabled: value === true })"
        />
        <Input
          :model-value="row.name"
          placeholder="字段名"
          spellcheck="false"
          class="w-40 font-mono text-xs"
          @update:model-value="(value) => patchMultipart(row.id, { name: String(value) })"
        />
        <Select
          :model-value="row.kind"
          @update:model-value="
            (value) => patchMultipart(row.id, { kind: value as 'text' | 'file' })
          "
        >
          <SelectTrigger class="w-24">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="text">文本</SelectItem>
            <SelectItem value="file">文件</SelectItem>
          </SelectContent>
        </Select>
        <template v-if="row.kind === 'text'">
          <Input
            :model-value="row.value"
            placeholder="字段值"
            spellcheck="false"
            class="flex-1 font-mono text-xs"
            @update:model-value="(value) => patchMultipart(row.id, { value: String(value) })"
          />
        </template>
        <template v-else>
          <span class="flex-1 truncate font-mono text-xs text-muted-foreground">
            {{ row.filePath || '未选择文件' }}
          </span>
          <Button variant="outline" size="sm" type="button" @click="pickFile(row)">
            <FilePlus2 class="mr-1 size-3.5" />
            选择
          </Button>
        </template>
        <Button
          variant="ghost"
          size="icon-sm"
          type="button"
          title="删除"
          aria-label="删除"
          @click="removeMultipart(row.id)"
        >
          <Trash2 class="size-3.5" />
        </Button>
      </div>
      <Button variant="outline" size="sm" type="button" class="w-full" @click="addMultipart">
        <Plus class="mr-1 size-3.5" />
        添加字段
      </Button>
    </div>

    <div v-else-if="body.type === 'binary'" class="space-y-2">
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" type="button" @click="pickBinary">
          <FilePlus2 class="mr-1 size-3.5" />
          选择文件
        </Button>
        <span class="truncate font-mono text-xs text-muted-foreground">
          {{ body.binaryPath || '未选择文件' }}
        </span>
      </div>
      <Input
        :model-value="body.binaryContentType"
        placeholder="Content-Type（可选）"
        spellcheck="false"
        class="font-mono text-xs"
        @update:model-value="(value) => patch({ binaryContentType: String(value) })"
      />
    </div>

    <p v-else class="py-6 text-center text-xs text-muted-foreground">该请求不携带请求体</p>
  </div>
</template>
