<!-- 根证书信息 / 导出 / 平台安装指引 -->
<script setup lang="ts">
import { computed } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import type { CaInfo } from '../../intercept-shared';
import { copyToClipboard } from '../../shared';

const props = defineProps<{ open: boolean; ca: CaInfo | null }>();
const emit = defineEmits<{
  'update:open': [value: boolean];
  export: [];
  regenerate: [];
}>();

const open = computed({
  get: () => props.open,
  set: (value: boolean) => emit('update:open', value),
});

const certPath = computed(() => props.ca?.certPath ?? '');

const macCommand = computed(
  () =>
    `sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain "${certPath.value}"`
);
const winCommand = computed(() => `certutil -addstore -f Root "${certPath.value}"`);

async function copy(text: string): Promise<void> {
  if (!text) return;
  try {
    await copyToClipboard(text);
    toast.success('已复制命令');
  } catch {
    toast.error('复制失败');
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="max-w-[92vw] sm:max-w-3xl">
      <DialogHeader>
        <DialogTitle>根证书（CA）</DialogTitle>
        <DialogDescription>
          拦截 HTTPS 需要客户端信任本机生成的根证书；证书与私钥仅保存在本机。
        </DialogDescription>
      </DialogHeader>

      <div class="min-w-0 space-y-3 text-xs">
        <div class="rounded-md border border-border bg-sunken p-3 font-mono">
          <p><span class="text-muted-foreground">存在：</span>{{ ca?.exists ? '是' : '否' }}</p>
          <p class="break-all">
            <span class="text-muted-foreground">指纹：</span>{{ ca?.fingerprint || '—' }}
          </p>
          <p><span class="text-muted-foreground">有效期至：</span>{{ ca?.notAfter || '—' }}</p>
          <p class="break-all">
            <span class="text-muted-foreground">路径：</span>{{ ca?.certPath || '—' }}
          </p>
        </div>

        <div v-if="ca?.exists" class="space-y-4 pt-1">
          <p class="font-medium">安装到系统信任库</p>
          <div class="space-y-2">
            <p class="text-muted-foreground">macOS</p>
            <div class="flex items-stretch gap-2">
              <code
                class="min-w-0 flex-1 overflow-x-auto whitespace-pre rounded-md bg-console px-2 py-1.5 font-mono text-console-foreground"
                >{{ macCommand }}</code
              >
              <Button variant="outline" size="sm" class="h-auto shrink-0" @click="copy(macCommand)">
                复制
              </Button>
            </div>
          </div>
          <div class="space-y-2">
            <p class="text-muted-foreground">Windows（管理员 PowerShell / CMD）</p>
            <div class="flex items-stretch gap-2">
              <code
                class="min-w-0 flex-1 overflow-x-auto whitespace-pre rounded-md bg-console px-2 py-1.5 font-mono text-console-foreground"
                >{{ winCommand }}</code
              >
              <Button variant="outline" size="sm" class="h-auto shrink-0" @click="copy(winCommand)">
                复制
              </Button>
            </div>
          </div>
          <p class="text-muted-foreground leading-5">
            Firefox 需在「设置 → 隐私与安全 → 证书 → 查看证书 → 导入」中单独信任该证书。 证书固定的
            App 无法被拦截。
          </p>
        </div>
        <p v-else class="text-muted-foreground">启动一次代理后将自动生成根证书。</p>
      </div>

      <DialogFooter class="gap-2 sm:justify-between">
        <Button variant="destructive" size="sm" :disabled="!ca?.exists" @click="emit('regenerate')">
          重新生成证书
        </Button>
        <div class="flex gap-2">
          <Button variant="outline" size="sm" :disabled="!ca?.exists" @click="emit('export')">
            导出证书
          </Button>
          <Button size="sm" @click="open = false">关闭</Button>
        </div>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
