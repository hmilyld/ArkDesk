<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { getVersion } from '@tauri-apps/api/app';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';
import { checkForUpdates, updaterState } from '@/core/updater';
import { useSettingsStore } from '@/stores/settings';
import MarkdownView from './MarkdownView.vue';
// 内置静态内容：?raw 导入，构建期内联（src/content/about.md）
import aboutMd from '@/content/about.md?raw';

const settings = useSettingsStore();
const version = ref('');

onMounted(async () => {
  try {
    version.value = await getVersion();
  } catch (err) {
    logger.debug(`读取应用版本失败: ${String(err)}`);
  }
});

async function onCheckUpdate(): Promise<void> {
  try {
    const info = await checkForUpdates();
    if (info) {
      toast.info(`发现新版本 ${info.version}`);
    } else {
      toast.success('当前已是最新版本');
    }
  } catch (err) {
    const error = normalizeError(err);
    logger.error(`检查更新失败: [${error.code}] ${error.message}`);
    toast.error(`检查更新失败：${error.message}`, { description: error.code });
  }
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between gap-3 rounded-lg border bg-card px-3 py-2.5">
      <div>
        <p class="text-sm">ArkDesk{{ version ? ` v${version}` : '' }}</p>
        <p class="text-xs text-muted-foreground">
          在线更新{{ settings.updateEnabled ? '已启用' : '未启用（可在系统设置中开启）' }}
        </p>
      </div>
      <Button
        variant="outline"
        size="sm"
        :disabled="!settings.updateEnabled || !settings.updateServerUrl || updaterState.checking"
        @click="onCheckUpdate"
      >
        {{ updaterState.checking ? '检查中…' : '检查更新' }}
      </Button>
    </div>

    <MarkdownView :source="aboutMd" />
  </div>
</template>
