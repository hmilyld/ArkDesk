<!--
  请求编辑区：Params / Headers / Cookies / Body / Auth。
  props 驱动（`v-model` 整个 HttpRequestSpec），供发送页与未来拦截页复用。
-->
<script setup lang="ts">
import { computed, ref } from 'vue';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import AuthEditor from './AuthEditor.vue';
import BodyEditor from './BodyEditor.vue';
import KeyValueEditor from './KeyValueEditor.vue';
import { parseCookieHeader, type HttpRequestSpec } from '../shared';

const props = defineProps<{ modelValue: HttpRequestSpec }>();
const emit = defineEmits<{ 'update:modelValue': [value: HttpRequestSpec] }>();

const spec = computed(() => props.modelValue);
const rawCookie = ref('');

function patch(changes: Partial<HttpRequestSpec>): void {
  emit('update:modelValue', { ...props.modelValue, ...changes });
}
function applyRawCookie(): void {
  if (!rawCookie.value.trim()) return;
  patch({ cookies: [...props.modelValue.cookies, ...parseCookieHeader(rawCookie.value)] });
  rawCookie.value = '';
}
</script>

<template>
  <Tabs default-value="params" class="gap-3">
    <TabsList>
      <TabsTrigger value="params">Params</TabsTrigger>
      <TabsTrigger value="headers">Headers</TabsTrigger>
      <TabsTrigger value="cookies">Cookies</TabsTrigger>
      <TabsTrigger value="body">Body</TabsTrigger>
      <TabsTrigger value="auth">Auth</TabsTrigger>
    </TabsList>

    <TabsContent value="params">
      <KeyValueEditor
        :model-value="spec.query"
        add-label="添加参数"
        key-placeholder="参数名"
        value-placeholder="参数值"
        @update:model-value="(value) => patch({ query: value })"
      />
    </TabsContent>

    <TabsContent value="headers">
      <KeyValueEditor
        :model-value="spec.headers"
        add-label="添加请求头"
        key-placeholder="Header 名"
        value-placeholder="Header 值"
        @update:model-value="(value) => patch({ headers: value })"
      />
    </TabsContent>

    <TabsContent value="cookies" class="space-y-3">
      <KeyValueEditor
        :model-value="spec.cookies"
        add-label="添加 Cookie"
        key-placeholder="Cookie 名"
        value-placeholder="Cookie 值"
        @update:model-value="(value) => patch({ cookies: value })"
      />
      <div class="space-y-1.5">
        <p class="text-xs text-muted-foreground">粘贴原始 Cookie 头（如从浏览器 F12 复制）</p>
        <Textarea
          v-model="rawCookie"
          placeholder="a=1; b=2"
          spellcheck="false"
          class="min-h-16 font-mono text-xs"
        />
        <Button variant="outline" size="sm" type="button" @click="applyRawCookie">
          解析并添加
        </Button>
      </div>
    </TabsContent>

    <TabsContent value="body">
      <BodyEditor
        :model-value="spec.body"
        @update:model-value="(value) => patch({ body: value })"
      />
    </TabsContent>

    <TabsContent value="auth">
      <AuthEditor
        :model-value="spec.auth"
        @update:model-value="(value) => patch({ auth: value })"
      />
    </TabsContent>
  </Tabs>
</template>
