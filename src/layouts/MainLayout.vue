<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { useRouter } from 'vue-router';
import TitleBar from './components/TitleBar.vue';
import SideNav from './components/SideNav.vue';
import ToolErrorBoundary from './components/ToolErrorBoundary.vue';
import EditorContextMenu from './components/EditorContextMenu.vue';
import UpdateDialog from './components/UpdateDialog.vue';
import CommandPalette from './components/CommandPalette.vue';
import { AppEvent, onEvent } from '@/core/events';
import { openPalette } from '@/core/search/palette';
import { registerShortcut } from '@/core/shortcuts';

const router = useRouter();
let disposeMenu: (() => void) | null = null;
let disposeSettings: (() => void) | null = null;

onMounted(async () => {
  // 原生菜单项 → 前端动作（菜单 accelerator 已在系统层消费按键）
  disposeMenu = await onEvent(AppEvent.Menu, ({ id }) => {
    if (id === 'home') void router.push('/');
    else if (id === 'settings') void router.push('/settings');
    else if (id === 'command-palette') openPalette();
  });
  // 无原生菜单的平台（或用户偏好）用应用内快捷键
  disposeSettings = registerShortcut('mod+,', () => void router.push('/settings'));
});

onUnmounted(() => {
  disposeMenu?.();
  disposeSettings?.();
});
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-background text-foreground">
    <SideNav />
    <div class="flex min-w-0 flex-1 flex-col">
      <TitleBar />
      <main class="min-h-0 flex-1 overflow-y-auto">
        <RouterView v-slot="{ Component, route }">
          <!-- keepAlive: false 的工具不缓存（插件清单可逐项声明） -->
          <KeepAlive v-if="route.meta.keepAlive !== false" :max="12">
            <ToolErrorBoundary :key="String(route.path)">
              <component :is="Component" />
            </ToolErrorBoundary>
          </KeepAlive>
          <ToolErrorBoundary v-else :key="String(route.path)">
            <component :is="Component" />
          </ToolErrorBoundary>
        </RouterView>
      </main>
    </div>
  </div>
  <EditorContextMenu />
  <UpdateDialog />
  <CommandPalette />
</template>
