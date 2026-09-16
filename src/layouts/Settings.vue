<script setup lang="ts">
import { computed, defineAsyncComponent, ref, watch } from 'vue';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { getToolsWithSettings, type ToolPlugin } from '@/core/plugins';
import { useSettingsStore } from '@/stores/settings';
import SystemSettings from './settings/SystemSettings.vue';
import AboutSettings from './settings/AboutSettings.vue';
import MarkdownView from './settings/MarkdownView.vue';
// 内置静态内容：?raw 导入，构建期内联（src/content/*.md）
import changelogMd from '@/content/changelog.md?raw';

const settings = useSettingsStore();

/** 有设置面板且已启用的工具（禁用后 tab 一并隐藏，配置数据保留） */
const toolSettings = computed<ToolPlugin[]>(() =>
  getToolsWithSettings().filter((tool) => settings.isToolEnabled(tool.meta.id))
);

/** 面板懒加载：defineAsyncComponent 结果按工具缓存，避免重复创建组件定义 */
const asyncPanels = computed<Record<string, ReturnType<typeof defineAsyncComponent>>>(() =>
  Object.fromEntries(
    toolSettings.value.map((tool) => [tool.meta.id, defineAsyncComponent(tool.settings!.component)])
  )
);

/** pane 列表（工具栏式切换：Safari / Xcode 偏好设置的用法，见 DESIGN-macos.md §5） */
const panes = computed(() => [
  { value: 'system', label: '系统设置' },
  ...toolSettings.value.map((tool) => ({
    value: tool.meta.id,
    label: tool.settings?.label ?? tool.meta.name,
  })),
  { value: 'changelog', label: '更新日志' },
  { value: 'about', label: '关于' },
]);

/** 当前面板：切换时把外层页面滚动容器回到顶部，避免沿用上一页的滚动位置 */
const activeTab = ref('system');
const rootEl = ref<HTMLElement | null>(null);
watch(activeTab, () => rootEl.value?.closest('main')?.scrollTo({ top: 0 }));
</script>

<template>
  <!-- 设置页：pane 切换做成工具栏式的横向 tab 条（粘在覆盖式工具栏之下），
       内容单列居中 8 列（与工具页栅格用法一致）。 -->
  <div ref="rootEl" class="w-full">
    <Tabs v-model="activeTab" class="w-full gap-0">
      <!-- 高度用内联样式覆盖：shadcn TabsList 对横向有 group 变体 h-9（36px），普通 class 覆盖不掉，
           与 py-2 叠加会让 28px 的 tab 溢出压到下边框；此处显式 = 工具栏高度 + py-0，垂直居中由 items-center 保证 -->
      <!-- 高度用内联样式置为 auto：shadcn TabsList 对横向有 group 变体 h-9（36px），普通 class 覆盖不掉。
           上下用等值 py-1.5 由构造保证对称（此前固定高度 + 容器边框会让下间距看起来偏小）。 -->
      <TabsList
        class="sticky top-0 z-10 flex w-full flex-wrap items-center justify-center gap-1 rounded-none border-b bg-material-toolbar px-5 py-1.5 backdrop-blur-xl"
        :style="{ height: 'auto' }"
      >
        <!-- 选中态用 accent 实心填充：与同页「主题」分段控件一致；
             低饱和 tint 铺在毛玻璃材质上会被冲淡（DESIGN.md §2.5） -->
        <TabsTrigger
          v-for="pane in panes"
          :key="pane.value"
          :value="pane.value"
          class="h-7 flex-none rounded-md px-2.5 text-sm font-normal data-[state=active]:bg-primary data-[state=active]:font-medium data-[state=active]:text-primary-foreground"
        >
          {{ pane.label }}
        </TabsTrigger>
      </TabsList>

      <div class="mx-auto grid w-full grid-cols-12 gap-4 p-5">
        <div class="col-span-12 lg:col-start-3 lg:col-span-8">
          <TabsContent value="system">
            <SystemSettings />
          </TabsContent>

          <TabsContent v-for="tool in toolSettings" :key="tool.meta.id" :value="tool.meta.id">
            <component :is="asyncPanels[tool.meta.id]" />
          </TabsContent>

          <TabsContent value="changelog">
            <MarkdownView :source="changelogMd" />
          </TabsContent>
          <TabsContent value="about">
            <AboutSettings />
          </TabsContent>
        </div>
      </div>
    </Tabs>
  </div>
</template>
