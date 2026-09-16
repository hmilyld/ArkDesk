<!--
  cURL 导入 / 导出：粘贴浏览器 F12「Copy as cURL」反向填充，或复制当前请求为 curl。
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';
import ErrorState from '@/components/native/ErrorState.vue';
import { parseCurl } from '../curl';
import { copyToClipboard, type HttpRequestSpec } from '../shared';

const props = defineProps<{ open: boolean; exportText: string }>();
const emit = defineEmits<{
  'update:open': [value: boolean];
  import: [spec: HttpRequestSpec];
}>();

const input = ref('');
const error = ref('');
const copied = ref(false);

const open = computed({
  get: () => props.open,
  set: (value: boolean) => emit('update:open', value),
});

async function copyExport(): Promise<void> {
  if (!props.exportText) return;
  try {
    await copyToClipboard(props.exportText);
    error.value = '';
    copied.value = true;
    setTimeout(() => (copied.value = false), 1500);
  } catch {
    error.value = '复制到剪贴板失败，请检查系统剪贴板权限';
  }
}

function doImport(): void {
  if (!input.value.trim()) return;
  try {
    emit('import', parseCurl(input.value));
    error.value = '';
    open.value = false;
    input.value = '';
  } catch {
    error.value = 'curl 解析失败，请检查粘贴内容是否为完整的 curl 命令';
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="max-w-[92vw] sm:max-w-2xl">
      <DialogHeader>
        <DialogTitle>cURL 互转</DialogTitle>
        <DialogDescription
          >粘贴浏览器「Copy as cURL」导入，或复制当前请求为 curl 命令。</DialogDescription
        >
      </DialogHeader>

      <ErrorState v-if="error" :message="error" />

      <Tabs default-value="import" class="gap-3">
        <TabsList>
          <TabsTrigger value="import">导入</TabsTrigger>
          <TabsTrigger value="export">导出</TabsTrigger>
        </TabsList>

        <TabsContent value="import" class="space-y-2">
          <Textarea
            v-model="input"
            placeholder="curl 'https://...' -H '...' -d '...'"
            spellcheck="false"
            class="min-h-48 font-mono text-xs"
            @input="error = ''"
          />
          <DialogFooter>
            <Button size="sm" :disabled="!input.trim()" @click="doImport">解析并填入</Button>
          </DialogFooter>
        </TabsContent>

        <TabsContent value="export" class="space-y-2">
          <Textarea
            :model-value="exportText"
            readonly
            spellcheck="false"
            class="min-h-48 font-mono text-xs"
          />
          <DialogFooter>
            <Button size="sm" variant="outline" :disabled="!exportText" @click="copyExport">
              {{ copied ? '已复制' : '复制 curl' }}
            </Button>
          </DialogFooter>
        </TabsContent>
      </Tabs>
    </DialogContent>
  </Dialog>
</template>
