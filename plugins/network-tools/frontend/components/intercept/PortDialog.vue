<!-- 首次启动代理时确认监听端口 -->
<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

const props = defineProps<{ open: boolean; port: number }>();
const emit = defineEmits<{ 'update:open': [value: boolean]; confirm: [port: number] }>();

const open = computed({
  get: () => props.open,
  set: (value: boolean) => emit('update:open', value),
});
const value = ref(String(props.port));
const error = ref('');

watch(
  () => props.open,
  (next) => {
    if (next) {
      value.value = String(props.port);
      error.value = '';
    }
  }
);

function submit(): void {
  const port = Number(value.value);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    error.value = '端口需为 1–65535 的整数';
    return;
  }
  emit('confirm', port);
  open.value = false;
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="max-w-[92vw] sm:max-w-sm">
      <DialogHeader>
        <DialogTitle>设置代理端口</DialogTitle>
        <DialogDescription>本地代理将监听 127.0.0.1:端口，仅本机可用。</DialogDescription>
      </DialogHeader>
      <div class="space-y-1.5">
        <Label class="text-xs">监听端口</Label>
        <Input
          v-model="value"
          type="number"
          min="1"
          max="65535"
          placeholder="8888"
          @keydown.enter="submit"
        />
        <p v-if="error" class="text-xs text-destructive">{{ error }}</p>
        <p v-else class="text-xs text-muted-foreground">建议 8888，避免使用系统保留端口。</p>
      </div>
      <DialogFooter>
        <Button size="sm" @click="submit">确定并启动</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
