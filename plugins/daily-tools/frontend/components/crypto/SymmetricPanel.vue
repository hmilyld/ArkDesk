<!--
  对称面板：AES / SM4 / DES / 3DES，支持原始密钥与 OpenSSL 口令模式，文本与文件。
-->
<script setup lang="ts">
import { computed, ref, watch } from 'vue';
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
import { open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { AlertTriangle, Dices, FileUp, Play, Save } from '@lucide/vue';
import { ipc } from '@/core/ipc';
import { useCryptoCall } from '../../composables/useCryptoCall';
import { errorMessage } from '../../shared';
import {
  BYTE_FORMATS,
  KEY_DERIVATIONS,
  SYMMETRIC_ALGORITHMS,
  SYMMETRIC_MODES,
  type ByteFormat,
} from '../../crypto-shared';
import ResultBox from './ResultBox.vue';
import TaskProgress from './TaskProgress.vue';

interface SymmetricKey {
  key: string;
  iv: string;
}

const operation = ref<'encrypt' | 'decrypt'>('encrypt');
const algorithm = ref('aes-256');
const mode = ref('cbc');
const keyMode = ref<'raw' | 'passphrase'>('raw');
const key = ref('');
const keyEncoding = ref<ByteFormat>('hex');
const iv = ref('');
const ivEncoding = ref<ByteFormat>('hex');
const passphrase = ref('');
const kdf = ref('evp');
const rounds = ref(10000);
const salt = ref('');
const saltEncoding = ref<ByteFormat>('hex');
const data = ref('');
const dataEncoding = ref<ByteFormat>('utf8');
const output = ref<ByteFormat>('hex');
const aad = ref('');
const aadEncoding = ref<ByteFormat>('utf8');

const isAes = computed(() => algorithm.value.startsWith('aes'));
const isGcm = computed(() => mode.value === 'gcm');
const isEcb = computed(() => mode.value === 'ecb');
const needsIv = computed(() => !isEcb.value);
const legacy = computed(
  () => SYMMETRIC_ALGORITHMS.find((item) => item.value === algorithm.value)?.legacy === true
);

watch([algorithm, mode], () => {
  if (mode.value === 'gcm' && !isAes.value) mode.value = 'cbc';
  if (keyMode.value === 'passphrase' && mode.value === 'gcm') mode.value = 'cbc';
  if (operation.value === 'decrypt' && output.value === 'hex') output.value = 'utf8';
});
watch(operation, (op) => {
  call.reset();
  output.value = op === 'encrypt' ? 'hex' : 'utf8';
  // 解密输入通常是密文编码，默认 base64；加密输入是文本
  dataEncoding.value = op === 'encrypt' ? 'utf8' : 'base64';
});
watch(keyMode, (value) => {
  if (value === 'passphrase' && mode.value === 'gcm') mode.value = 'cbc';
});
watch(mode, () => {
  // 不同模式 IV/nonce 长度不同，切换时清空避免误用
  if (keyMode.value === 'raw') iv.value = '';
});

const call = useCryptoCall<string>('daily_tools_symmetric');

async function generateKey(): Promise<void> {
  // 随机密钥是二进制，不能用 utf8 展示
  const encoding = keyEncoding.value === 'utf8' ? 'hex' : keyEncoding.value;
  if (encoding !== keyEncoding.value) keyEncoding.value = encoding;
  try {
    const result = await ipc<SymmetricKey>('daily_tools_symmetric_generate', {
      algorithm: algorithm.value,
      encoding,
      mode: mode.value,
    });
    key.value = result.key;
    iv.value = result.iv;
    toast.success('已生成随机密钥与 IV');
  } catch (err) {
    toast.error(`生成密钥失败：${errorMessage(err)}`);
  }
}

function commonArgs(): Record<string, unknown> {
  return {
    operation: operation.value,
    algorithm: algorithm.value,
    mode: mode.value,
    key: keyMode.value === 'raw' ? key.value : undefined,
    keyEncoding: keyEncoding.value,
    iv: keyMode.value === 'raw' && needsIv.value ? iv.value : undefined,
    ivEncoding: ivEncoding.value,
    passphrase: keyMode.value === 'passphrase' ? passphrase.value : undefined,
    kdf: kdf.value,
    salt: keyMode.value === 'passphrase' && salt.value ? salt.value : undefined,
    saltEncoding: saltEncoding.value,
    rounds: rounds.value,
  };
}

async function runText(): Promise<void> {
  await call.run({
    req: {
      ...commonArgs(),
      data: data.value,
      dataEncoding: dataEncoding.value,
      output: output.value,
      aad: isGcm.value ? aad.value : undefined,
      aadEncoding: aadEncoding.value,
    },
  });
}

// ── 文件 ──────────────────────────────────────────────────────
const FILE_TASK_ID = 'crypto-symmetric-file';
const inputPath = ref('');
const inputName = ref('');
const fileResult = ref('');
const fileCall = useCryptoCall<null>('daily_tools_symmetric_file');

async function pickInput(): Promise<void> {
  const path = await openFileDialog({ multiple: false });
  if (!path) return;
  inputPath.value = String(path);
  inputName.value = inputPath.value.split(/[/\\]/).pop() ?? '';
  fileResult.value = '';
}

async function runFile(): Promise<void> {
  if (!inputPath.value) {
    toast.error('请先选择输入文件');
    return;
  }
  const defaultName =
    operation.value === 'encrypt'
      ? `${inputName.value}.enc`
      : inputName.value.replace(/\.enc$/, '') || `${inputName.value}.dec`;
  const path = await saveFileDialog({ defaultPath: defaultName });
  if (!path) return;

  fileResult.value = '';
  await fileCall.run({
    taskId: FILE_TASK_ID,
    req: {
      ...commonArgs(),
      inputPath: inputPath.value,
      outputPath: String(path),
    },
  });
  if (!fileCall.error.value) {
    fileResult.value = String(path);
    toast.success('处理完成');
  } else if (fileCall.error.value.includes('已取消')) {
    fileCall.reset();
    toast.info('已取消');
  } else {
    toast.error(fileCall.error.value);
  }
}
</script>

<template>
  <div class="grid w-full grid-cols-12 gap-4">
    <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
      <!-- 操作与参数 -->
      <div class="space-y-4 rounded-lg border bg-card p-4">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div class="inline-flex rounded-lg bg-muted p-0.5 text-sm">
            <button
              type="button"
              class="rounded-md px-3 py-1 transition-colors"
              :class="
                operation === 'encrypt'
                  ? 'bg-primary text-primary-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              "
              @click="operation = 'encrypt'"
            >
              加密
            </button>
            <button
              type="button"
              class="rounded-md px-3 py-1 transition-colors"
              :class="
                operation === 'decrypt'
                  ? 'bg-primary text-primary-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              "
              @click="operation = 'decrypt'"
            >
              解密
            </button>
          </div>
          <Button
            variant="ghost"
            size="sm"
            @click="keyMode = keyMode === 'raw' ? 'passphrase' : 'raw'"
          >
            密钥方式：{{ keyMode === 'raw' ? '原始密钥' : '口令' }}
          </Button>
        </div>

        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <div class="space-y-1.5">
            <Label>算法</Label>
            <Select v-model="algorithm">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="item in SYMMETRIC_ALGORITHMS"
                  :key="item.value"
                  :value="item.value"
                >
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>模式</Label>
            <Select v-model="mode">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="item in SYMMETRIC_MODES"
                  :key="item.value"
                  :value="item.value"
                  :disabled="item.value === 'gcm' && !isAes"
                >
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>{{ operation === 'encrypt' ? '输出' : '输入' }}格式</Label>
            <Select v-if="operation === 'encrypt'" v-model="output">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
            <Select v-else v-model="dataEncoding">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <!-- 原始密钥 -->
        <div v-if="keyMode === 'raw'" class="space-y-3">
          <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div class="space-y-1.5 sm:col-span-2">
              <Label>密钥</Label>
              <Input v-model="key" class="font-mono" :placeholder="`密钥（${keyEncoding}）`" />
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
          <div v-if="needsIv" class="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div class="space-y-1.5 sm:col-span-2">
              <Label>{{ isGcm ? 'Nonce' : 'IV' }}</Label>
              <Input v-model="iv" class="font-mono" :placeholder="isGcm ? '12 字节' : 'IV'" />
            </div>
            <div class="space-y-1.5">
              <Label>IV 格式</Label>
              <Select v-model="ivEncoding">
                <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                    {{ item.label }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <Button variant="secondary" size="sm" @click="generateKey">
            <Dices class="mr-1 size-3.5" />
            生成随机密钥与 IV
          </Button>
        </div>

        <!-- 口令模式 -->
        <div v-else class="space-y-3">
          <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div class="space-y-1.5 sm:col-span-2">
              <Label>口令</Label>
              <Input v-model="passphrase" type="password" placeholder="口令" />
            </div>
            <div class="space-y-1.5">
              <Label>KDF</Label>
              <Select v-model="kdf">
                <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem v-for="item in KEY_DERIVATIONS" :key="item.value" :value="item.value">
                    {{ item.label }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div v-if="kdf === 'pbkdf2'" class="space-y-1.5">
              <Label>迭代次数</Label>
              <Input v-model.number="rounds" type="number" min="1" />
            </div>
            <template v-if="operation === 'encrypt'">
              <div class="space-y-1.5 sm:col-span-2">
                <Label>Salt（可选，留空随机 8 字节）</Label>
                <Input v-model="salt" class="font-mono" placeholder="hex / base64" />
              </div>
              <div class="space-y-1.5">
                <Label>Salt 格式</Label>
                <Select v-model="saltEncoding">
                  <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
                  <SelectContent>
                    <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                      {{ item.label }}
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </template>
          </div>
          <p class="text-xs text-muted-foreground">
            口令模式输出带 <code>Salted__</code> 头，兼容 <code>openssl enc</code>；GCM
            不支持口令模式。
          </p>
        </div>

        <div v-if="isGcm" class="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <div class="space-y-1.5 sm:col-span-2">
            <Label>AAD（可选）</Label>
            <Input v-model="aad" class="font-mono" placeholder="附加认证数据" />
          </div>
          <div class="space-y-1.5">
            <Label>AAD 格式</Label>
            <Select v-model="aadEncoding">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <p v-if="legacy" class="flex items-center gap-1.5 text-xs text-warning">
          <AlertTriangle class="size-3.5" />
          DES/3DES 已不安全，仅用于兼容旧系统。
        </p>
      </div>

      <!-- 文本 -->
      <div class="space-y-3 rounded-lg border bg-card p-4">
        <Textarea
          v-model="data"
          class="h-40 resize-y overflow-y-auto font-mono text-sm"
          style="field-sizing: fixed"
          :placeholder="operation === 'encrypt' ? '输入待加密文本…' : '输入待解密的密文…'"
        />
        <div class="flex items-center gap-3">
          <Button :disabled="!data" @click="runText">
            <Play class="mr-1 size-3.5" />
            {{ operation === 'encrypt' ? '加密' : '解密' }}
          </Button>
        </div>
        <ResultBox
          :value="call.result.value ?? ''"
          :loading="call.loading.value"
          :label="operation === 'encrypt' ? '密文' : '明文'"
        />
        <p v-if="call.error.value" class="text-sm text-destructive">{{ call.error.value }}</p>
      </div>

      <!-- 文件 -->
      <div class="space-y-3 rounded-lg border bg-card p-4">
        <div>
          <p class="text-sm font-medium">文件加解密（流式）</p>
          <p class="text-xs text-muted-foreground">使用上方算法/密钥参数；GCM 不支持文件。</p>
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <Button variant="secondary" size="sm" @click="pickInput">
            <FileUp class="mr-1 size-3.5" />
            选择文件
          </Button>
          <span v-if="inputName" class="truncate text-xs text-muted-foreground">{{
            inputName
          }}</span>
          <Button size="sm" :disabled="!inputPath || isGcm" @click="runFile">
            <Save class="mr-1 size-3.5" />
            处理并另存为…
          </Button>
        </div>
        <p v-if="isGcm" class="text-xs text-warning">GCM 无法流式处理文件，请改用 CBC/CTR。</p>
        <TaskProgress :task-id="FILE_TASK_ID" />
        <ResultBox
          v-if="fileResult"
          :value="fileResult"
          :loading="fileCall.loading.value"
          label="输出文件"
        />
        <p v-if="fileCall.error.value" class="text-sm text-destructive">
          {{ fileCall.error.value }}
        </p>
      </div>
    </div>
  </div>
</template>
