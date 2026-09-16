<!--
  后台任务与通知：core/tasks + core/taskbar + core/notify + core/events 综合演示。
  - Rust 侧 hello_world_start_task 循环上报进度，前端经 tasksState 响应式读取
  - 事件总线：onEvent 订阅 task:// 事件并统计到达次数（控制台样式文本块）
  - 任务栏/Dock：进度由 core/tasks 自动联动；徽标手动演示
  - 系统通知：notify / notifyIfBackground 两种投递策略
  操作结果一律就地反馈，不用 toast。
-->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { Play, Square } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ipc } from '@/core/ipc';
import { logger } from '@/core/logger';
import { cancelTask, tasksState } from '@/core/tasks';
import { setAppBadgeCount } from '@/core/taskbar';
import { notify, notifyIfBackground } from '@/core/notify';
import { onEvent, TaskEvent } from '@/core/events';
import FormRow from '@/components/native/FormRow.vue';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';

const TASK_ID = 'hello-demo-task';

/** 响应式读取任务状态：tasksState 是 readonly reactive，未注册时回退默认态 */
const task = computed(() => tasksState[TASK_ID] ?? { running: false, done: 0, total: null });
const percent = computed(() => {
  const { done, total } = task.value;
  return total && total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 0;
});

const steps = ref(30);
const starting = ref(false);
const taskNote = ref('');

async function startTask(): Promise<void> {
  starting.value = true;
  taskNote.value = '';
  try {
    // 后台任务：不等返回（进度经 task:// 事件回传），错误单独记日志
    void ipc('hello_world_start_task', { taskId: TASK_ID, total: Math.max(1, steps.value) }).catch(
      (err) => logger.error(`后台任务失败: ${String(err)}`)
    );
    taskNote.value = '任务已启动，可切换到其他工具观察任务栏 / Dock 进度';
  } finally {
    starting.value = false;
  }
}

async function stopTask(): Promise<void> {
  await cancelTask(TASK_ID);
  taskNote.value = '已请求取消任务';
}

// ── 事件总线：订阅 task:// 事件（框架 core/tasks 亦在消费，互不影响） ──
const progressEvents = ref(0);
const lastEvent = ref('—');
let offProgress: (() => void) | undefined;
let offDone: (() => void) | undefined;
let offError: (() => void) | undefined;

onMounted(async () => {
  offProgress = await onEvent(TaskEvent.Progress, (payload) => {
    if (payload.taskId !== TASK_ID) return;
    progressEvents.value += 1;
    lastEvent.value = `progress · ${payload.done}/${payload.total ?? '?'} · ${payload.message ?? ''}`;
  });
  offDone = await onEvent(TaskEvent.Done, (payload) => {
    if (payload.taskId !== TASK_ID) return;
    lastEvent.value = 'done · 任务结束';
  });
  offError = await onEvent(TaskEvent.Error, (payload) => {
    if (payload.taskId !== TASK_ID) return;
    lastEvent.value = `error · ${payload.message}`;
  });
});

onUnmounted(() => {
  offProgress?.();
  offDone?.();
  offError?.();
});

// ── 任务栏 / Dock 徽标 ────────────────────────────────────────
const badgeCount = ref(3);
const badgeNote = ref('');

async function applyBadge(): Promise<void> {
  const count = Math.max(0, Math.floor(badgeCount.value));
  await setAppBadgeCount(count);
  badgeNote.value = count > 0 ? `Dock 徽标已设为 ${count}` : 'Dock 徽标已清除';
}

async function clearBadge(): Promise<void> {
  badgeCount.value = 0;
  await setAppBadgeCount(0);
  badgeNote.value = 'Dock 徽标已清除';
}

// ── 系统通知 ──────────────────────────────────────────────────
const notifyNote = ref('');

async function sendNow(): Promise<void> {
  await notify('PocketArk 演示通知', '由示例工具触发的系统通知。');
  notifyNote.value = '已发送系统通知（受设置页开关与系统权限控制）';
}

async function sendIfBackground(): Promise<void> {
  await notifyIfBackground('后台任务已完成', '窗口未在前台时才会打扰你。');
  notifyNote.value = '已尝试后台通知：窗口在前台时会静默跳过';
}
</script>

<template>
  <ToolShell
    title="后台任务与通知"
    description="core/tasks 进度与取消、任务栏/Dock 联动、事件总线订阅、系统通知投递"
  >
    <div class="grid grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
        <!-- 后台任务 -->
        <Panel title="后台任务" hint="core/tasks">
          <p class="text-xs text-muted-foreground">
            Rust 侧 hello_world_start_task 循环上报进度，前端响应式展示并可随时取消
          </p>
          <FormRow label="总步数" description="1–200，每步约 80ms">
            <template #default="{ id }">
              <div class="flex items-end gap-2">
                <Input
                  :id="id"
                  v-model.number="steps"
                  type="number"
                  min="1"
                  max="200"
                  class="w-28 text-right tabular-nums"
                  :disabled="task.running"
                />
                <Button :disabled="starting || task.running" @click="startTask">
                  <Play class="size-4" />
                  启动任务
                </Button>
                <Button variant="outline" :disabled="!task.running" @click="stopTask">
                  <Square class="size-4" />
                  取消
                </Button>
              </div>
            </template>
          </FormRow>

          <!-- 进度条（原生 div：进度数据来自 tasksState） -->
          <div class="space-y-1">
            <div class="h-2 overflow-hidden rounded-full bg-muted">
              <div
                class="h-full rounded-full bg-primary transition-[width] duration-100"
                :style="{ width: `${task.running ? Math.max(percent, 2) : percent}%` }"
              />
            </div>
            <div
              class="flex items-center justify-between font-mono text-xs tabular-nums text-muted-foreground"
            >
              <span>
                {{ task.running ? '运行中' : task.done > 0 ? '已结束' : '空闲' }} ·
                {{ task.done }}/{{ task.total ?? steps }}
              </span>
              <span>{{ percent }}%</span>
            </div>
            <p v-if="task.message" class="text-xs text-muted-foreground">{{ task.message }}</p>
          </div>
          <p v-if="taskNote" role="status" class="text-xs text-muted-foreground">{{ taskNote }}</p>
        </Panel>

        <!-- 事件总线（控制台样式只读文本块） -->
        <Panel title="事件总线" hint="task:// 事件">
          <p class="text-xs text-muted-foreground">
            onEvent(TaskEvent.*) 订阅 task:// 事件；core/tasks 亦在消费，互不影响
          </p>
          <div
            class="space-y-1 rounded-md bg-console p-3 font-mono text-xs whitespace-pre-wrap break-words text-console-foreground/80"
          >
            <p>
              <span class="text-console-foreground/60">已接收 progress 事件：</span
              >{{ progressEvents }}
            </p>
            <p class="break-all">
              <span class="text-console-foreground/60">最近事件：</span>{{ lastEvent }}
            </p>
          </div>
        </Panel>

        <!-- 任务栏 / Dock -->
        <Panel title="任务栏 / Dock" hint="core/taskbar">
          <p class="text-xs text-muted-foreground">
            任务进度由 core/tasks 自动写入任务栏；徽标数字手动设置（macOS Dock / 部分平台生效）
          </p>
          <FormRow label="徽标数字" description="0 表示清除徽标">
            <template #default="{ id }">
              <div class="flex items-center gap-2">
                <Input
                  :id="id"
                  v-model.number="badgeCount"
                  type="number"
                  min="0"
                  class="w-24 text-right tabular-nums"
                />
                <Button variant="secondary" size="sm" @click="applyBadge">设置徽标</Button>
                <Button variant="outline" size="sm" @click="clearBadge">清除徽标</Button>
              </div>
            </template>
          </FormRow>
          <p v-if="badgeNote" role="status" class="text-xs text-muted-foreground">
            {{ badgeNote }}
          </p>
        </Panel>

        <!-- 系统通知 -->
        <Panel title="系统通知" hint="core/notify">
          <p class="text-xs text-muted-foreground">
            受设置页「系统通知」开关与系统权限控制；后台通知仅在窗口失焦时发送
          </p>
          <div class="flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" @click="sendNow">立即通知</Button>
            <Button variant="outline" size="sm" @click="sendIfBackground">后台通知</Button>
          </div>
          <p v-if="notifyNote" role="status" class="text-xs text-muted-foreground">
            {{ notifyNote }}
          </p>
        </Panel>
      </div>
    </div>
  </ToolShell>
</template>
