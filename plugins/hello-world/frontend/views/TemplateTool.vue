<!--
  页面模板：模块 Panel、布局档位与页面三态的标准写法（新工具起手样板）。
  - 每个功能模块用 @/components/tool/Panel 包裹（外框 + 头部条 + 右上角动作）
  - 栅格档位参考 _template README：表格满幅 / 宽内容 10 / 混合 8 / 表单设置 6
-->
<script setup lang="ts">
import { ref } from 'vue';
import { toast } from 'vue-sonner';
import { PackageOpen } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';

/** 错误态演示开关（真实场景由工具自身状态驱动） */
const simulateError = ref(true);
const reloading = ref(false);

function reload(): void {
  reloading.value = true;
  // 演示：真实工具此处重新拉取数据
  setTimeout(() => {
    reloading.value = false;
    toast.info('重试完成（演示）');
  }, 600);
}
</script>

<template>
  <ToolShell
    title="页面模板"
    description="新工具起手样板：模块 Panel + Tailwind 栅格布局档位 + 空态 / 加载态 / 错误态标准写法"
  >
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-10 lg:col-start-2">
        <!-- 模块面板：每个功能模块（设置 / 输入 / 输出 / 结果）都用 Panel 包裹，不裸露、不嵌套卡片 -->
        <Panel title="模块面板（Panel）" hint="外框 + 头部条 + 右上角动作">
          <template #actions>
            <Button variant="ghost" size="sm">右上角动作</Button>
          </template>
          <p class="text-xs text-muted-foreground">
            模块外框与头部视觉由 Panel 统一：标题走 props，动作放 #actions 插槽，正文默认 space-y-3
            p-4（满幅场景用 body-class 覆盖）
          </p>
        </Panel>

        <!-- 布局档位：grid-cols-12 开栅格，col-start-* / col-span-* 定位居中列 -->
        <Panel title="布局档位（12 列栅格）">
          <p class="text-xs text-muted-foreground">
            页面用 grid grid-cols-12 开栅格，内容以 col-start-* / col-span-*
            定位（居中列用偶数跨距）
          </p>
          <div
            v-for="[cols, label] in [
              [12, 'col-span-12 · 表格类满幅'],
              [10, 'col-span-10 · 宽内容'],
              [8, 'col-span-8 · 混合内容'],
              [6, 'col-span-6 · 表单 / 设置'],
            ] as const"
            :key="cols"
            class="flex items-center gap-3"
          >
            <div class="flex h-7 flex-1 items-center rounded-sm bg-muted">
              <div
                class="flex h-full items-center rounded-sm bg-primary/15 pl-2 text-xs font-medium text-primary"
                :style="{ width: `${(cols / 12) * 100}%` }"
              >
                {{ label }}
              </div>
            </div>
            <span class="w-10 text-right font-mono text-xs text-muted-foreground">
              {{ Math.round((cols / 12) * 100) }}%
            </span>
          </div>
        </Panel>

        <!-- 空态：图标 + 主文案 + 动作（Panel 内不再套虚线卡片；满幅页面可用 border-dashed 框住） -->
        <Panel title="空态">
          <p class="text-xs text-muted-foreground">无数据时给出下一步动作，禁止留白</p>
          <div class="flex flex-col items-center gap-2 py-8">
            <PackageOpen class="size-8 text-muted-foreground/60" />
            <p class="text-sm font-medium">还没有任何记录</p>
            <p class="text-xs text-muted-foreground">从动作区开始你的第一步</p>
            <Button variant="outline" size="sm" class="mt-1" @click="toast.info('演示：新建记录')">
              新建记录
            </Button>
          </div>
        </Panel>

        <!-- 加载态：Skeleton 骨架屏（结构仿真实列表） -->
        <Panel title="加载态">
          <p class="text-xs text-muted-foreground">骨架屏占位，结构与真实内容一致</p>
          <div class="space-y-2">
            <Skeleton class="h-4 w-1/3" />
            <Skeleton class="h-4 w-2/3" />
            <Skeleton class="h-4 w-1/2" />
            <Skeleton class="h-4 w-3/4" />
          </div>
        </Panel>

        <!-- 错误态：描述 + 重试动作（真实场景由 ToolErrorBoundary 兜底，此处为业务级错误示范） -->
        <Panel title="错误态">
          <p class="text-xs text-muted-foreground">
            业务级错误就地展示（组件级异常由 ToolErrorBoundary 统一兜底）
          </p>
          <div
            v-if="simulateError"
            class="flex items-center justify-between gap-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2"
          >
            <div>
              <p class="text-xs font-medium text-destructive">数据加载失败</p>
              <p class="text-xs text-muted-foreground">[DB_ERROR] 查询超时，请检查数据库文件</p>
            </div>
            <Button variant="outline" size="sm" :disabled="reloading" @click="reload">
              {{ reloading ? '重试中…' : '重试' }}
            </Button>
          </div>
          <Button v-else variant="outline" size="sm" @click="simulateError = true">
            显示错误态示例
          </Button>
        </Panel>
      </div>
    </div>
  </ToolShell>
</template>
