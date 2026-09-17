<!--
  复制按钮：统一各输出面板的复制交互。
-->
<script setup lang="ts">
import { ref } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Check, Copy } from '@lucide/vue';
import { copyToClipboard } from '../../lib/crypto-shared';

const props = defineProps<{ value: string }>();
const copied = ref(false);

async function handleCopy(): Promise<void> {
  if (!props.value) return;
  try {
    await copyToClipboard(props.value);
    copied.value = true;
    toast.success('已复制到剪贴板');
    setTimeout(() => (copied.value = false), 1500);
  } catch {
    toast.error('复制失败');
  }
}
</script>

<template>
  <Button variant="ghost" size="sm" :disabled="!value" @click="handleCopy">
    <component :is="copied ? Check : Copy" class="mr-1 size-3.5" />
    {{ copied ? '已复制' : '复制' }}
  </Button>
</template>
