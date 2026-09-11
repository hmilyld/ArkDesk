<!--
  工具页面模板。扩展步骤见本插件目录 README.md。

  页面约定（Tailwind 原生栅格）：
  - 用 ToolShell 包裹：页头（标题/说明/动作区）+ 全幅内容区
  - 居中列 / 分栏在页面内用 grid grid-cols-12 + col-start-* / col-span-*，
    居中列必须偶数跨距；禁止 mx-auto max-w-* 居中容器
-->
<script setup lang="ts">
import { Button } from '@/components/ui/button';
import ToolShell from '@/components/tool/ToolShell.vue';
</script>

<template>
  <ToolShell title="工具名称" description="一句话说明这个工具做什么">
    <template #actions>
      <Button variant="outline" size="sm">动作按钮（可选）</Button>
    </template>

    <!-- 混合内容示例：居中 8 列，lg 以下自动满幅 -->
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 lg:col-start-3 lg:col-span-8 space-y-4">
        <p class="text-sm text-muted-foreground">从这里开始构建你的工具页面。</p>
      </div>
    </div>
  </ToolShell>
</template>
