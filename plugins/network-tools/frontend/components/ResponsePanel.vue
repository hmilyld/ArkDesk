<!--
  响应查看：状态摘要 + Body / Headers / Cookies 三个 tab。
  仅文本预览；二进制或任意内容均可另存为文件。
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { toast } from 'vue-sonner';
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { Check, Copy, Download } from '@lucide/vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ipc } from '@/core/ipc';
import { errorMessage } from '../error';
import {
  copyToClipboard,
  formatBytes,
  formatDuration,
  prettyBody,
  statusBadgeVariant,
  type HttpResponseView,
} from '../shared';

const props = withDefaults(
  defineProps<{
    response: HttpResponseView | null;
    loading: boolean;
    error: string;
    warnings: string[];
    /** 撑满父容器高度（拦截器详情用）；默认按内容限高 */
    fill?: boolean;
  }>(),
  { fill: false }
);
const emit = defineEmits<{ 'apply-cookie': [cookieHeader: string] }>();

const viewMode = ref<'pretty' | 'raw'>('pretty');
const copied = ref(false);

const bodyText = computed(() => props.response?.bodyText ?? '');
const prettyText = computed(() => prettyBody(bodyText.value, props.response?.contentType));
const hasPretty = computed(() => bodyText.value !== prettyText.value);
const displayBody = computed(() =>
  viewMode.value === 'pretty' ? prettyText.value : bodyText.value
);
const headerEntries = computed(() => props.response?.headers ?? []);
const emptyHint = computed(() => {
  const status = props.response?.status ?? 0;
  if (status === 304)
    return '304 未修改：网络层无响应体；若页面由浏览器缓存 / Service Worker（sw.js、workbox）提供，则根本不会发起文档请求';
  if (status === 204) return '204 无内容：该响应没有 body（常见于探测 / 预检 / 空操作）';
  if (status >= 300 && status < 400) return `${status} 重定向：通常不携带响应体`;
  return '（空响应体）';
});
const setCookies = computed(() =>
  headerEntries.value.filter((header) => header.name.toLowerCase() === 'set-cookie')
);

async function copyText(text: string, label = '已复制'): Promise<void> {
  if (!text) return;
  try {
    await copyToClipboard(text);
    copied.value = true;
    toast.success(label);
    setTimeout(() => (copied.value = false), 1500);
  } catch {
    toast.error('复制失败');
  }
}

function suggestFileName(): string {
  const res = props.response;
  if (!res) return 'response.txt';
  try {
    const name = new URL(res.finalUrl).pathname.split('/').filter(Boolean).pop();
    if (name) return name;
  } catch {
    /* ignore */
  }
  return res.isBinary ? 'response.bin' : 'response.txt';
}

async function download(): Promise<void> {
  const res = props.response;
  if (!res) return;
  const target = await saveFileDialog({ defaultPath: suggestFileName() });
  if (!target) return;
  try {
    if (res.isBinary && res.bodyBase64) {
      await ipc('file_write_bytes', { path: target, contents: res.bodyBase64 });
    } else {
      await ipc('file_write_text', { path: target, contents: res.bodyText ?? '' });
    }
    toast.success('已保存', { description: target });
  } catch (err) {
    toast.error(`保存失败：${errorMessage(err)}`);
  }
}

function applySetCookie(setCookieValue: string): void {
  const pair = setCookieValue.split(';')[0]?.trim();
  if (pair) emit('apply-cookie', pair);
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <p
      v-if="error"
      class="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 font-mono text-xs text-destructive"
    >
      {{ error }}
    </p>
    <p
      v-for="warning in warnings"
      :key="warning"
      class="rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning"
    >
      {{ warning }}
    </p>

    <div
      v-if="!response && !error"
      class="flex flex-1 items-center justify-center rounded-lg border border-dashed text-xs text-muted-foreground"
    >
      {{ loading ? '请求中…' : '发送请求后在此查看响应' }}
    </div>

    <template v-if="response">
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
        <Badge :variant="statusBadgeVariant(response.status)">
          {{ response.status }} {{ response.statusText }}
        </Badge>
        <span class="font-mono text-xs text-muted-foreground">{{
          formatDuration(response.elapsedMs)
        }}</span>
        <span class="font-mono text-xs text-muted-foreground">{{
          formatBytes(response.sizeBytes)
        }}</span>
        <span v-if="response.truncated" class="text-xs text-warning">（已截断）</span>
        <span class="truncate font-mono text-xs text-muted-foreground">{{
          response.finalUrl
        }}</span>
      </div>

      <p
        v-if="response.redirectLocation"
        class="rounded-md border border-border bg-sunken px-3 py-2 font-mono text-xs"
      >
        重定向至：{{ response.redirectLocation }}
      </p>

      <Tabs default-value="body" class="flex min-h-0 flex-1 flex-col gap-2">
        <div class="flex items-center justify-between gap-2">
          <TabsList>
            <TabsTrigger value="body">Body</TabsTrigger>
            <TabsTrigger value="headers">Headers ({{ headerEntries.length }})</TabsTrigger>
            <TabsTrigger value="cookies">Cookies ({{ setCookies.length }})</TabsTrigger>
          </TabsList>
          <div class="flex items-center gap-1">
            <template v-if="!response.isBinary">
              <Button
                variant="ghost"
                size="xs"
                :class="{ 'text-primary': viewMode === 'pretty' }"
                :disabled="!hasPretty"
                @click="viewMode = 'pretty'"
              >
                Pretty
              </Button>
              <Button
                variant="ghost"
                size="xs"
                :class="{ 'text-primary': viewMode === 'raw' }"
                @click="viewMode = 'raw'"
              >
                Raw
              </Button>
            </template>
            <Button
              v-if="!response.isBinary"
              variant="ghost"
              size="xs"
              @click="copyText(displayBody, '已复制响应体')"
            >
              <component :is="copied ? Check : Copy" class="size-3.5" />
              复制
            </Button>
            <Button variant="ghost" size="xs" @click="download">
              <Download class="size-3.5" />
              下载
            </Button>
          </div>
        </div>

        <TabsContent value="body" class="min-h-0 flex-1">
          <div
            v-if="response.isBinary"
            class="rounded-lg border border-border p-4 text-xs text-muted-foreground"
          >
            二进制响应（{{ response.contentType || '未知类型' }}，{{
              formatBytes(response.sizeBytes)
            }}），请下载查看。
          </div>
          <pre
            v-else
            class="overflow-auto rounded-lg border border-border bg-console p-3 font-mono text-xs leading-5 text-console-foreground whitespace-pre-wrap break-all"
            :class="fill ? 'h-full min-h-0' : 'max-h-96 min-h-40'"
            >{{ displayBody || emptyHint }}</pre>
        </TabsContent>

        <TabsContent value="headers" class="min-h-0 flex-1">
          <div
            class="divide-y divide-border/60 overflow-auto rounded-lg border border-border font-mono text-xs"
            :class="fill ? 'h-full' : 'max-h-96'"
          >
            <p
              v-for="(header, index) in headerEntries"
              :key="`${header.name}-${index}`"
              class="px-3 py-1.5 break-all"
            >
              <span class="text-muted-foreground">{{ header.name }}:</span> {{ header.value }}
            </p>
            <p
              v-if="headerEntries.length === 0"
              class="px-3 py-4 text-center text-muted-foreground"
            >
              无响应头
            </p>
          </div>
        </TabsContent>

        <TabsContent value="cookies" class="min-h-0 flex-1">
          <div class="space-y-2 overflow-auto" :class="fill ? 'h-full' : 'max-h-96'">
            <div
              v-for="(cookie, index) in setCookies"
              :key="index"
              class="flex items-start justify-between gap-3 rounded-lg border border-border px-3 py-2"
            >
              <span class="min-w-0 flex-1 break-all font-mono text-xs">{{ cookie.value }}</span>
              <Button variant="outline" size="xs" @click="applySetCookie(cookie.value)">
                复制到 Cookie 编辑器
              </Button>
            </div>
            <p
              v-if="setCookies.length === 0"
              class="py-4 text-center text-xs text-muted-foreground"
            >
              响应未下发 Set-Cookie
            </p>
          </div>
        </TabsContent>
      </Tabs>
    </template>
  </div>
</template>
