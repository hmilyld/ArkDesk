<!--
  工具页面模板。扩展步骤见本插件目录 README.md。

  页面约定：
  - 用 ToolShell 包裹：页头（标题 / 说明 / 动作区，动作按钮 size="sm"、图标 size-3.5）
  - 页面内每个功能模块都用 Panel 包裹（输入 / 结果 / 列表…）：外框 + 头部条（标题 + 右上角
    动作）+ 正文；不要在页面上裸露模块，也不要在 Panel 内嵌套卡片
  - 设置模块例外：设置面板统一用 @/components/settings 的 SettingsSection / SettingsRow /
    SettingsField（SettingsSection 本身就是分组卡片），不要再套一层 Panel，以免卡片叠卡片
  - 空 / 加载 / 错误三态用 @/components/native 的 EmptyState / LoadingState / ErrorState，就地呈现
  - 视觉取值只来自 token（字号 ≥ text-xs、圆角 md/lg/浮层 xl、边框 hairline、焦点环统一写法）
  - 居中列 / 分栏用 grid grid-cols-12 + col-start-* / col-span-*（居中列偶数跨距），
    禁止 mx-auto max-w-* 居中容器
-->
<script setup lang="ts">
import { ref } from 'vue';
import { PackageOpen, Play, RefreshCw } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { greet } from '../lib/example';
import { useCounter } from '../composables/useCounter';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';
import EmptyState from '@/components/native/EmptyState.vue';
import ErrorState from '@/components/native/ErrorState.vue';
import LoadingState from '@/components/native/LoadingState.vue';
import { SettingsField, SettingsRow, SettingsSection } from '@/components/settings';

// 设置示例（真实插件用 useToolSettings 持久化，见本插件 frontend/settings/Settings.vue）
const trimSpaces = ref(true);
const maxLines = ref('200');
const timeout = ref('30');
const retries = ref('3');

// 输入 / 结果
const input = ref('');
const result = ref('');

// 目录规范示例：纯逻辑放 frontend/lib/（此处 greet），响应式逻辑放 frontend/composables/（此处 useCounter）
const { count, doubled, increment, reset: resetCounter } = useCounter();

// 结果区状态：idle（空态）/ loading（加载态）/ error（错误态）/ done（结果）
type ResultState = 'idle' | 'loading' | 'error' | 'done';
const state = ref<ResultState>('idle');

function run(): void {
  state.value = 'loading';
  result.value = '';
  // 演示：真实工具在此调用 ipc / http / db；<1s 不显示加载态，>1s 用 LoadingState
  window.setTimeout(() => {
    const lines = input.value
      .split('\n')
      .map((line) => (trimSpaces.value ? line.trim() : line))
      .filter((line) => line.length > 0);
    if (lines.length === 0) {
      state.value = 'error';
      result.value = '';
      return;
    }
    increment();
    result.value =
      `${greet(lines[0])}\n` +
      `已处理 ${lines.length} 行（上限 ${maxLines.value}，超时 ${timeout.value} s，重试 ${retries.value} 次）\n` +
      `示例 · lib/example.ts 的 greet() · 第 ${count.value} 次运行（计数翻倍 ${doubled.value}）`;
    state.value = 'done';
  }, 400);
}

function reset(): void {
  input.value = '';
  result.value = '';
  state.value = 'idle';
  resetCounter();
}

async function copyResult(): Promise<void> {
  if (!result.value) return;
  await navigator.clipboard?.writeText(result.value);
}
</script>

<template>
  <ToolShell title="工具名称" description="一句话说明这个工具做什么">
    <template #actions>
      <Button variant="ghost" size="sm" @click="reset">
        <RefreshCw class="size-3.5" />
        清空
      </Button>
      <Button size="sm" :disabled="state === 'loading'" @click="run">
        <Play class="size-3.5" />
        运行
      </Button>
    </template>

    <!-- 混合内容示例：居中 8 列，lg 以下自动满幅 -->
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
        <!-- 设置模块：SettingsSection 自带分组卡片，不再套 Panel -->
        <SettingsSection title="设置">
          <SettingsRow title="去除首尾空格" description="写入前清理每行两端的空白字符">
            <Checkbox v-model="trimSpaces" />
          </SettingsRow>
          <SettingsRow title="最多保留行数" description="超出部分将被截断">
            <Input v-model="maxLines" type="number" class="w-32 text-right tabular-nums" />
          </SettingsRow>
          <!-- 密集数值字段：在 SettingsSection 内用 grid + SettingsField -->
          <div class="grid grid-cols-2 gap-3 p-3">
            <SettingsField label="超时（秒）" description="单次请求的最大等待时间">
              <Input v-model="timeout" type="number" class="text-right tabular-nums" />
            </SettingsField>
            <SettingsField label="重试次数" description="失败后自动重试的次数">
              <Input v-model="retries" type="number" class="text-right tabular-nums" />
            </SettingsField>
          </div>
        </SettingsSection>

        <!-- 输入模块：右上角放选择文件 / 清空等动作（ghost / secondary） -->
        <Panel title="输入">
          <template #actions>
            <Button variant="ghost" size="sm" @click="input = ''">清空</Button>
            <Button variant="secondary" size="sm">选择文件</Button>
          </template>
          <Textarea
            v-model="input"
            class="h-40 font-mono text-sm"
            placeholder="每行一条，例如：要处理的文本"
          />
        </Panel>

        <!-- 结果模块：空 / 加载 / 错误 / 结果 四态就地切换，成功不用 toast -->
        <Panel title="结果" hint="处理结果将显示在这里">
          <template #actions>
            <Button variant="ghost" size="sm" :disabled="state !== 'done'" @click="copyResult">
              复制
            </Button>
          </template>

          <LoadingState v-if="state === 'loading'" variant="spinner" label="处理中…" />
          <ErrorState
            v-else-if="state === 'error'"
            message="输入为空：请先填写至少一行内容，再点「运行」"
            :on-retry="run"
          />
          <EmptyState
            v-else-if="state === 'idle'"
            :icon="PackageOpen"
            title="还没有结果"
            description="填写输入后点右上角「运行」"
          />
          <pre
            v-else
            class="overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap"
            >{{ result }}</pre>
        </Panel>
      </div>
    </div>
  </ToolShell>
</template>
