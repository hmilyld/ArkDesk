<!--
  认证编辑器：None / Basic / Bearer / API Key（header 或 query）。
  发送时合成；若手写了同名头，以手写优先（由 buildSendOptions 处理并警示）。
-->
<script setup lang="ts">
import { computed } from 'vue';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { HttpAuth, HttpAuthLocation, HttpAuthType } from '../shared';

const props = defineProps<{ modelValue: HttpAuth }>();
const emit = defineEmits<{ 'update:modelValue': [value: HttpAuth] }>();

const AUTH_TYPES: { value: HttpAuthType; label: string }[] = [
  { value: 'none', label: '无' },
  { value: 'basic', label: 'Basic Auth' },
  { value: 'bearer', label: 'Bearer Token' },
  { value: 'apikey', label: 'API Key' },
];

const auth = computed(() => props.modelValue);

function patch(changes: Partial<HttpAuth>): void {
  emit('update:modelValue', { ...props.modelValue, ...changes });
}
function setType(value: unknown): void {
  patch({ type: value as HttpAuthType });
}
function setLocation(value: unknown): void {
  patch({ apiKeyIn: value as HttpAuthLocation });
}
</script>

<template>
  <div class="space-y-3">
    <div class="flex items-center gap-2">
      <Label class="text-xs text-muted-foreground">类型</Label>
      <Select :model-value="auth.type" @update:model-value="setType">
        <SelectTrigger class="w-56">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem v-for="item in AUTH_TYPES" :key="item.value" :value="item.value">
            {{ item.label }}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>

    <div v-if="auth.type === 'basic'" class="grid grid-cols-2 gap-2">
      <Input
        :model-value="auth.basicUsername"
        placeholder="用户名"
        spellcheck="false"
        class="font-mono text-xs"
        @update:model-value="(value) => patch({ basicUsername: String(value) })"
      />
      <Input
        :model-value="auth.basicPassword"
        type="password"
        placeholder="密码"
        class="font-mono text-xs"
        @update:model-value="(value) => patch({ basicPassword: String(value) })"
      />
    </div>

    <Input
      v-else-if="auth.type === 'bearer'"
      :model-value="auth.bearerToken"
      placeholder="Token（自动加 Bearer 前缀）"
      spellcheck="false"
      class="font-mono text-xs"
      @update:model-value="(value) => patch({ bearerToken: String(value) })"
    />

    <div v-else-if="auth.type === 'apikey'" class="space-y-2">
      <div class="grid grid-cols-3 gap-2">
        <Input
          :model-value="auth.apiKeyName"
          placeholder="Key 名称"
          spellcheck="false"
          class="font-mono text-xs"
          @update:model-value="(value) => patch({ apiKeyName: String(value) })"
        />
        <Input
          :model-value="auth.apiKeyValue"
          placeholder="Key 值"
          spellcheck="false"
          class="col-span-2 font-mono text-xs"
          @update:model-value="(value) => patch({ apiKeyValue: String(value) })"
        />
      </div>
      <div class="flex items-center gap-2">
        <Label class="text-xs text-muted-foreground">添加到</Label>
        <Select :model-value="auth.apiKeyIn" @update:model-value="setLocation">
          <SelectTrigger class="w-36">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="header">Header</SelectItem>
            <SelectItem value="query">Query</SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>

    <p v-else class="py-6 text-center text-xs text-muted-foreground">未启用认证</p>

    <p class="text-xs text-muted-foreground">
      认证信息以明文保存于本地数据库，请勿在共享设备上存放敏感凭据。
    </p>
  </div>
</template>
