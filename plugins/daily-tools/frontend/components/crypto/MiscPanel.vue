<!--
  杂项面板：UUID、时间戳转换、Unicode 转义。
-->
<script setup lang="ts">
import { ref } from 'vue';
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
import { Play, Sparkles } from '@lucide/vue';
import { ipc } from '@/core/ipc';
import { errorMessage } from '../../shared';
import ResultBox from './ResultBox.vue';

// ── UUID ──────────────────────────────────────────────────────
const uuidVersion = ref('4');
const uuidCount = ref(5);
const uuidResult = ref('');
const uuidError = ref('');

async function generateUuid(): Promise<void> {
  uuidError.value = '';
  try {
    const list = await ipc<string[]>('daily_tools_uuid', {
      version: Number(uuidVersion.value),
      count: uuidCount.value,
    });
    uuidResult.value = list.join('\n');
  } catch (err) {
    uuidError.value = `生成 UUID 失败：${errorMessage(err)}`;
  }
}

// ── 时间戳 ────────────────────────────────────────────────────
const tsInput = ref('');
const tsMode = ref('auto');
const tsResult = ref('');
const tsError = ref('');

async function convertTimestamp(): Promise<void> {
  tsError.value = '';
  try {
    tsResult.value = await ipc<string>('daily_tools_timestamp', {
      input: tsInput.value,
      mode: tsMode.value,
    });
  } catch (err) {
    tsError.value = `时间戳转换失败：${errorMessage(err)}`;
  }
}

// ── Unicode ───────────────────────────────────────────────────
const uniInput = ref('');
const uniStyle = ref('u');
const uniResult = ref('');
const uniError = ref('');

async function escapeUnicode(): Promise<void> {
  uniError.value = '';
  try {
    uniResult.value = await ipc<string>('daily_tools_unicode_escape', {
      input: uniInput.value,
      style: uniStyle.value,
    });
  } catch (err) {
    uniError.value = `转义失败：${errorMessage(err)}`;
  }
}

async function unescapeUnicode(): Promise<void> {
  uniError.value = '';
  try {
    uniResult.value = await ipc<string>('daily_tools_unicode_unescape', {
      input: uniInput.value,
    });
  } catch (err) {
    uniError.value = `反转义失败：${errorMessage(err)}`;
  }
}
</script>

<template>
  <div class="grid w-full grid-cols-12 gap-4">
    <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
      <!-- UUID -->
      <Panel title="UUID 生成">
        <div class="flex flex-wrap items-end gap-3">
          <div class="space-y-1.5">
            <Label>版本</Label>
            <Select v-model="uuidVersion">
              <SelectTrigger class="h-9 w-28"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="4">v4（随机）</SelectItem>
                <SelectItem value="7">v7（时间有序）</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="space-y-1.5">
            <Label>数量</Label>
            <Input v-model.number="uuidCount" type="number" min="1" max="1000" class="w-28" />
          </div>
          <Button @click="generateUuid">
            <Play class="mr-1 size-3.5" />
            生成
          </Button>
        </div>
        <ResultBox :value="uuidResult" label="UUID" max-height="180px" />
        <p v-if="uuidError" class="text-sm text-destructive">{{ uuidError }}</p>
      </Panel>

      <!-- 时间戳 -->
      <Panel title="时间戳转换">
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-4">
          <div class="space-y-1.5 sm:col-span-3">
            <Label>输入</Label>
            <Input
              v-model="tsInput"
              class="font-mono"
              placeholder="1700000000 或 2024-01-02 03:04:05"
            />
          </div>
          <div class="space-y-1.5">
            <Label>模式</Label>
            <Select v-model="tsMode">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="auto">自动（数字）</SelectItem>
                <SelectItem value="s">秒</SelectItem>
                <SelectItem value="ms">毫秒</SelectItem>
                <SelectItem value="to">日期 → 时间戳</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
        <Button :disabled="!tsInput" @click="convertTimestamp">
          <Play class="mr-1 size-3.5" />
          转换
        </Button>
        <ResultBox :value="tsResult" label="结果" />
        <p v-if="tsError" class="text-sm text-destructive">{{ tsError }}</p>
      </Panel>

      <!-- Unicode -->
      <Panel title="Unicode 转义">
        <div class="flex items-end gap-3">
          <div class="space-y-1.5">
            <Label>风格</Label>
            <Select v-model="uniStyle">
              <SelectTrigger class="h-9 w-32"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="u">\uXXXX</SelectItem>
                <SelectItem value="brace">\u{XXXX}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
        <Textarea
          v-model="uniInput"
          class="h-32 resize-y overflow-y-auto font-mono text-sm"
          style="field-sizing: fixed"
          placeholder="输入原始文本或转义序列…"
        />
        <div class="flex gap-2">
          <Button :disabled="!uniInput" @click="escapeUnicode">
            <Sparkles class="mr-1 size-3.5" />
            转义
          </Button>
          <Button variant="secondary" :disabled="!uniInput" @click="unescapeUnicode">
            反转义
          </Button>
        </div>
        <ResultBox :value="uniResult" label="结果" />
        <p v-if="uniError" class="text-sm text-destructive">{{ uniError }}</p>
      </Panel>
    </div>
  </div>
</template>
