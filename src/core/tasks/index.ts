/**
 * 后台任务（框架能力）。
 *
 * 消费 `task://` 事件，维护各任务的状态，并联动任务栏/Dock 进度。
 *   const off = onTask... （暂未提供订阅；用 taskState(id) 响应式读取）
 *   await cancelTask('my-task');
 */
import { reactive, readonly } from 'vue';
import { onEvent, TaskEvent } from '@/core/events';
import { ipc } from '@/core/ipc';
import { setTaskbarProgress } from '@/core/taskbar';

export interface TaskState {
  running: boolean;
  done: number;
  total: number | null;
  message?: string;
}

const state = reactive<Record<string, TaskState>>({});

export const tasksState = readonly(state);

function ensure(taskId: string): TaskState {
  if (!state[taskId]) {
    state[taskId] = { running: true, done: 0, total: null };
  }
  return state[taskId];
}

/** 读取任务状态（未运行返回默认态） */
export function taskState(taskId: string): TaskState {
  return state[taskId] ?? { running: false, done: 0, total: null };
}

/** 取消指定任务 */
export async function cancelTask(taskId: string): Promise<void> {
  await ipc('task_cancel', { taskId });
}

let initialized = false;

/** 应用启动时调用一次：注册任务事件监听 */
export async function initTasks(): Promise<void> {
  if (initialized) return;
  initialized = true;

  await onEvent(TaskEvent.Progress, (payload) => {
    const current = ensure(payload.taskId);
    current.running = true;
    current.done = payload.done;
    current.total = payload.total;
    current.message = payload.message;
    void setTaskbarProgress(
      payload.total && payload.total > 0 ? Math.round((payload.done / payload.total) * 100) : null
    );
  });
  await onEvent(TaskEvent.Done, (payload) => {
    ensure(payload.taskId).running = false;
    void setTaskbarProgress(null);
  });
  await onEvent(TaskEvent.Error, (payload) => {
    const current = ensure(payload.taskId);
    current.running = false;
    current.message = payload.message;
    void setTaskbarProgress(null);
  });
}
