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

/** 当前标签页：切换时把外层页面滚动容器回到顶部，避免沿用上一页的滚动位置 */
const activeTab = ref('system');
const rootEl = ref<HTMLElement | null>(null);
watch(activeTab, () => rootEl.value?.closest('main')?.scrollTo({ top: 0 }));
</script>

<template>
  <!-- 竖向设置导航：Tailwind 栅格居中 8 列（lg 以下满幅），惯用法同 ToolShell 页面。
       整页随 MainLayout 的 main 滚动；左侧导航 sticky 吸附在滚动容器顶部，始终可见。 -->
  <div ref="rootEl" class="mx-auto grid w-full grid-cols-12">
    <Tabs
      v-model="activeTab"
      orientation="vertical"
      class="col-span-12 lg:col-start-3 lg:col-span-8 items-start w-full gap-6 p-5"
    >
      <!-- self-start 阻止列表被内容区高度拉伸（flex 默认 stretch）；top-5 抵消 Tabs 的 p-5 -->
      <TabsList
        class="sticky top-5 w-36 shrink-0 self-start flex-col items-stretch gap-1 bg-transparent p-0"
      >
        <TabsTrigger
          value="system"
          class="h-auto flex-none justify-start px-3 py-1.5 text-sm font-normal"
        >
          系统设置
        </TabsTrigger>
        <TabsTrigger
          v-for="tool in toolSettings"
          :key="tool.meta.id"
          :value="tool.meta.id"
          class="h-auto flex-none justify-start px-3 py-1.5 text-sm font-normal"
        >
          {{ tool.settings?.label ?? tool.meta.name }}
        </TabsTrigger>
        <!-- 固定尾部：更新日志 / 关于（Markdown 驱动，永远排在所有工具设置之后） -->
        <TabsTrigger
          value="changelog"
          class="h-auto flex-none justify-start px-3 py-1.5 text-sm font-normal"
        >
          更新日志
        </TabsTrigger>
        <TabsTrigger
          value="about"
          class="h-auto flex-none justify-start px-3 py-1.5 text-sm font-normal"
        >
          关于
        </TabsTrigger>
      </TabsList>

      <div class="min-w-0 flex-1">
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
    </Tabs>
  </div>
</template>
