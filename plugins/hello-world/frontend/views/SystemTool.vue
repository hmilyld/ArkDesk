<!--
  窗口与系统集成：core/windows + core/open-with + core/shortcuts + core/platform 演示。
  - 多窗口：openAppWindow 打开（或聚焦）同一应用路由的次级窗口
  - 打开内容：onOpenFiles 统一接收拖拽 / CLI / 深链接来源
  - 应用内快捷键：registerShortcut 注册页面级快捷键（返回取消函数）
  - 平台探测：isMac 驱动平台差异
-->
<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { AppWindow, FolderOpen, Keyboard, Monitor } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { logger } from '@/core/logger';
import { openAppWindow } from '@/core/windows';
import { onOpenFiles, type OpenSource } from '@/core/open-with';
import { registerShortcut } from '@/core/shortcuts';
import { isMac } from '@/core/platform';
import ToolShell from '@/components/tool/ToolShell.vue';

// ── 多窗口 ────────────────────────────────────────────────────
const opening = ref('');

async function openChild(id: string, title: string, route: string): Promise<void> {
  opening.value = id;
  try {
    const win = await openAppWindow({ id, title, route, width: 860, height: 600 });
    if (win) toast.success(`窗口已打开：${title}`, { description: `路由 ${route}` });
  } catch (err) {
    logger.error(`打开窗口失败: ${String(err)}`);
    toast.error('打开窗口失败');
  } finally {
    opening.value = '';
  }
}

// ── 打开内容（拖拽 / CLI / 深链接） ──────────────────────────
interface ReceivedItem {
  path: string;
  source: OpenSource;
  at: string;
}

const received = ref<ReceivedItem[]>([]);
let offOpenFiles: (() => void) | undefined;

onMounted(() => {
  offOpenFiles = onOpenFiles((paths, source) => {
    const at = new Date().toLocaleTimeString('zh-CN', { hour12: false });
    for (const path of paths) received.value.unshift({ path, source, at });
    received.value = received.value.slice(0, 20);
  });
});

onUnmounted(() => offOpenFiles?.());

// ── 应用内快捷键 ──────────────────────────────────────────────
const hotkeyCount = ref(0);
const COMBO = 'mod+shift+d';
let disposeShortcut: (() => void) | undefined;

onMounted(() => {
  disposeShortcut = registerShortcut(COMBO, () => {
    hotkeyCount.value += 1;
    toast.info(`快捷键已触发（第 ${hotkeyCount.value} 次）`, { description: COMBO });
  });
});

onUnmounted(() => disposeShortcut?.());

const comboLabel = isMac ? '⌘ ⇧ D' : 'Ctrl + Shift + D';
const userAgent = navigator.userAgent;
</script>

<template>
  <ToolShell title="窗口与系统集成" description="多窗口、打开内容分发、应用内快捷键与平台探测">
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 space-y-6 lg:col-start-3 lg:col-span-8">
        <!-- 多窗口 -->
        <section class="space-y-2.5">
          <div>
            <h3 class="flex items-center gap-1.5 text-sm font-medium">
              <AppWindow class="size-4 text-muted-foreground" />多窗口
            </h3>
            <p class="text-xs text-muted-foreground">
              openAppWindow 打开同一前端的次级窗口（label = win-&lt;id&gt;）；已存在则聚焦
            </p>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button
              variant="secondary"
              size="sm"
              :disabled="opening !== ''"
              @click="openChild('demo-table', '数据表格 · 子窗口', '/tool/hello-table')"
            >
              打开「数据表格」窗口
            </Button>
            <Button
              variant="outline"
              size="sm"
              :disabled="opening !== ''"
              @click="openChild('demo-task', '后台任务 · 子窗口', '/tool/hello-task')"
            >
              打开「后台任务」窗口
            </Button>
          </div>
        </section>

        <!-- 打开内容 -->
        <section class="space-y-2.5">
          <div>
            <h3 class="flex items-center gap-1.5 text-sm font-medium">
              <FolderOpen class="size-4 text-muted-foreground" />打开内容（拖拽 / CLI / 深链接）
            </h3>
            <p class="text-xs text-muted-foreground">
              本页注册 onOpenFiles：把文件拖到窗口任意位置，即可看到来源与路径
            </p>
          </div>
          <div
            class="rounded-lg border border-dashed border bg-card px-4 py-6 text-center text-xs text-muted-foreground"
          >
            把文件拖进窗口试试 —— 或从命令行传入文件路径启动应用
          </div>
          <div v-if="received.length > 0" class="space-y-1">
            <div class="flex items-center justify-between">
              <span class="text-xs text-muted-foreground">最近接收（{{ received.length }}）</span>
              <Button variant="ghost" size="sm" @click="received = []">清空</Button>
            </div>
            <div class="divide-y divide-border/60 overflow-hidden rounded-lg border border-border">
              <div
                v-for="(item, index) in received"
                :key="`${item.at}-${index}`"
                class="flex items-center gap-2 px-3 py-2"
              >
                <Badge variant="secondary" class="shrink-0">{{ item.source }}</Badge>
                <span class="min-w-0 flex-1 truncate font-mono text-xs">{{ item.path }}</span>
                <span class="shrink-0 font-mono text-xs text-muted-foreground">{{ item.at }}</span>
              </div>
            </div>
          </div>
        </section>

        <!-- 应用内快捷键 -->
        <section class="space-y-2.5">
          <div>
            <h3 class="flex items-center gap-1.5 text-sm font-medium">
              <Keyboard class="size-4 text-muted-foreground" />应用内快捷键
            </h3>
            <p class="text-xs text-muted-foreground">
              registerShortcut 注册页面级快捷键，返回取消函数在卸载时调用
            </p>
          </div>
          <div class="flex items-center gap-3 rounded-lg border bg-card px-4 py-3">
            <span class="font-mono text-sm">{{ comboLabel }}</span>
            <span class="text-xs text-muted-foreground">已触发 {{ hotkeyCount }} 次</span>
          </div>
        </section>

        <!-- 平台探测 -->
        <section class="space-y-2.5">
          <div>
            <h3 class="flex items-center gap-1.5 text-sm font-medium">
              <Monitor class="size-4 text-muted-foreground" />平台探测
            </h3>
            <p class="text-xs text-muted-foreground">
              isMac 驱动平台差异（样式与快捷键修饰键）；下方为只读环境信息
            </p>
          </div>
          <div class="rounded-lg border bg-card px-4 py-3 font-mono text-xs">
            <p><span class="text-muted-foreground">isMac：</span>{{ isMac }}</p>
            <p class="break-all">
              <span class="text-muted-foreground">userAgent：</span>{{ userAgent }}
            </p>
          </div>
        </section>
      </div>
    </div>
  </ToolShell>
</template>
