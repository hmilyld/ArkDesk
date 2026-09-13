<!--
  摘要面板：MD5 / SHA / SHA3 / SM3 / CRC32 与 HMAC，支持文本与文件。
-->
<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { AlertTriangle, FileUp, Fingerprint } from '@lucide/vue';
import { useCryptoCall } from '../../composables/useCryptoCall';
import {
  BYTE_FORMATS,
  BYTE_LABELS,
  DIGEST_OUTPUTS,
  HASH_ALGORITHMS,
  HMAC_ALGORITHMS,
  type ByteFormat,
} from '../../crypto-shared';
import ResultBox from './ResultBox.vue';
import TaskProgress from './TaskProgress.vue';

const mode = ref<'hash' | 'hmac'>('hash');
const text = ref('');
const algorithm = ref('sha256');
const inputEncoding = ref<ByteFormat>('utf8');
const output = ref('hex');
const key = ref('');
const keyEncoding = ref<ByteFormat>('utf8');

const algorithms = computed(() => (mode.value === 'hash' ? HASH_ALGORITHMS : HMAC_ALGORITHMS));
const legacy = computed(
  () => algorithms.value.find((item) => item.value === algorithm.value)?.legacy === true
);

/** 输入占位：随输入格式变化，避免把普通文本当成 Hex/Base64 粘贴。 */
const inputPlaceholder = computed(() => {
  switch (inputEncoding.value) {
    case 'hex':
      return '输入十六进制字符（0-9、a-f），如 68656c6c6f';
    case 'base64':
      return '输入 Base64 内容…';
    default:
      return mode.value === 'hmac' ? '输入待计算 HMAC 的文本…' : '输入待计算的文本，或粘贴内容…';
  }
});

const hashCall = useCryptoCall<string>('daily_tools_hash');
const hmacCall = useCryptoCall<string>('daily_tools_hmac');
const active = computed(() => (mode.value === 'hash' ? hashCall : hmacCall));
const result = computed(() => active.value.result.value ?? '');
const loading = computed(() => active.value.loading.value);
const error = computed(() => active.value.error.value);

let timer: ReturnType<typeof setTimeout> | undefined;

function schedule(): void {
  if (timer) clearTimeout(timer);
  if (!text.value) {
    hashCall.reset();
    hmacCall.reset();
    return;
  }
  timer = setTimeout(() => void run(), 250);
}

async function run(): Promise<void> {
  if (!text.value) return;
  if (mode.value === 'hash') {
    await hashCall.run({
      data: text.value,
      algorithm: algorithm.value,
      inputEncoding: inputEncoding.value,
      output: output.value,
    });
  } else {
    await hmacCall.run({
      data: text.value,
      algorithm: algorithm.value,
      key: key.value,
      keyEncoding: keyEncoding.value,
      inputEncoding: inputEncoding.value,
      output: output.value,
    });
  }
}

watch([text, mode, algorithm, inputEncoding, output, key, keyEncoding], schedule);

watch(mode, () => {
  if (!algorithms.value.some((item) => item.value === algorithm.value)) {
    algorithm.value = 'sha256';
  }
  schedule();
});

onUnmounted(() => {
  if (timer) clearTimeout(timer);
});

// ── 文件哈希 ──────────────────────────────────────────────────
const FILE_TASK_ID = 'crypto-hash-file';
const filePath = ref('');
const fileName = ref('');
const fileCall = useCryptoCall<string>('daily_tools_hash_file');

async function pickFile(): Promise<void> {
  const path = await openFileDialog({ multiple: false });
  if (!path) return;
  filePath.value = String(path);
  fileName.value = filePath.value.split(/[/\\]/).pop() ?? '';
  await fileCall.run({
    taskId: FILE_TASK_ID,
    path: filePath.value,
    algorithm: algorithm.value,
    output: output.value,
  });
  if (fileCall.error.value) {
    if (fileCall.error.value.includes('已取消')) {
      fileCall.reset();
      toast.info('已取消');
    } else {
      toast.error(`文件哈希失败：${fileCall.error.value}`);
    }
  }
}
</script>

<template>
  <div class="grid w-full grid-cols-12 gap-4">
    <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
      <!-- 哈希 / HMAC -->
      <div class="space-y-4 rounded-lg border bg-card p-4">
        <!-- 模式切换 -->
        <div class="inline-flex rounded-lg bg-muted p-0.5 text-sm">
          <button
            type="button"
            class="rounded-md px-3 py-1 transition-colors"
            :class="
              mode === 'hash'
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            "
            @click="mode = 'hash'"
          >
            哈希
          </button>
          <button
            type="button"
            class="rounded-md px-3 py-1 transition-colors"
            :class="
              mode === 'hmac'
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            "
            @click="mode = 'hmac'"
          >
            HMAC
          </button>
        </div>

        <Textarea
          v-model="text"
          class="h-40 resize-y overflow-y-auto font-mono text-sm"
          style="field-sizing: fixed"
          :placeholder="inputPlaceholder"
        />

        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <div class="space-y-1.5">
            <Label>算法</Label>
            <Select v-model="algorithm">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in algorithms" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>输入内容格式</Label>
            <Select v-model="inputEncoding">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>输出格式</Label>
            <Select v-model="output">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in DIGEST_OUTPUTS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div v-if="mode === 'hmac'" class="grid grid-cols-2 gap-3">
          <div class="space-y-1.5">
            <Label>密钥</Label>
            <Input v-model="key" class="font-mono" placeholder="HMAC 密钥" />
          </div>
          <div class="space-y-1.5">
            <Label>密钥格式</Label>
            <Select v-model="keyEncoding">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <p v-if="inputEncoding !== 'utf8'" class="text-xs text-warning">
          当前「输入内容格式」为「{{
            BYTE_LABELS[inputEncoding]
          }}」，请粘贴该格式的内容；若要处理中文等普通文本，请改为「文本 (UTF-8)」。
        </p>

        <p v-if="legacy" class="flex items-center gap-1.5 text-xs text-warning">
          <AlertTriangle class="size-3.5" />
          该算法已不安全，仅用于兼容或校验，请勿用于安全场景。
        </p>

        <ResultBox
          :value="result"
          :loading="loading"
          label="摘要结果"
          placeholder="输入文本后自动计算"
        />
        <p v-if="error" class="text-sm text-destructive">{{ error }}</p>
      </div>

      <!-- 文件哈希 -->
      <div class="space-y-3 rounded-lg border bg-card p-4">
        <div class="flex items-center justify-between gap-3">
          <div>
            <p class="text-sm font-medium">对文件计算哈希</p>
            <p class="text-xs text-muted-foreground">流式读取，支持大文件</p>
          </div>
          <Button
            variant="secondary"
            size="sm"
            :disabled="fileCall.loading.value"
            @click="pickFile"
          >
            <FileUp class="mr-1 size-3.5" />
            选择文件
          </Button>
        </div>
        <div v-if="fileName" class="flex items-center gap-2 text-xs text-muted-foreground">
          <Fingerprint class="size-3.5 shrink-0" />
          <span class="truncate">{{ fileName }}</span>
        </div>
        <TaskProgress :task-id="FILE_TASK_ID" />
        <ResultBox
          v-if="fileName"
          :value="fileCall.result.value ?? ''"
          :loading="fileCall.loading.value"
          label="文件摘要"
        />
        <p v-if="fileCall.error.value" class="text-sm text-destructive">
          {{ fileCall.error.value }}
        </p>
      </div>
    </div>
  </div>
</template>
