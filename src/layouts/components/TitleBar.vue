<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { getName } from '@tauri-apps/api/app';
import { ChevronRight, Minus, Search, Square, X } from '@lucide/vue';
import { openPalette } from '@/core/search/palette';
import { isMac } from '@/core/platform';

const route = useRoute();
const pageTitle = computed(() => route.meta.title as string | undefined);
const shortcutHint = isMac ? '⌘K' : 'Ctrl K';

/** 应用显示名：动态读取 tauri.conf 的 productName，改名无需改前端代码 */
const appName = ref('ArkDesk');
onMounted(async () => {
  try {
    appName.value = await getName();
  } catch {
    // 读取失败回退默认名，不影响使用
  }
});

// Windows/Linux（decorations: false）需要自绘窗口控制按钮；macOS 用系统红绿灯
const appWindow = getCurrentWindow();
const maximized = ref(false);

onMounted(() => {
  if (isMac) return;
  void appWindow.isMaximized().then((value) => (maximized.value = value));
  void appWindow.onResized(async () => {
    maximized.value = await appWindow.isMaximized();
  });
});
</script>

<template>
  <!-- 内容区顶栏：整栏 deep 拖拽区（任意非可点击子元素均可拖动窗口 / 双击缩放）；
       面包屑 = 应用名（仅 macOS，Windows 下应用名在侧栏顶部）+ 当前工具名 -->
  <header
    data-tauri-drag-region="deep"
    class="flex h-[var(--toolbar-h)] shrink-0 items-center justify-between gap-3 border-b bg-material-toolbar pl-4 pr-0 backdrop-blur-xl"
  >
    <nav class="flex min-w-0 items-center gap-1.5" aria-label="面包屑">
      <span
        v-if="isMac"
        class="shrink-0 text-xs font-medium tracking-wide text-muted-foreground/70"
      >
        {{ appName }}
      </span>
      <template v-if="pageTitle">
        <ChevronRight
          v-if="isMac"
          class="size-3.5 shrink-0 text-muted-foreground/40"
          aria-hidden="true"
        />
        <span class="truncate text-sm font-medium text-foreground">
          {{ pageTitle }}
        </span>
      </template>
    </nav>

    <div class="flex min-w-0 flex-1 items-center justify-end pr-2">
      <button
        type="button"
        class="flex h-7 items-center gap-1.5 rounded-md border border-border px-2 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        aria-label="搜索"
        @click="openPalette"
      >
        <Search class="size-3.5" />
        <span class="hidden sm:inline">搜索</span>
        <kbd class="hidden rounded border border-border px-1 text-[10px] sm:inline">
          {{ shortcutHint }}
        </kbd>
      </button>
    </div>

    <div v-if="!isMac" class="flex h-full items-stretch">
      <button
        type="button"
        class="flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        aria-label="最小化"
        @click="appWindow.minimize()"
      >
        <Minus class="size-4" />
      </button>
      <button
        type="button"
        class="flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
        :aria-label="maximized ? '还原' : '最大化'"
        @click="appWindow.toggleMaximize()"
      >
        <Square class="size-3.5" />
      </button>
      <button
        type="button"
        class="flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-destructive hover:text-white"
        aria-label="关闭"
        @click="appWindow.close()"
      >
        <X class="size-4" />
      </button>
    </div>
  </header>
</template>
