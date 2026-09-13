<!--
  cURL 导入 / 导出：粘贴浏览器 F12「Copy as cURL」反向填充，或复制当前请求为 curl。
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
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
import { parseCurl } from '../curl';
import { copyToClipboard, type HttpRequestSpec } from '../shared';

const props = defineProps<{ open: boolean; exportText: string }>();
const emit = defineEmits<{
  'update:open': [value: boolean];
  import: [spec: HttpRequestSpec];
}>();

const input = ref('');

const open = computed({
  get: () => props.open,
  set: (value: boolean) => emit('update:open', value),
});

async function copyExport(): Promise<void> {
  if (!props.exportText) return;
  try {
    await copyToClipboard(props.exportText);
    toast.success('已复制 curl');
  } catch {
    toast.error('复制失败');
  }
}

function doImport(): void {
  if (!input.value.trim()) return;
  try {
    emit('import', parseCurl(input.value));
    toast.success('已解析并填入请求');
    open.value = false;
    input.value = '';
  } catch {
    toast.error('curl 解析失败');
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
              复制 curl
            </Button>
          </DialogFooter>
        </TabsContent>
      </Tabs>
    </DialogContent>
  </Dialog>
</template>
