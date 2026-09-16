<!--
  编码面板：Base64 / Base32 / Hex / URL 编解码 + 文本字符集转换 + 文件编码。
-->
<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { toast } from 'vue-sonner';
import { ArrowLeftRight, FileUp, Save } from '@lucide/vue';
import Panel from '@/components/tool/Panel.vue';
import Segmented from '@/components/native/Segmented.vue';
import { useCryptoCall } from '../../composables/useCryptoCall';
import {
  BYTE_FORMATS,
  BYTE_LABELS,
  ENCODE_SCHEMES,
  TEXT_ENCODINGS,
  type ByteFormat,
} from '../../crypto-shared';
import ResultBox from './ResultBox.vue';
import TaskProgress from './TaskProgress.vue';

// ── 编解码 ────────────────────────────────────────────────────
const mode = ref<'encode' | 'decode'>('encode');
const text = ref('');
const scheme = ref('base64');
const padding = ref(true);
const uppercase = ref(false);
const inputEncoding = ref<ByteFormat>('utf8');
const outputEncoding = ref<ByteFormat>('utf8');

const isHex = computed(() => scheme.value === 'hex');
const isUrl = computed(() => scheme.value === 'url');
const supportsPadding = computed(() => ['base64', 'base64url', 'base32'].includes(scheme.value));

const SCHEME_LABELS: Record<string, string> = {
  base64: 'Base64',
  base64url: 'Base64 URL-safe',
  base32: 'Base32',
  hex: 'Hex',
  url: 'URL',
};

/** 输入占位：随模式与输入格式变化，避免把普通文本当成 Hex/Base64 粘贴。 */
const inputPlaceholder = computed(() => {
  if (mode.value === 'decode') {
    return `输入待解码的 ${SCHEME_LABELS[scheme.value] ?? scheme.value} 内容…`;
  }
  switch (inputEncoding.value) {
    case 'hex':
      return '输入十六进制字符（0-9、a-f），如 48656c6c6f';
    case 'base64':
      return '输入 Base64 内容…';
    default:
      return '输入待编码的文本…';
  }
});

const encodeCall = useCryptoCall<string>('daily_tools_encode');
const decodeCall = useCryptoCall<string>('daily_tools_decode');
const active = computed(() => (mode.value === 'encode' ? encodeCall : decodeCall));
const result = computed(() => active.value.result.value ?? '');
const loading = computed(() => active.value.loading.value);
const error = computed(() => active.value.error.value);

let timer: ReturnType<typeof setTimeout> | undefined;

function schedule(): void {
  if (timer) clearTimeout(timer);
  if (!text.value) {
    encodeCall.reset();
    decodeCall.reset();
    return;
  }
  timer = setTimeout(() => void run(), 200);
}

async function run(): Promise<void> {
  if (!text.value) return;
  if (mode.value === 'encode') {
    await encodeCall.run({
      data: text.value,
      scheme: scheme.value,
      inputEncoding: inputEncoding.value,
      padding: padding.value,
      uppercase: uppercase.value,
    });
  } else {
    await decodeCall.run({
      data: text.value,
      scheme: scheme.value,
      output: outputEncoding.value,
    });
  }
}

watch([text, mode, scheme, padding, uppercase, inputEncoding, outputEncoding], schedule);
watch(mode, schedule);
onUnmounted(() => {
  if (timer) clearTimeout(timer);
});

// ── 文本字符集转换 ────────────────────────────────────────────
const tcText = ref('');
const tcFrom = ref('utf-8');
const tcTo = ref('gbk');
const tcInput = ref<ByteFormat>('utf8');
const tcOutput = ref<ByteFormat>('hex');
const tcLossy = ref(false);
const tcCall = useCryptoCall<string>('daily_tools_text_convert');

let tcTimer: ReturnType<typeof setTimeout> | undefined;

function tcSchedule(): void {
  if (tcTimer) clearTimeout(tcTimer);
  if (!tcText.value) {
    tcCall.reset();
    return;
  }
  tcTimer = setTimeout(() => void tcRun(), 250);
}

async function tcRun(): Promise<void> {
  if (!tcText.value) return;
  await tcCall.run({
    data: tcText.value,
    from: tcFrom.value,
    to: tcTo.value,
    inputEncoding: tcInput.value,
    output: tcOutput.value,
    lossy: tcLossy.value,
  });
}

watch([tcText, tcFrom, tcTo, tcInput, tcOutput, tcLossy], tcSchedule);
watch(tcTo, (value) => {
  if (value === 'utf-8' && tcOutput.value === 'hex') tcOutput.value = 'utf8';
  if (value !== 'utf-8' && tcOutput.value === 'utf8') tcOutput.value = 'hex';
});
watch(tcFrom, (value) => {
  if (value === 'utf-8' && tcInput.value === 'hex') tcInput.value = 'utf8';
  if (value !== 'utf-8' && tcInput.value === 'utf8') tcInput.value = 'hex';
});
onUnmounted(() => {
  if (tcTimer) clearTimeout(tcTimer);
});

// ── 文件编码 ──────────────────────────────────────────────────
const PREVIEW_TASK_ID = 'crypto-encode-preview';
const SAVE_TASK_ID = 'crypto-encode-save';
const fileName = ref('');
const filePath = ref('');
const fileCall = useCryptoCall<string>('daily_tools_encode_file');
const saveCall = useCryptoCall<string>('daily_tools_encode_file');
const saveResult = ref('');

async function pickFile(): Promise<void> {
  const path = await openFileDialog({ multiple: false });
  if (!path) return;
  filePath.value = String(path);
  fileName.value = filePath.value.split(/[/\\]/).pop() ?? '';
  fileCall.reset();
  saveCall.reset();
  saveResult.value = '';
}

/** 预览编码（大文件请用「编码并保存」，避免结果驻留内存）。 */
async function previewEncodedFile(): Promise<void> {
  if (!filePath.value) return;
  await fileCall.run({
    taskId: PREVIEW_TASK_ID,
    path: filePath.value,
    scheme: scheme.value,
    padding: padding.value,
    uppercase: uppercase.value,
  });
  if (fileCall.error.value?.includes('已取消')) {
    fileCall.reset();
    toast.info('已取消');
  }
}

/** 大文件：流式编码并直接保存，结果不经 IPC 回传。 */
async function saveEncodedFile(): Promise<void> {
  if (!filePath.value) return;
  const path = await saveFileDialog({
    defaultPath: `${fileName.value}.${scheme.value}`,
  });
  if (!path) return;
  await saveCall.run({
    taskId: SAVE_TASK_ID,
    path: filePath.value,
    scheme: scheme.value,
    padding: padding.value,
    uppercase: uppercase.value,
    outputPath: String(path),
  });
  if (saveCall.error.value) {
    if (saveCall.error.value.includes('已取消')) {
      saveCall.reset();
      toast.info('已取消');
    } else {
      toast.error(saveCall.error.value);
    }
  } else {
    saveResult.value = String(path);
    toast.success('已保存');
  }
}
</script>

<template>
  <div class="grid w-full grid-cols-12 gap-4">
    <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
      <!-- 编解码 -->
      <Panel body-class="space-y-4">
        <div class="flex items-center justify-between">
          <Segmented
            v-model="mode"
            size="sm"
            :segments="[
              { value: 'encode', label: '编码' },
              { value: 'decode', label: '解码' },
            ]"
          />
          <Button variant="ghost" size="sm" @click="mode = mode === 'encode' ? 'decode' : 'encode'">
            <ArrowLeftRight class="mr-1 size-3.5" />
            互换
          </Button>
        </div>

        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div class="space-y-1.5">
            <Label>方式</Label>
            <Select v-model="scheme">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in ENCODE_SCHEMES" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>{{ mode === 'encode' ? '输入内容格式' : '输出内容格式' }}</Label>
            <Select v-if="mode === 'encode'" v-model="inputEncoding">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
            <Select v-else v-model="outputEncoding">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div v-if="supportsPadding" class="flex items-end gap-2 pb-2">
            <Switch v-model="padding" size="sm" />
            <Label class="text-xs text-muted-foreground">补齐 padding</Label>
          </div>
          <div v-if="isHex" class="flex items-end gap-2 pb-2">
            <Switch v-model="uppercase" size="sm" />
            <Label class="text-xs text-muted-foreground">大写</Label>
          </div>
        </div>

        <Textarea
          v-model="text"
          class="h-40 resize-y overflow-y-auto font-mono text-sm"
          style="field-sizing: fixed"
          :placeholder="inputPlaceholder"
        />

        <p v-if="mode === 'encode' && inputEncoding !== 'utf8'" class="text-xs text-warning">
          当前「输入内容格式」为「{{
            BYTE_LABELS[inputEncoding]
          }}」，请粘贴该格式的内容；若要编码中文等普通文本，请改为「文本 (UTF-8)」。
        </p>

        <ResultBox
          :value="result"
          :loading="loading"
          :label="mode === 'encode' ? '编码结果' : '解码结果'"
        />
        <p v-if="error" class="text-sm text-destructive">{{ error }}</p>
      </Panel>

      <!-- 文件编码 -->
      <Panel title="文件编码">
        <template #actions>
          <Button variant="secondary" size="sm" :disabled="isUrl" @click="pickFile">
            <FileUp class="mr-1 size-3.5" />
            选择文件
          </Button>
        </template>
        <p class="text-xs text-muted-foreground">
          以当前「方式」流式编码整个文件（URL 不支持文件）
        </p>
        <p v-if="fileName" class="text-xs text-muted-foreground">{{ fileName }}</p>
        <div v-if="fileName" class="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            size="sm"
            :disabled="isUrl || fileCall.loading.value"
            @click="previewEncodedFile"
          >
            <FileUp class="mr-1 size-3.5" />
            编码预览
          </Button>
          <Button size="sm" :disabled="isUrl || saveCall.loading.value" @click="saveEncodedFile">
            <Save class="mr-1 size-3.5" />
            编码并保存为文件
          </Button>
        </div>
        <TaskProgress v-if="fileCall.loading.value" :task-id="PREVIEW_TASK_ID" />
        <TaskProgress v-if="saveCall.loading.value" :task-id="SAVE_TASK_ID" />
        <ResultBox
          v-if="fileName"
          :value="fileCall.result.value ?? ''"
          :loading="fileCall.loading.value"
          label="文件编码结果"
          max-height="200px"
        />
        <ResultBox
          v-if="saveResult"
          :value="saveResult"
          :loading="saveCall.loading.value"
          label="输出文件"
          max-height="80px"
        />
        <p v-if="fileCall.error.value" class="text-sm text-destructive">
          {{ fileCall.error.value }}
        </p>
        <p v-if="saveCall.error.value" class="text-sm text-destructive">
          {{ saveCall.error.value }}
        </p>
      </Panel>

      <!-- 文本字符集转换 -->
      <Panel title="文本字符集转换" body-class="space-y-4">
        <p class="text-xs text-muted-foreground">
          源/目标非 UTF-8 时，请用 Hex 提供或查看原始字节。
        </p>
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div class="space-y-1.5">
            <Label>源字符集</Label>
            <Select v-model="tcFrom">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in TEXT_ENCODINGS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>目标字符集</Label>
            <Select v-model="tcTo">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in TEXT_ENCODINGS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>输入格式</Label>
            <Select v-model="tcInput">
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
            <Select v-model="tcOutput">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <Switch v-model="tcLossy" size="sm" />
          <Label class="text-xs text-muted-foreground">忽略非法字符（用替换字符容错）</Label>
        </div>
        <Textarea
          v-model="tcText"
          class="h-32 resize-y overflow-y-auto font-mono text-sm"
          style="field-sizing: fixed"
          placeholder="输入待转换内容…"
        />
        <ResultBox
          :value="tcCall.result.value ?? ''"
          :loading="tcCall.loading.value"
          label="转换结果"
        />
        <p v-if="tcCall.error.value" class="text-sm text-destructive">
          {{ tcCall.error.value }}
        </p>
      </Panel>
    </div>
  </div>
</template>
