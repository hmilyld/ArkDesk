<!--
  Markdown 源码展示组件：纯文本展示 Markdown 源码，支持一键复制。
  外框与头部由 Panel 统一；调用方可用 #actions 追加动作（如「导出 .md」）。
-->
<script setup lang="ts">
import { ref } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Copy } from '@lucide/vue';
import Panel from '@/components/tool/Panel.vue';

defineProps<{
  /** Markdown 内容 */
  content: string;
}>();

const copied = ref(false);

async function copyToClipboard(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    copied.value = true;
    toast.success('已复制到剪贴板');
    setTimeout(() => (copied.value = false), 2000);
  } catch {
    toast.error('复制失败');
  }
}
</script>

<template>
  <Panel title="Markdown">
    <template #actions>
      <slot name="actions" />
      <Button variant="ghost" size="sm" @click="copyToClipboard(content)">
        <Copy class="mr-1.5 size-3.5" />
        {{ copied ? '已复制' : '复制' }}
      </Button>
    </template>

    <pre
      class="max-h-[500px] overflow-auto whitespace-pre-wrap break-words font-mono text-sm leading-relaxed"
      >{{ content }}</pre>
  </Panel>
</template>
