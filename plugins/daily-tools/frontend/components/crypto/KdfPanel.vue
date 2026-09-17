<!--
  口令派生面板：PBKDF2 / scrypt / Argon2id。
-->
<script setup lang="ts">
import { ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Play } from '@lucide/vue';
import Panel from '@/components/tool/Panel.vue';
import { useCryptoCall } from '../../composables/useCryptoCall';
import { BYTE_FORMATS, type ByteFormat } from '../../lib/crypto-shared';
import ResultBox from './ResultBox.vue';

const algorithm = ref('pbkdf2');
const password = ref('');
const salt = ref('');
const saltEncoding = ref<ByteFormat>('utf8');
const length = ref(32);
const output = ref<ByteFormat>('hex');
const rounds = ref(10000);
const logN = ref(15);
const r = ref(8);
const p = ref(1);
const memoryKib = ref(19456);
const iterations = ref(2);
const parallelism = ref(1);

const call = useCryptoCall<string>('daily_tools_kdf');

watch(algorithm, () => call.reset());

async function run(): Promise<void> {
  await call.run({
    req: {
      password: password.value,
      salt: salt.value,
      saltEncoding: saltEncoding.value,
      algorithm: algorithm.value,
      length: length.value,
      output: output.value,
      rounds: rounds.value,
      logN: logN.value,
      r: r.value,
      p: p.value,
      memoryKib: memoryKib.value,
      iterations: iterations.value,
      parallelism: parallelism.value,
    },
  });
}
</script>

<template>
  <div class="grid w-full grid-cols-12 gap-4">
    <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
      <Panel body-class="space-y-4">
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <div class="space-y-1.5">
            <Label>算法</Label>
            <Select v-model="algorithm">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="pbkdf2">PBKDF2-HMAC-SHA256</SelectItem>
                <SelectItem value="scrypt">scrypt</SelectItem>
                <SelectItem value="argon2">Argon2id</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>输出长度（字节）</Label>
            <Input v-model.number="length" type="number" min="1" max="1024" />
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

        <div class="space-y-1.5">
          <Label>口令</Label>
          <Input v-model="password" type="password" placeholder="口令" />
        </div>

        <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <div class="space-y-1.5 sm:col-span-2">
            <Label>Salt</Label>
            <Input v-model="salt" class="font-mono" placeholder="salt" />
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
        </div>

        <!-- PBKDF2 -->
        <div v-if="algorithm === 'pbkdf2'" class="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <div class="space-y-1.5">
            <Label>迭代次数</Label>
            <Input v-model.number="rounds" type="number" min="1" />
          </div>
        </div>

        <!-- scrypt -->
        <div v-else-if="algorithm === 'scrypt'" class="grid grid-cols-3 gap-3">
          <div class="space-y-1.5">
            <Label>log₂(N)</Label>
            <Input v-model.number="logN" type="number" min="1" max="31" />
          </div>
          <div class="space-y-1.5">
            <Label>r</Label>
            <Input v-model.number="r" type="number" min="1" />
          </div>
          <div class="space-y-1.5">
            <Label>p</Label>
            <Input v-model.number="p" type="number" min="1" />
          </div>
        </div>

        <!-- Argon2 -->
        <div v-else class="grid grid-cols-3 gap-3">
          <div class="space-y-1.5">
            <Label>内存（KiB）</Label>
            <Input v-model.number="memoryKib" type="number" min="8" />
          </div>
          <div class="space-y-1.5">
            <Label>迭代次数</Label>
            <Input v-model.number="iterations" type="number" min="1" />
          </div>
          <div class="space-y-1.5">
            <Label>并行度</Label>
            <Input v-model.number="parallelism" type="number" min="1" />
          </div>
        </div>

        <Button :disabled="!password" @click="run">
          <Play class="mr-1 size-3.5" />
          派生密钥
        </Button>

        <ResultBox
          :value="call.result.value ?? ''"
          :loading="call.loading.value"
          label="派生结果"
        />
        <p v-if="call.error.value" class="text-sm text-destructive">{{ call.error.value }}</p>
      </Panel>
    </div>
  </div>
</template>
