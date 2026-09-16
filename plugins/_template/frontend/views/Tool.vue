<!--
  工具页面模板。扩展步骤见本插件目录 README.md。

  页面约定（Tailwind 原生栅格）：
  - 用 ToolShell 包裹：页头（标题/说明/动作区）+ 全幅内容区
  - 页面内每个功能模块（设置 / 输入 / 输出 / 结果）都用 Panel 包裹：外框 + 头部条（标题 +
    右上角动作）+ 正文；不要在页面上裸露模块，也不要在 Panel 内嵌套卡片
  - 居中列 / 分栏在页面内用 grid grid-cols-12 + col-start-* / col-span-*，
    居中列必须偶数跨距；禁止 mx-auto max-w-* 居中容器
-->
<script setup lang="ts">
import { ref } from 'vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';

const input = ref('');
const result = ref('');
</script>

<template>
  <ToolShell title="工具名称" description="一句话说明这个工具做什么">
    <template #actions>
      <Button variant="outline" size="sm">动作按钮（可选）</Button>
    </template>

    <!-- 混合内容示例：居中 8 列，lg 以下自动满幅 -->
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
        <!-- 设置面板：选项、开关等 -->
        <Panel title="设置">
          <div class="grid grid-cols-2 gap-3">
            <div class="space-y-1.5">
              <Label>示例选项</Label>
              <Input placeholder="输入点什么" />
            </div>
          </div>
        </Panel>

        <!-- 输入面板：右上角放选择文件 / 清空等动作 -->
        <Panel title="输入">
          <template #actions>
            <Button variant="ghost" size="sm" @click="input = ''">清空</Button>
            <Button variant="secondary" size="sm">选择文件</Button>
          </template>
          <Textarea v-model="input" class="h-40 font-mono text-sm" />
        </Panel>

        <!-- 输出面板：右上角放复制 / 导出等动作，内容用等宽文本或表格 -->
        <Panel title="输出" hint="结果将显示在这里">
          <template #actions>
            <Button variant="ghost" size="sm" :disabled="!result">复制</Button>
          </template>
          <pre v-if="result" class="max-h-96 overflow-auto font-mono text-sm whitespace-pre-wrap">{{
            result
          }}</pre>
          <p v-else class="py-10 text-center text-sm text-muted-foreground">暂无内容</p>
        </Panel>
      </div>
    </div>
  </ToolShell>
</template>
