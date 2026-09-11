<!--
  图片 OCR 工具页：识别图片中的文字。
  支持点击选择、粘贴图片，展示预览和识别结果。
-->
<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { ipc } from '@/core/ipc';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { ScanText, RotateCcw, Upload } from '@lucide/vue';
import ToolShell from '@/components/tool/ToolShell.vue';
import { OCR_EXTENSIONS } from '../shared';

interface OcrResult {
  text: string;
  confidence: number;
}

const filePath = ref('');
const fileName = ref('');
const imagePreview = ref('');
const resultText = ref('');
const confidence = ref(0);
const loading = ref(false);

/** 从 File 对象读取为 data URL（用于预览） */
function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

/** Uint8Array → base64（分块，避免参数上限） */
function bytesToBase64(bytes: Uint8Array): string {
  const chunks: string[] = [];
  for (let i = 0; i < bytes.length; i += 8192) {
    chunks.push(String.fromCharCode(...bytes.subarray(i, i + 8192)));
  }
  return btoa(chunks.join(''));
}

/** 将 File 保存到临时文件，返回路径 */
async function saveFileToTemp(file: File): Promise<string> {
  const { join, appDataDir } = await import('@tauri-apps/api/path');
  const dataDir = await appDataDir();
  const ext = file.name.split('.').pop() ?? 'png';
  const name = `ocr_${Date.now()}.${ext}`;
  const path = await join(dataDir, name);

  const buffer = await file.arrayBuffer();
  await ipc('file_write_bytes', { path, contents: bytesToBase64(new Uint8Array(buffer)) });

  return path;
}

/** 执行 OCR 识别 */
async function runOcr(path: string, preview: string, name: string): Promise<void> {
  filePath.value = path;
  fileName.value = name;
  imagePreview.value = preview;
  resultText.value = '';
  confidence.value = 0;
  loading.value = true;

  try {
    const result = await ipc<OcrResult>('daily_tools_ocr_image', { path });
    resultText.value = result.text;
    confidence.value = result.confidence;
    toast.success('识别完成');
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    toast.error(`识别失败: ${message}`);
  } finally {
    loading.value = false;
  }
}

/** 点击选择图片 */
async function handleOpenImage(): Promise<void> {
  const path = await openFileDialog({
    filters: [{ name: '图片', extensions: OCR_EXTENSIONS }],
  });
  if (!path) return;

  const name = String(path).split(/[/\\]/).pop() ?? '';
  // 用框架文件命令读取为 base64，直接构造 data URL 预览
  const base64 = await ipc<string>('file_read_bytes', { path: String(path) });
  const ext = name.split('.').pop() ?? 'png';
  const mime = ext === 'jpg' ? 'image/jpeg' : `image/${ext}`;
  const preview = `data:${mime};base64,${base64}`;

  await runOcr(String(path), preview, name);
}

/** 粘贴图片 */
async function handlePaste(e: ClipboardEvent): Promise<void> {
  const items = e.clipboardData?.items;
  if (!items) return;

  for (const item of items) {
    if (item.kind === 'file') {
      const file = item.getAsFile();
      if (!file) continue;

      const ext = file.type.split('/')[1] ?? 'png';
      const name = `paste_${Date.now()}.${ext}`;
      const preview = await readFileAsDataUrl(file);
      const path = await saveFileToTemp(file);
      await runOcr(path, preview, name);
      return;
    }
  }
}

async function copyText(): Promise<void> {
  if (!resultText.value) return;
  try {
    await navigator.clipboard.writeText(resultText.value);
    toast.success('已复制到剪贴板');
  } catch {
    toast.error('复制失败');
  }
}

function handleReset(): void {
  filePath.value = '';
  fileName.value = '';
  imagePreview.value = '';
  resultText.value = '';
  confidence.value = 0;
}

onMounted(() => {
  document.addEventListener('paste', handlePaste);
});
onUnmounted(() => {
  document.removeEventListener('paste', handlePaste);
});
</script>

<template>
  <ToolShell title="图片 OCR" description="识别图片中的文字（支持粘贴）">
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
        <!-- 无图片时显示选择区域 -->
        <button
          v-if="!filePath"
          type="button"
          class="flex min-h-[180px] w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-lg border-2 border-dashed border-muted-foreground/25 bg-muted/30 px-6 py-8 text-center transition-colors hover:border-muted-foreground/40 hover:bg-muted/50"
          @click="handleOpenImage"
        >
          <Upload class="size-8 text-muted-foreground" />
          <p class="text-sm text-muted-foreground">点击选择图片，或 Ctrl+V 粘贴</p>
          <p class="text-xs text-muted-foreground">
            支持格式：{{ OCR_EXTENSIONS.map((e) => `.${e}`).join('、') }}
          </p>
        </button>

        <!-- 有图片时显示预览和结果 -->
        <template v-else>
          <!-- 文件信息栏 -->
          <div class="flex items-center justify-between rounded-lg border bg-card px-4 py-3">
            <div class="flex min-w-0 items-center gap-3">
              <ScanText class="size-5 shrink-0 text-muted-foreground" />
              <div class="min-w-0">
                <p class="truncate text-sm font-medium">{{ fileName }}</p>
                <p v-if="loading" class="text-xs text-muted-foreground">识别中…</p>
                <p v-else-if="resultText" class="text-xs text-success">
                  识别完成 · 置信度 {{ (confidence * 100).toFixed(1) }}%
                </p>
              </div>
            </div>
            <div class="flex shrink-0 gap-2">
              <Button variant="ghost" size="sm" @click="handleReset">
                <RotateCcw class="mr-1 size-3.5" />
                重新选择
              </Button>
              <Button variant="secondary" size="sm" @click="handleOpenImage">
                <Upload class="mr-1 size-3.5" />
                打开图片
              </Button>
            </div>
          </div>

          <!-- 图片预览 + 识别结果 双栏 -->
          <div class="grid grid-cols-1 gap-4 md:grid-cols-2">
            <!-- 图片预览 -->
            <div class="overflow-hidden rounded-lg border bg-card">
              <div class="border-b px-4 py-2">
                <span class="text-xs font-medium text-muted-foreground">图片预览</span>
              </div>
              <div class="flex items-center justify-center bg-muted/30 p-2">
                <img
                  :src="imagePreview"
                  :alt="fileName"
                  class="max-h-[400px] max-w-full object-contain"
                />
              </div>
            </div>

            <!-- 识别结果 -->
            <div class="overflow-hidden rounded-lg border bg-card">
              <div class="flex items-center justify-between border-b px-4 py-2">
                <span class="text-xs font-medium text-muted-foreground">识别结果</span>
                <Button variant="ghost" size="sm" :disabled="!resultText" @click="copyText">
                  复制文字
                </Button>
              </div>
              <div class="p-4">
                <div v-if="loading" class="flex items-center justify-center py-8">
                  <div class="flex items-center gap-2 text-muted-foreground">
                    <div
                      class="size-4 animate-spin rounded-full border-2 border-current border-t-transparent"
                    />
                    <span class="text-sm">正在识别…</span>
                  </div>
                </div>
                <pre
                  v-else-if="resultText"
                  class="max-h-[380px] overflow-auto whitespace-pre-wrap break-words font-mono text-sm leading-relaxed"
                  >{{ resultText }}</pre>
                <p v-else class="py-8 text-center text-sm text-muted-foreground">
                  识别结果将显示在这里
                </p>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </ToolShell>
</template>
