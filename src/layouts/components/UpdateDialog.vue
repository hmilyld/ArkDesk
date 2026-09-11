<script setup lang="ts">
import { computed } from 'vue';
import { marked } from 'marked';
import { toast } from 'vue-sonner';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { logger } from '@/core/logger';
import { normalizeError } from '@/core/errors';
import { dismissUpdateDialog, installUpdate, restartApp, updaterState } from '@/core/updater';

const open = computed({
  get: () => updaterState.dialogOpen,
  set: (value: boolean) => {
    if (!value) dismissUpdateDialog();
  },
});

const notesHtml = computed(() =>
  updaterState.info?.notes ? marked.parse(updaterState.info.notes, { async: false }) : ''
);

/** 下载百分比：无总大小时返回 null（显示不确定进度） */
const percent = computed(() => {
  if (!updaterState.total || updaterState.total <= 0) return null;
  return Math.min(100, Math.round((updaterState.downloaded / updaterState.total) * 100));
});

const versionLabel = computed(() => {
  const info = updaterState.info;
  if (!info) return '';
  return `${info.currentVersion} → ${info.version}`;
});

async function onInstall(): Promise<void> {
  try {
    await installUpdate();
  } catch (err) {
    const error = normalizeError(err);
    logger.error(`安装更新失败: [${error.code}] ${error.message}`);
    toast.error(`安装更新失败：${error.message}`, { description: error.code });
  }
}

async function onRestart(): Promise<void> {
  try {
    await restartApp();
  } catch (err) {
    const error = normalizeError(err);
    logger.error(`重启失败: [${error.code}] ${error.message}`);
    toast.error(`重启失败：${error.message}`, { description: error.code });
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>
          {{ updaterState.installed ? '更新已就绪' : '发现新版本' }}
        </DialogTitle>
        <DialogDescription>
          {{
            updaterState.installed
              ? '安装完成，重启应用以运行新版本'
              : `新版本 ${updaterState.info?.version ?? ''} 可用（${versionLabel}）`
          }}
        </DialogDescription>
      </DialogHeader>

      <!-- 更新说明 -->
      <!-- eslint-disable vue/no-v-html —— 内容来自更新服务器，经 marked 渲染；CSP 严格无内联脚本 -->
      <div
        v-if="notesHtml && !updaterState.installed"
        class="markdown-body max-h-64 overflow-y-auto rounded-md border border-border bg-muted/30 p-3"
        v-html="notesHtml"
      />
      <!-- eslint-enable vue/no-v-html -->

      <!-- 下载进度 -->
      <div v-if="updaterState.installing" class="space-y-2">
        <div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
          <div
            class="h-full bg-primary transition-all"
            :style="{ width: percent === null ? '40%' : `${percent}%` }"
          />
        </div>
        <p class="text-xs text-muted-foreground">
          {{
            percent === null
              ? '正在下载…'
              : `正在下载… ${percent}%（${Math.round(updaterState.downloaded / 1024)} KB）`
          }}
        </p>
      </div>

      <p v-if="updaterState.error" class="text-xs text-destructive">{{ updaterState.error }}</p>

      <DialogFooter>
        <template v-if="updaterState.installed">
          <Button size="sm" @click="onRestart">立即重启</Button>
        </template>
        <template v-else>
          <Button
            variant="outline"
            size="sm"
            :disabled="updaterState.installing"
            @click="open = false"
          >
            稍后
          </Button>
          <Button size="sm" :disabled="updaterState.installing" @click="onInstall">
            {{ updaterState.installing ? '安装中…' : '下载并安装' }}
          </Button>
        </template>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
