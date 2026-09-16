<!--
  非对称面板：RSA 与 SM2 的密钥生成、加解密、签名/验签。
-->
<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import Panel from '@/components/tool/Panel.vue';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { KeyRound, Play } from '@lucide/vue';
import { ipc } from '@/core/ipc';
import { errorMessage } from '../../shared';
import { BYTE_FORMATS, type ByteFormat } from '../../crypto-shared';
import CopyableTextarea from './CopyableTextarea.vue';
import ResultBox from './ResultBox.vue';

interface KeyPair {
  privateKey?: string;
  publicKey?: string;
  // 兼容潜在的下划线序列化
  private_key?: string;
  public_key?: string;
}

type Operation = 'generate' | 'encrypt' | 'decrypt' | 'sign' | 'verify';

const algorithm = ref<'rsa' | 'sm2'>('rsa');
const operation = ref<Operation>('generate');

const rsaBits = ref(2048);
const rsaFormat = ref('pkcs8');
const rsaPadding = ref('oaep');
const hash = ref('sha256');

const sm2Format = ref('asn1');
const signerId = ref('');

const privateKey = ref('');
const publicKey = ref('');
const data = ref('');
const dataEncoding = ref<ByteFormat>('utf8');
const output = ref<ByteFormat>('hex');
const signature = ref('');
const signatureEncoding = ref<ByteFormat>('hex');

const result = ref('');
const verifyResult = ref<boolean | null>(null);
const error = ref('');
const loading = ref(false);

const isSignatureOp = computed(() => operation.value === 'sign' || operation.value === 'verify');
const paddingOptions = computed(() =>
  isSignatureOp.value
    ? [
        { value: 'pss', label: 'PSS' },
        { value: 'pkcs1v15', label: 'PKCS#1 v1.5' },
      ]
    : [
        { value: 'oaep', label: 'OAEP' },
        { value: 'pkcs1v15', label: 'PKCS#1 v1.5' },
      ]
);

/** 结果面板内容：验签时展示结论，其余展示实际输出。 */
const resultDisplay = computed(() => {
  if (operation.value === 'verify') {
    if (verifyResult.value === null) return '';
    return verifyResult.value ? '验签通过' : '验签失败';
  }
  return result.value;
});
const resultTone = computed<'default' | 'success' | 'danger'>(() => {
  if (operation.value !== 'verify' || verifyResult.value === null) return 'default';
  return verifyResult.value ? 'success' : 'danger';
});

watch(algorithm, () => {
  operation.value = 'generate';
  result.value = '';
  verifyResult.value = null;
  error.value = '';
  rsaPadding.value = 'oaep';
});
watch(operation, (op) => {
  result.value = '';
  verifyResult.value = null;
  error.value = '';
  rsaPadding.value = op === 'sign' || op === 'verify' ? 'pss' : 'oaep';
  dataEncoding.value = op === 'decrypt' ? 'hex' : 'utf8';
  output.value = op === 'decrypt' ? 'utf8' : 'hex';
});

function keyForOperation(): string {
  return operation.value === 'decrypt' || operation.value === 'sign'
    ? privateKey.value
    : publicKey.value;
}

async function run(): Promise<void> {
  error.value = '';
  verifyResult.value = null;
  result.value = '';

  // 生成密钥对
  if (operation.value === 'generate') {
    loading.value = true;
    try {
      const kp =
        algorithm.value === 'rsa'
          ? await ipc<KeyPair>('daily_tools_rsa_generate', {
              bits: rsaBits.value,
              format: rsaFormat.value,
            })
          : await ipc<KeyPair>('daily_tools_sm2_generate');
      const privatePem = kp.privateKey ?? kp.private_key ?? '';
      const publicPem = kp.publicKey ?? kp.public_key ?? '';
      if (!privatePem || !publicPem) {
        error.value = '密钥对生成结果为空，请重试';
        return;
      }
      privateKey.value = privatePem;
      publicKey.value = publicPem;
      result.value = '密钥对已生成';
    } catch (err) {
      error.value = errorMessage(err);
    } finally {
      loading.value = false;
    }
    return;
  }

  // 必填项校验（给出通俗提示，而不是后端参数错误）
  const needPrivate = operation.value === 'decrypt' || operation.value === 'sign';
  const key = keyForOperation();
  if (!key.trim()) {
    error.value = `请先粘贴或生成${needPrivate ? '私钥' : '公钥'}（PEM 格式）`;
    return;
  }
  if (!data.value.trim()) {
    error.value = '请输入待处理的数据';
    return;
  }
  if (operation.value === 'verify' && !signature.value.trim()) {
    error.value = '请填写待验签的签名';
    return;
  }

  loading.value = true;
  try {
    const base = {
      key,
      data: data.value,
      dataEncoding: dataEncoding.value,
      output: output.value,
    };

    if (algorithm.value === 'rsa') {
      if (operation.value === 'encrypt') {
        result.value = await ipc<string>('daily_tools_rsa_encrypt', {
          req: { ...base, padding: rsaPadding.value, hash: hash.value },
        });
      } else if (operation.value === 'decrypt') {
        result.value = await ipc<string>('daily_tools_rsa_decrypt', {
          req: { ...base, padding: rsaPadding.value, hash: hash.value },
        });
      } else if (operation.value === 'sign') {
        result.value = await ipc<string>('daily_tools_rsa_sign', {
          req: { ...base, padding: rsaPadding.value, hash: hash.value },
        });
      } else {
        verifyResult.value = await ipc<boolean>('daily_tools_rsa_verify', {
          req: {
            ...base,
            padding: rsaPadding.value,
            hash: hash.value,
            signature: signature.value,
            signatureEncoding: signatureEncoding.value,
          },
        });
      }
    } else {
      if (operation.value === 'encrypt') {
        result.value = await ipc<string>('daily_tools_sm2_encrypt', {
          req: { ...base, format: sm2Format.value },
        });
      } else if (operation.value === 'decrypt') {
        result.value = await ipc<string>('daily_tools_sm2_decrypt', {
          req: { ...base, format: sm2Format.value },
        });
      } else if (operation.value === 'sign') {
        result.value = await ipc<string>('daily_tools_sm2_sign', {
          req: { ...base, id: signerId.value },
        });
      } else {
        verifyResult.value = await ipc<boolean>('daily_tools_sm2_verify', {
          req: {
            ...base,
            id: signerId.value,
            signature: signature.value,
            signatureEncoding: signatureEncoding.value,
          },
        });
      }
    }
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="grid w-full grid-cols-12 gap-4">
    <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
      <Panel body-class="space-y-4">
        <div class="flex flex-wrap items-center gap-3">
          <div class="inline-flex rounded-lg bg-muted p-0.5 text-sm">
            <button
              type="button"
              class="rounded-md px-3 py-1 transition-colors"
              :class="
                algorithm === 'rsa'
                  ? 'bg-primary text-primary-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              "
              @click="algorithm = 'rsa'"
            >
              RSA
            </button>
            <button
              type="button"
              class="rounded-md px-3 py-1 transition-colors"
              :class="
                algorithm === 'sm2'
                  ? 'bg-primary text-primary-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              "
              @click="algorithm = 'sm2'"
            >
              SM2（国密）
            </button>
          </div>
          <Select v-model="operation">
            <SelectTrigger class="h-9 w-40"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="generate">生成密钥对</SelectItem>
              <SelectItem value="encrypt">加密</SelectItem>
              <SelectItem value="decrypt">解密</SelectItem>
              <SelectItem value="sign">签名</SelectItem>
              <SelectItem value="verify">验签</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <!-- 生成参数 -->
        <div v-if="operation === 'generate'" class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <template v-if="algorithm === 'rsa'">
            <div class="space-y-1.5">
              <Label>密钥位数</Label>
              <Select v-model.number="rsaBits">
                <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem :value="1024">1024</SelectItem>
                  <SelectItem :value="2048">2048</SelectItem>
                  <SelectItem :value="3072">3072</SelectItem>
                  <SelectItem :value="4096">4096</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div class="space-y-1.5">
              <Label>格式</Label>
              <Select v-model="rsaFormat">
                <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="pkcs8">PKCS#8 / SPKI</SelectItem>
                  <SelectItem value="pkcs1">PKCS#1（传统）</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </template>
          <template v-else>
            <p class="self-end pb-2 text-xs text-muted-foreground">
              SM2 密钥固定为 PKCS#8 / SPKI PEM
            </p>
          </template>
        </div>

        <!-- 加密/签名参数 -->
        <div v-else class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <template v-if="algorithm === 'rsa'">
            <div class="space-y-1.5">
              <Label>填充</Label>
              <Select v-model="rsaPadding">
                <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem v-for="item in paddingOptions" :key="item.value" :value="item.value">
                    {{ item.label }}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div class="space-y-1.5">
              <Label>哈希</Label>
              <Select v-model="hash">
                <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="sha256">SHA-256</SelectItem>
                  <SelectItem value="sha1">SHA-1</SelectItem>
                  <SelectItem value="sha384">SHA-384</SelectItem>
                  <SelectItem value="sha512">SHA-512</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </template>
          <template v-else>
            <div v-if="operation === 'encrypt' || operation === 'decrypt'" class="space-y-1.5">
              <Label>密文格式</Label>
              <Select v-model="sm2Format">
                <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="asn1">ASN.1 DER</SelectItem>
                  <SelectItem value="raw">raw C1C3C2</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div v-if="isSignatureOp" class="space-y-1.5">
              <Label>签名者 ID</Label>
              <Input v-model="signerId" placeholder="留空使用默认 ID" />
            </div>
          </template>
        </div>

        <Button :disabled="loading" @click="run">
          <KeyRound v-if="operation === 'generate'" class="mr-1 size-3.5" />
          <Play v-else class="mr-1 size-3.5" />
          {{ operation === 'generate' ? '生成密钥对' : '执行' }}
        </Button>
        <p v-if="operation === 'generate' && error" class="text-sm text-destructive">
          {{ error }}
        </p>
        <p v-else-if="operation === 'generate' && result" class="text-sm text-success">
          {{ result }}（密钥见下方，可直接复制）
        </p>
      </Panel>

      <!-- 密钥输入（生成后也可直接复制） -->
      <div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
        <CopyableTextarea
          v-model="privateKey"
          :label="`私钥 PEM${operation === 'decrypt' || operation === 'sign' ? '（本操作使用）' : ''}`"
          placeholder="-----BEGIN PRIVATE KEY-----"
        />
        <CopyableTextarea
          v-model="publicKey"
          :label="`公钥 PEM${operation === 'encrypt' || operation === 'verify' ? '（本操作使用）' : ''}`"
          placeholder="-----BEGIN PUBLIC KEY-----"
        />
      </div>

      <!-- 数据与签名 -->
      <Panel v-if="operation !== 'generate'" body-class="space-y-3">
        <Textarea
          v-model="data"
          class="h-40 resize-y overflow-y-auto font-mono text-sm"
          style="field-sizing: fixed"
          placeholder="输入数据…"
        />
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div class="space-y-1.5">
            <Label>数据格式</Label>
            <Select v-model="dataEncoding">
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
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <div v-if="operation === 'verify'" class="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <div class="space-y-1.5 sm:col-span-2">
            <Label>签名</Label>
            <Input v-model="signature" class="font-mono" placeholder="待验签数据" />
          </div>
          <div class="space-y-1.5">
            <Label>签名格式</Label>
            <Select v-model="signatureEncoding">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in BYTE_FORMATS" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <ResultBox
          :value="resultDisplay"
          :loading="loading"
          :label="operation === 'verify' ? '验签结果' : '结果'"
          :tone="resultTone"
        />
        <p v-if="error" class="text-sm text-destructive">{{ error }}</p>
      </Panel>
    </div>
  </div>
</template>
