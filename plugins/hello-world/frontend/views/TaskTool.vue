<!--
  后台任务与通知：core/tasks + core/taskbar + core/notify + core/events 综合演示。
  - Rust 侧 hello_world_start_task 循环上报进度，前端经 tasksState 响应式读取
  - 事件总线：onEvent 订阅 task:// 事件并统计到达次数
  - 任务栏/Dock：进度由 core/tasks 自动联动；徽标手动演示
  - 系统通知：notify / notifyIfBackground 两种投递策略
-->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Play, Square } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ipc } from '@/core/ipc';
import { logger } from '@/core/logger';
import { cancelTask, tasksState } from '@/core/tasks';
import { setAppBadgeCount } from '@/core/taskbar';
import { notify, notifyIfBackground } from '@/core/notify';
import { onEvent, TaskEvent } from '@/core/events';
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

async function startTask(): Promise<void> {
  starting.value = true;
  try {
    // 后台任务：不等返回（进度经 task:// 事件回传），错误单独记日志
    void ipc('hello_world_start_task', { taskId: TASK_ID, total: Math.max(1, steps.value) }).catch(
      (err) => logger.error(`后台任务失败: ${String(err)}`)
    );
    toast.info('后台任务已启动', { description: '可切换工具，任务栏/Dock 仍显示进度' });
  } finally {
    starting.value = false;
  }
}

async function stopTask(): Promise<void> {
  await cancelTask(TASK_ID);
  toast.warning('已请求取消后台任务');
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

async function applyBadge(): Promise<void> {
  await setAppBadgeCount(Math.max(0, Math.floor(badgeCount.value)));
  toast.success(`Dock 徽标已设为 ${Math.max(0, Math.floor(badgeCount.value))}`);
}

async function clearBadge(): Promise<void> {
  await setAppBadgeCount(0);
  toast.success('Dock 徽标已清除');
}

// ── 系统通知 ──────────────────────────────────────────────────
async function sendNow(): Promise<void> {
  await notify('PocketArk 演示通知', '由示例工具触发的系统通知。');
  toast.info('已发送系统通知（受设置页开关与系统权限控制）');
}

async function sendIfBackground(): Promise<void> {
  await notifyIfBackground('后台任务已完成', '窗口未在前台时才会打扰你。');
  toast.info('已尝试后台通知：窗口在前台时会静默跳过');
}
</script>

<template>
  <ToolShell
    title="后台任务与通知"
    description="core/tasks 进度与取消、任务栏/Dock 联动、事件总线订阅、系统通知投递"
  >
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 space-y-6 lg:col-start-3 lg:col-span-8">
        <!-- 后台任务 -->
        <section class="space-y-2.5">
          <div>
            <h3 class="text-sm font-medium">后台任务</h3>
            <p class="text-xs text-muted-foreground">
              Rust 侧 hello_world_start_task 循环上报进度，前端响应式展示并可随时取消
            </p>
          </div>

          <div class="space-y-3 rounded-lg border border-border bg-card p-4">
            <div class="flex items-end gap-2">
              <div class="space-y-1">
                <Label for="task-steps" class="text-xs text-muted-foreground">总步数</Label>
                <Input
                  id="task-steps"
                  v-model.number="steps"
                  type="number"
                  min="1"
                  max="200"
                  class="w-28"
                  :disabled="task.running"
                />
              </div>
              <Button :disabled="starting || task.running" @click="startTask">
                <Play class="size-4" />
                启动任务
              </Button>
              <Button variant="outline" :disabled="!task.running" @click="stopTask">
                <Square class="size-4" />
                取消
              </Button>
            </div>

            <!-- 进度条（原生 div：进度数据来自 tasksState） -->
            <div class="space-y-1">
              <div class="h-2 overflow-hidden rounded-full bg-muted">
                <div
                  class="h-full rounded-full bg-primary transition-[width] duration-100"
                  :style="{ width: `${task.running ? Math.max(percent, 2) : percent}%` }"
                />
              </div>
              <div
                class="flex items-center justify-between font-mono text-xs text-muted-foreground"
              >
                <span>
                  {{ task.running ? '运行中' : task.done > 0 ? '已结束' : '空闲' }} ·
                  {{ task.done }}/{{ task.total ?? steps }}
                </span>
                <span>{{ percent }}%</span>
              </div>
              <p v-if="task.message" class="text-xs text-muted-foreground">{{ task.message }}</p>
            </div>
          </div>
        </section>

        <!-- 事件总线 -->
        <section class="space-y-2.5">
          <div>
            <h3 class="text-sm font-medium">事件总线</h3>
            <p class="text-xs text-muted-foreground">
              onEvent(TaskEvent.*) 订阅 task:// 事件；core/tasks 亦在消费，互不影响
            </p>
          </div>
          <div class="rounded-lg border border-border bg-card px-4 py-3 font-mono text-xs">
            <p>
              <span class="text-muted-foreground">已接收 progress 事件：</span>{{ progressEvents }}
            </p>
            <p class="truncate">
              <span class="text-muted-foreground">最近事件：</span>{{ lastEvent }}
            </p>
          </div>
        </section>

        <!-- 任务栏 / Dock -->
        <section class="space-y-2.5">
          <div>
            <h3 class="text-sm font-medium">任务栏 / Dock</h3>
            <p class="text-xs text-muted-foreground">
              任务进度由 core/tasks 自动写入任务栏；徽标数字手动设置（macOS Dock / 部分平台生效）
            </p>
          </div>
          <div class="flex items-center gap-2">
            <Input v-model.number="badgeCount" type="number" min="0" class="w-24" />
            <Button variant="secondary" size="sm" @click="applyBadge">设置徽标</Button>
            <Button variant="outline" size="sm" @click="clearBadge">清除徽标</Button>
          </div>
        </section>

        <!-- 系统通知 -->
        <section class="space-y-2.5">
          <div>
            <h3 class="text-sm font-medium">系统通知</h3>
            <p class="text-xs text-muted-foreground">
              受设置页「系统通知」开关与系统权限控制；后台通知仅在窗口失焦时发送
            </p>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" @click="sendNow">立即通知</Button>
            <Button variant="outline" size="sm" @click="sendIfBackground">后台通知</Button>
          </div>
        </section>
      </div>
    </div>
  </ToolShell>
</template>
