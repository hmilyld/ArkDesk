<!--
  认证编辑器：None / Basic / Bearer / API Key（header 或 query）。
  发送时合成；若手写了同名头，以手写优先（由 buildSendOptions 处理并警示）。
-->
<script setup lang="ts">
import { computed } from 'vue';
import { ShieldOff } from '@lucide/vue';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import EmptyState from '@/components/native/EmptyState.vue';
import Segmented from '@/components/native/Segmented.vue';
import type { HttpAuth, HttpAuthLocation, HttpAuthType } from '../shared';

const props = defineProps<{ modelValue: HttpAuth }>();
const emit = defineEmits<{ 'update:modelValue': [value: HttpAuth] }>();

const AUTH_TYPES: { value: HttpAuthType; label: string }[] = [
  { value: 'none', label: '无' },
  { value: 'basic', label: 'Basic Auth' },
  { value: 'bearer', label: 'Bearer Token' },
  { value: 'apikey', label: 'API Key' },
];

const AUTH_LOCATIONS: { value: HttpAuthLocation; label: string }[] = [
  { value: 'header', label: 'Header' },
  { value: 'query', label: 'Query' },
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
    <Segmented :model-value="auth.type" :segments="AUTH_TYPES" @update:model-value="setType" />

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
        <Segmented
          :model-value="auth.apiKeyIn"
          size="sm"
          :segments="AUTH_LOCATIONS"
          @update:model-value="setLocation"
        />
      </div>
    </div>

    <EmptyState
      v-else
      :icon="ShieldOff"
      title="未启用认证"
      description="选择上方的认证类型以添加凭据"
    />

    <p class="text-xs text-muted-foreground">
      认证信息以明文保存于本地数据库，请勿在共享设备上存放敏感凭据。
    </p>
  </div>
</template>
