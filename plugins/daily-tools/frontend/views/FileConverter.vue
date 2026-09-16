<!--
  文件转换工具页：将常见文档格式转换为 Markdown 源码。
  使用 Tauri 对话框 API 选择文件，路径直接传给 Rust 处理。
-->
<script setup lang="ts">
import { ref } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { ipc } from '@/core/ipc';
import { open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { FileDown, FileText, RotateCcw, Upload } from '@lucide/vue';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';
import MarkdownPreview from '../components/MarkdownPreview.vue';
import { CONVERT_EXTENSIONS, errorMessage } from '../shared';

interface ConvertResult {
  markdown: string;
}

const filePath = ref('');
const fileName = ref('');
const markdown = ref('');
const loading = ref(false);

async function handleOpenFile(): Promise<void> {
  const path = await openFileDialog({
    filters: [{ name: '文档', extensions: CONVERT_EXTENSIONS }],
  });
  if (!path) return;

  filePath.value = String(path);
  fileName.value = filePath.value.split(/[/\\]/).pop() ?? '';
  markdown.value = '';
  loading.value = true;

  try {
    const result = await ipc<ConvertResult>('daily_tools_convert_file', {
      path: filePath.value,
    });
    markdown.value = result.markdown;
    toast.success('转换完成');
  } catch (err) {
    toast.error(`转换失败：${errorMessage(err)}`);
  } finally {
    loading.value = false;
  }
}

async function handleSave(): Promise<void> {
  if (!markdown.value) return;
  const path = await saveFileDialog({
    defaultPath: fileName.value.replace(/\.[^.]+$/, '.md'),
    filters: [{ name: 'Markdown', extensions: ['md'] }],
  });
  if (!path) return;

  try {
    await ipc('daily_tools_save_markdown', {
      path: String(path),
      content: markdown.value,
    });
    toast.success('文件已保存');
  } catch (err) {
    toast.error(`保存失败：${errorMessage(err)}`);
  }
}

function handleReset(): void {
  filePath.value = '';
  fileName.value = '';
  markdown.value = '';
}
</script>

<template>
  <ToolShell title="文件转换" description="将常见文档格式转换为 Markdown 源码">
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
        <!-- 无文件时显示选择区域 -->
        <Panel v-if="!filePath" title="输入">
          <template #actions>
            <Button variant="secondary" size="sm" @click="handleOpenFile">
              <Upload class="mr-1 size-3.5" />
              选择文件
            </Button>
          </template>

          <button
            type="button"
            class="flex min-h-[180px] w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-md bg-sunken px-6 py-8 text-center transition-colors hover:bg-accent"
            @click="handleOpenFile"
          >
            <Upload class="size-8 text-muted-foreground" />
            <p class="text-sm text-muted-foreground">点击选择文件</p>
            <p class="text-xs text-muted-foreground">
              支持格式：{{ CONVERT_EXTENSIONS.map((e) => `.${e}`).join('、') }}
            </p>
          </button>
        </Panel>

        <!-- 有文件时显示状态和结果 -->
        <template v-else>
          <!-- 文件信息 -->
          <Panel title="文件" :hint="fileName">
            <template #actions>
              <Button variant="ghost" size="sm" @click="handleReset">
                <RotateCcw class="mr-1 size-3.5" />
                重新选择
              </Button>
              <Button variant="secondary" size="sm" @click="handleOpenFile">
                <FileDown class="mr-1 size-3.5" />
                打开文件
              </Button>
            </template>

            <div class="flex items-center gap-3 text-xs">
              <FileText class="size-4 shrink-0 text-muted-foreground" />
              <span v-if="loading" class="text-muted-foreground">转换中…</span>
              <span v-else-if="markdown" class="text-success">转换完成</span>
              <span v-else class="text-muted-foreground">等待转换</span>
            </div>
          </Panel>

          <!-- 加载提示 -->
          <Panel v-if="loading" title="Markdown">
            <div class="flex items-center justify-center py-10">
              <div class="flex items-center gap-2 text-muted-foreground">
                <div
                  class="size-4 animate-spin rounded-full border-2 border-current border-t-transparent"
                />
                <span class="text-sm">正在转换…</span>
              </div>
            </div>
          </Panel>

          <!-- Markdown 结果 -->
          <MarkdownPreview v-else-if="markdown" :content="markdown">
            <template #actions>
              <Button variant="secondary" size="sm" @click="handleSave">
                <FileDown class="mr-1 size-3.5" />
                导出 .md
              </Button>
            </template>
          </MarkdownPreview>
        </template>
      </div>
    </div>
  </ToolShell>
</template>
