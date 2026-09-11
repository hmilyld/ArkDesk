<!--
  HTTP 请求：core/http 原始响应演示（span=9 档位示范）。
  输入地址（默认 baidu.com）→ GET/POST → 展示状态码、耗时、响应头与响应体。
  JSON 响应自动美化；HTML 等文本原样展示。
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { http, type HttpResponse } from '@/core/http';
import { normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';
import ToolShell from '@/components/tool/ToolShell.vue';

const url = ref('https://www.baidu.com');
const method = ref<'GET' | 'POST'>('GET');
const body = ref('');
const loading = ref(false);
const response = ref<HttpResponse | null>(null);
const errorText = ref('');

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
    errorText.value = `[${error.code}] ${error.message}`;
    logger.error(`HTTP 请求失败: ${errorText.value}`);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <ToolShell
    title="HTTP 请求"
    description="core/http 通道演示：Rust reqwest 发起，无 CORS 限制，返回原始响应"
  >
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 lg:col-start-2 lg:col-span-10 space-y-4">
        <!-- 请求区 -->
        <form class="space-y-3" @submit.prevent="send">
          <div class="flex gap-2">
            <Select v-model="method" :disabled="loading">
              <SelectTrigger class="w-28">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="GET">GET</SelectItem>
                <SelectItem value="POST">POST</SelectItem>
              </SelectContent>
            </Select>
            <Input v-model="url" placeholder="https://" spellcheck="false" class="flex-1" />
            <Button type="submit" :disabled="loading || !url.trim()">
              {{ loading ? '请求中…' : '发送' }}
            </Button>
          </div>
          <div v-if="method === 'POST'" class="space-y-1">
            <Label for="http-body" class="text-xs text-muted-foreground">请求体（原样发送）</Label>
            <Textarea
              id="http-body"
              v-model="body"
              placeholder='{"key": "value"}'
              spellcheck="false"
              class="min-h-20 font-mono text-xs"
            />
          </div>
        </form>

        <!-- 响应区 -->
        <p
          v-if="errorText"
          class="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 font-mono text-xs text-destructive"
        >
          {{ errorText }}
        </p>

        <template v-if="response">
          <!-- 状态摘要 -->
          <div class="flex flex-wrap items-center gap-2">
            <Badge :variant="response.ok ? 'default' : 'destructive'">
              {{ response.status }} {{ response.ok ? 'OK' : 'Error' }}
            </Badge>
            <span class="font-mono text-xs text-muted-foreground">
              {{ response.elapsedMs }}ms
            </span>
            <span class="truncate font-mono text-xs text-muted-foreground">
              {{ response.finalUrl }}
            </span>
          </div>

          <!-- 响应体 -->
          <div class="space-y-1">
            <p class="text-xs font-medium text-muted-foreground">
              响应体{{ prettyBody !== response.body ? '（JSON 已美化）' : '' }}
            </p>
            <pre
              class="max-h-96 overflow-auto rounded-lg border border-border bg-console p-3 font-mono text-xs leading-5 text-console-foreground"
              >{{ prettyBody }}</pre>
          </div>

          <!-- 响应头（可折叠，细节演示 details/summary 原生用法） -->
          <details class="rounded-lg border border-border">
            <summary class="cursor-pointer px-3 py-2 text-xs font-medium text-muted-foreground">
              响应头（{{ headerEntries.length }}）
            </summary>
            <div
              class="divide-y divide-border/60 border-t border-border/60 px-3 py-1 font-mono text-xs"
            >
              <p v-for="[key, value] in headerEntries" :key="key" class="py-1.5 break-all">
                <span class="text-muted-foreground">{{ key }}:</span>
                {{ value }}
              </p>
            </div>
          </details>
        </template>

        <p
          v-else-if="!loading && !errorText"
          class="rounded-lg border border-dashed py-10 text-center text-xs text-muted-foreground"
        >
          输入地址后发送，默认请求 baidu.com 体验 HTML 响应；换成 JSON API 可看自动美化
        </p>
      </div>
    </div>
  </ToolShell>
</template>
