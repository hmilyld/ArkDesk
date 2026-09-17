<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { RouterLink } from 'vue-router';
import { getName } from '@tauri-apps/api/app';
import { House, PanelLeftClose, PanelLeftOpen, Settings } from '@lucide/vue';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { getAllTools, type ToolPlugin } from '@/core/plugins';
import { isMac } from '@/core/platform';
import { useSettingsStore } from '@/stores/settings';

const settings = useSettingsStore();

/** 窄窗自动折叠（容器变窄时自动隐藏；仅自动折叠过才在加宽时恢复） */
const AUTO_COLLAPSE_WIDTH = 900;
const autoCollapseApplied = ref(false);

function syncAutoCollapse(): void {
  const narrow = window.innerWidth < AUTO_COLLAPSE_WIDTH;
  if (narrow && !settings.sidebarCollapsed) {
    settings.sidebarCollapsed = true;
    autoCollapseApplied.value = true;
  } else if (!narrow && autoCollapseApplied.value && settings.sidebarCollapsed) {
    settings.sidebarCollapsed = false;
    autoCollapseApplied.value = false;
  }
}

/** 手动切换写回 store（collapsed 是 computed，不能直接赋值） */
function toggleSidebar(): void {
  settings.sidebarCollapsed = !settings.sidebarCollapsed;
  if (!settings.sidebarCollapsed) autoCollapseApplied.value = false;
}

/** 应用显示名：动态读取 tauri.conf 的 productName，改名无需改前端代码 */
const appName = ref('PocketArk');
onMounted(async () => {
  try {
    appName.value = await getName();
  } catch {
    // 读取失败回退默认名，不影响使用
  }
  syncAutoCollapse();
  window.addEventListener('resize', syncAutoCollapse);
});
onUnmounted(() => window.removeEventListener('resize', syncAutoCollapse));

/** 收起态：图标 rail（macOS 红绿灯已由 trafficLightPosition 左移收紧，68px 可完整容纳；
 *  其余平台 56px。宽度用固定 px，不随字号漂移） */
// design-lint-ignore：rail 宽度必须固定 px —— macOS 红绿灯留位（68px）不能随三档字号缩放
const railWidthClass = computed(() => (isMac ? 'w-[68px]' : 'w-14'));

interface ToolGroup {
  name: string;
  tools: ToolPlugin[];
}

const groups = computed<ToolGroup[]>(() => {
  const map = new Map<string, ToolPlugin[]>();
  for (const tool of getAllTools()) {
    if (!settings.isToolEnabled(tool.meta.id)) continue;
    const group = tool.meta.group ?? '其他';
    const list = map.get(group) ?? [];
    list.push(tool);
    map.set(group, list);
  }
  return [...map.entries()].map(([name, tools]) => ({ name, tools }));
});

/** 激活态：accent 低饱和填充 + 主色图标 + 提亮文字（DESIGN-macos.md §7：不用强调线/边框） */
const activeClasses = 'bg-primary/10 text-foreground font-medium [&>svg]:text-primary';
</script>

<template>
  <!-- 侧边导航：展开 = 图标+名称+分组，可收起为图标 rail；顶部整段为窗口拖拽区
       （macOS 红绿灯悬浮于拖拽区上方，不放任何文字） -->
  <TooltipProvider :delay-duration="200">
    <aside
      class="flex shrink-0 flex-col border-r bg-material-sidebar text-sidebar-foreground backdrop-blur-xl transition-[width]"
      :class="settings.sidebarCollapsed ? railWidthClass : 'w-52'"
    >
      <!-- 拖拽区（deep）：macOS 红绿灯旁不放文字；Windows/Linux 顶部显示应用名（收起态放不下） -->
      <div
        data-tauri-drag-region="deep"
        class="flex h-10 shrink-0 items-center border-b border-sidebar-border"
        :class="isMac || settings.sidebarCollapsed ? '' : 'px-4'"
      >
        <span
          v-if="!isMac && !settings.sidebarCollapsed"
          class="text-sm font-semibold tracking-wide"
        >
          {{ appName }}
        </span>
      </div>

      <nav class="min-h-0 flex-1 overflow-y-auto px-2 py-3">
        <!-- 首页：启动默认页，固定置顶 -->
        <template v-if="!settings.sidebarCollapsed">
          <RouterLink
            to="/"
            class="relative mb-3 flex items-center gap-2.5 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
            :active-class="activeClasses"
          >
            <House class="size-4 shrink-0 text-muted-foreground" />
            <span class="truncate">首页</span>
          </RouterLink>
        </template>
        <Tooltip v-else>
          <TooltipTrigger as-child>
            <RouterLink
              to="/"
              class="relative mx-auto mb-3 flex size-9 items-center justify-center rounded-md transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
              :active-class="activeClasses"
            >
              <House class="size-4 shrink-0 text-muted-foreground" />
            </RouterLink>
          </TooltipTrigger>
          <TooltipContent side="right" :side-offset="8">首页</TooltipContent>
        </Tooltip>

        <div v-for="group in groups" :key="group.name" class="mb-3">
          <!-- 分组标题：展开显示 uppercase 微标签，收起用分隔线代替 -->
          <p
            v-if="!settings.sidebarCollapsed"
            class="px-2 pb-1 text-xs font-medium text-muted-foreground"
          >
            {{ group.name }}
          </p>
          <div v-else class="mx-2 mb-2 border-t border-sidebar-border" />

          <template v-if="!settings.sidebarCollapsed">
            <RouterLink
              v-for="tool in group.tools"
              :key="tool.meta.id"
              :to="`/tool/${tool.meta.id}`"
              class="relative flex items-center gap-2.5 rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
              :active-class="activeClasses"
              exact-active-class="activeClasses"
            >
              <component :is="tool.meta.icon" class="size-4 shrink-0 text-muted-foreground" />
              <span class="truncate">{{ tool.meta.name }}</span>
            </RouterLink>
          </template>

          <template v-else>
            <Tooltip v-for="tool in group.tools" :key="tool.meta.id">
              <TooltipTrigger as-child>
                <RouterLink
                  :to="`/tool/${tool.meta.id}`"
                  class="relative mx-auto mb-0.5 flex size-9 items-center justify-center rounded-md transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
                  :active-class="activeClasses"
                  exact-active-class="activeClasses"
                >
                  <component :is="tool.meta.icon" class="size-4 shrink-0 text-muted-foreground" />
                </RouterLink>
              </TooltipTrigger>
              <TooltipContent side="right" :side-offset="8">
                {{ tool.meta.name }}
              </TooltipContent>
            </Tooltip>
          </template>
        </div>
      </nav>

      <div
        class="flex shrink-0 flex-col border-t border-sidebar-border"
        :class="settings.sidebarCollapsed ? 'items-center gap-0.5 px-2 py-2' : 'gap-0.5 px-2 py-2'"
      >
        <!-- 收起/展开开关 -->
        <Tooltip>
          <TooltipTrigger as-child>
            <button
              type="button"
              class="flex rounded-md text-muted-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
              :class="
                settings.sidebarCollapsed
                  ? 'size-9 items-center justify-center'
                  : 'items-center gap-2.5 px-2 py-1.5 text-sm'
              "
              :aria-label="settings.sidebarCollapsed ? '展开侧栏' : '收起侧栏'"
              @click="toggleSidebar"
            >
              <PanelLeftOpen v-if="settings.sidebarCollapsed" class="size-4" />
              <PanelLeftClose v-else class="size-4" />
              <span v-if="!settings.sidebarCollapsed">收起侧栏</span>
            </button>
          </TooltipTrigger>
          <TooltipContent v-if="settings.sidebarCollapsed" side="right" :side-offset="8">
            展开侧栏
          </TooltipContent>
        </Tooltip>

        <!-- 设置入口 -->
        <Tooltip>
          <TooltipTrigger as-child>
            <RouterLink
              to="/settings"
              class="relative flex rounded-md transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
              :class="
                settings.sidebarCollapsed
                  ? 'size-9 items-center justify-center'
                  : 'items-center gap-2.5 px-2 py-1.5 text-sm'
              "
              :active-class="activeClasses"
              exact-active-class="activeClasses"
            >
              <Settings class="size-4 shrink-0 text-muted-foreground" />
              <span v-if="!settings.sidebarCollapsed">设置</span>
            </RouterLink>
          </TooltipTrigger>
          <TooltipContent v-if="settings.sidebarCollapsed" side="right" :side-offset="8">
            设置
          </TooltipContent>
        </Tooltip>
      </div>
    </aside>
  </TooltipProvider>
</template>
