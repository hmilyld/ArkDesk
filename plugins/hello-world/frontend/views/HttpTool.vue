<!--
  HTTP 请求：core/http 原始响应演示（span=10 档位示范）。
  输入地址（默认 baidu.com）→ GET/POST → 展示状态码、耗时、响应头与响应体。
  JSON 响应自动美化；HTML 等文本原样展示。错误就地呈现并提供重试。
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { Globe } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Textarea } from '@/components/ui/textarea';
import { http, type HttpResponse } from '@/core/http';
import { normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';
import EmptyState from '@/components/native/EmptyState.vue';
import ErrorState from '@/components/native/ErrorState.vue';
import FormRow from '@/components/native/FormRow.vue';
import LoadingState from '@/components/native/LoadingState.vue';
import Segmented from '@/components/native/Segmented.vue';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';

/** 2 个互斥方法用分段控件（DESIGN §3） */
const METHODS = [
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
];

const url = ref('https://www.baidu.com');
const method = ref<'GET' | 'POST'>('GET');
const body = ref('');
const loading = ref(false);
const response = ref<HttpResponse | null>(null);
const errorText = ref('');

const methodModel = computed({
  get: () => method.value,
  set: (value: string) => (method.value = value as 'GET' | 'POST'),
});

/** JSON 响应自动美化（失败则原样返回） */
const prettyBody = computed(() => {
  const raw = response.value?.body ?? '';
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
});

const headerEntries = computed(() => Object.entries(response.value?.headers ?? {}));

async function send(): Promise<void> {
  loading.value = true;
  errorText.value = '';
  response.value = null;
  try {
    response.value =
      method.value === 'GET'
        ? await http.get(url.value.trim())
        : await http.post(url.value.trim(), body.value);
    logger.info(`HTTP ${method.value} ${url.value} → ${response.value.status}`);
  } catch (err) {
    const error = normalizeError(err);
    errorText.value = `[${error.code}] ${error.message}，请检查地址或网络后重试。`;
    logger.error(`HTTP 请求失败: [${error.code}] ${error.message}`);
  } finally {
    loading.value = false;
  }
}

// ── 流式下载（core/http download + http://download-progress 进度事件） ──
const downloadUrl = ref('https://speed.cloudflare.com/__down?bytes=10485760');
const downloading = ref(false);
const downloadPercent = ref(0);
const downloadedBytes = ref(0);
const totalBytes = ref<number | null>(null);
const downloadPath = ref('');
const downloadError = ref('');

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

async function downloadFile(): Promise<void> {
  const url = downloadUrl.value.trim();
  if (!url) return;
  const target = await saveFileDialog({ defaultPath: 'pocketark-download.bin' });
  if (!target) return;

  downloading.value = true;
  downloadError.value = '';
  downloadPath.value = '';
  downloadPercent.value = 0;
  downloadedBytes.value = 0;
  totalBytes.value = null;
  try {
    const saved = await http.download(url, target, {
      onProgress: ({ downloaded, total }) => {
        downloadedBytes.value = downloaded;
        totalBytes.value = total;
        if (total && total > 0) {
          downloadPercent.value = Math.min(100, Math.round((downloaded / total) * 100));
        }
      },
    });
    // 成功用结果区表达，不弹 toast
    downloadPath.value = saved;
    logger.info(`下载完成: ${saved}`);
  } catch (err) {
    const error = normalizeError(err);
    downloadError.value = `[${error.code}] ${error.message}，请检查地址后重试。`;
    logger.error(`下载失败: [${error.code}] ${error.message}`);
  } finally {
    downloading.value = false;
  }
}
</script>

<template>
  <ToolShell
    title="HTTP 请求"
    description="core/http 通道演示：Rust reqwest 发起，无 CORS 限制，支持请求与流式下载"
  >
    <div class="grid grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-10 lg:col-start-2">
        <!-- 请求区 -->
        <Panel title="请求" hint="core/http">
          <form class="space-y-3" @submit.prevent="send">
            <FormRow label="请求地址" description="选择方法并填写地址，回车发送">
              <template #default="{ id }">
                <div class="flex gap-2">
                  <Segmented v-model="methodModel" :segments="METHODS" size="sm" />
                  <Input
                    :id="id"
                    v-model="url"
                    placeholder="https://"
                    spellcheck="false"
                    class="flex-1"
                    :disabled="loading"
                  />
                </div>
              </template>
            </FormRow>
            <FormRow
              v-if="method === 'POST'"
              label="请求体"
              description="按原文发送，不自动加 Content-Type"
            >
              <Textarea
                v-model="body"
                placeholder='{"key": "value"}'
                spellcheck="false"
                class="min-h-20 font-mono text-xs"
              />
            </FormRow>
            <div class="flex justify-end">
              <Button type="submit" :disabled="loading || !url.trim()">
                {{ loading ? '请求中…' : '发送' }}
              </Button>
            </div>
          </form>
        </Panel>

        <!-- 响应区（三态：加载 / 错误 / 空 / 结果） -->
        <Panel title="响应" hint="状态码 · 耗时 · 响应头 · 响应体">
          <LoadingState v-if="loading" :rows="5" />
          <ErrorState v-else-if="errorText" :message="errorText" :on-retry="send" />
          <EmptyState
            v-else-if="!response"
            :icon="Globe"
            title="还没有响应"
            description="输入地址后发送，默认请求 baidu.com 体验 HTML 响应；换成 JSON API 可看自动美化"
          />
          <template v-else>
            <!-- 状态摘要 -->
            <div class="flex flex-wrap items-center gap-2">
              <Badge :variant="response.ok ? 'default' : 'destructive'">
                {{ response.status }} {{ response.ok ? 'OK' : 'Error' }}
              </Badge>
              <span class="font-mono text-xs tabular-nums text-muted-foreground">
                {{ response.elapsedMs }} ms
              </span>
              <span class="truncate font-mono text-xs text-muted-foreground">
                {{ response.finalUrl }}
              </span>
            </div>

            <!-- 响应体：只读控制台文本块 -->
            <div class="space-y-1">
              <p class="text-xs font-medium text-muted-foreground">
                响应体{{ prettyBody !== response.body ? '（JSON 已美化）' : '' }}
              </p>
              <pre
                class="max-h-96 overflow-auto rounded-md bg-console p-3 font-mono text-xs whitespace-pre-wrap break-words text-console-foreground/80"
                >{{ prettyBody }}</pre>
            </div>

            <!-- 响应头（可折叠，细节演示 details/summary 原生用法） -->
            <details class="overflow-hidden rounded-md border">
              <summary
                class="cursor-pointer px-3 py-2 text-xs font-medium text-muted-foreground select-none"
              >
                响应头（{{ headerEntries.length }}）
              </summary>
              <div class="divide-y divide-border border-t px-3 py-1 font-mono text-xs">
                <p v-for="[key, value] in headerEntries" :key="key" class="py-1.5 break-all">
                  <span class="text-muted-foreground">{{ key }}:</span>
                  {{ value }}
                </p>
              </div>
            </details>
          </template>
        </Panel>

        <!-- 流式下载：http.download + 进度事件 -->
        <Panel title="流式下载" hint="http.download">
          <p class="text-xs text-muted-foreground">
            由 Rust 侧流式写入文件，进度经 http://download-progress 事件回传
          </p>
          <FormRow label="文件地址" description="默认 10 MB 测速文件，用于观察下载进度">
            <template #default="{ id }">
              <div class="flex gap-2">
                <Input
                  :id="id"
                  v-model="downloadUrl"
                  placeholder="文件 URL"
                  spellcheck="false"
                  class="flex-1"
                  :disabled="downloading"
                />
                <Button
                  variant="secondary"
                  :disabled="downloading || !downloadUrl.trim()"
                  @click="downloadFile"
                >
                  {{ downloading ? '下载中…' : '下载' }}
                </Button>
              </div>
            </template>
          </FormRow>

          <ErrorState v-if="downloadError" :message="downloadError" :on-retry="downloadFile" />

          <div v-if="downloading || downloadPath" class="space-y-1">
            <div class="h-2 overflow-hidden rounded-full bg-muted">
              <div
                class="h-full rounded-full bg-primary transition-[width] duration-150"
                :style="{ width: `${downloading ? Math.max(downloadPercent, 2) : 100}%` }"
              />
            </div>
            <div
              class="flex items-center justify-between font-mono text-xs tabular-nums text-muted-foreground"
            >
              <span>
                {{ formatBytes(downloadedBytes) }}
                <template v-if="totalBytes"> / {{ formatBytes(totalBytes) }}</template>
              </span>
              <span>{{ totalBytes ? `${downloadPercent}%` : '未知大小' }}</span>
            </div>
            <p v-if="downloadPath" class="break-all font-mono text-xs text-muted-foreground">
              已保存：{{ downloadPath }}
            </p>
          </div>
        </Panel>
      </div>
    </div>
  </ToolShell>
</template>
