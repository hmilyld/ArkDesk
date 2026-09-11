/**
 * 类型化事件总线（框架能力）。
 *
 * - 统一事件命名前缀（`app://` / `task://` / `http://` / `updater://`），避免随意命名
 * - `AppEventMap` 登记「事件名 → 负载类型」：emit/on 即获得编译期类型约束
 * - Rust 侧同名常量见 `src-tauri/src/events.rs`（两端保持同步）
 *
 * 使用：
 *   await emitEvent(AppEvent.Open, { paths, source: 'cli' });
 *   const un = await onEvent(TaskEvent.Progress, (p) => { ... });
 *   un(); // 取消订阅
 */
import { emit, listen, type Event, type UnlistenFn } from '@tauri-apps/api/event';
import { logger } from '@/core/logger';

/** 事件名前缀（命名规范，仅供约定与校验） */
export const EVENT_PREFIX = {
  app: 'app://',
  task: 'task://',
  http: 'http://',
  updater: 'updater://',
} as const;

/** 应用级事件名 */
export const AppEvent = {
  /** 统一「打开内容」入口（CLI 参数 / 深链接 / 拖拽文件） */
  Open: 'app://open',
  /** 原生菜单项点击（id 由 Rust 菜单定义） */
  Menu: 'app://menu',
} as const;

/** 后台任务事件名 */
export const TaskEvent = {
  Progress: 'task://progress',
  Done: 'task://done',
  Error: 'task://error',
} as const;

/** HTTP 事件名 */
export const HttpEvent = {
  DownloadProgress: 'http://download-progress',
} as const;

/** 在线更新事件名 */
export const UpdaterEvent = {
  Progress: 'updater://progress',
  Installed: 'updater://installed',
} as const;

/** 事件名 → 负载类型映射；新增事件在此登记 */
export interface AppEventMap {
  'app://open': { paths: string[]; source: 'cli' | 'deep-link' | 'drag-drop' };
  'app://menu': { id: string };
  'task://progress': { taskId: string; done: number; total: number | null; message?: string };
  'task://done': { taskId: string };
  'task://error': { taskId: string; message: string };
  'http://download-progress': { url: string; downloaded: number; total: number | null };
  'updater://progress': { downloaded: number; total: number | null };
  'updater://installed': Record<string, never>;
}

export type AppEventName = keyof AppEventMap;

/** 发送类型化事件（失败仅记日志，不抛出） */
export async function emitEvent<K extends AppEventName>(
  name: K,
  payload: AppEventMap[K]
): Promise<void> {
  try {
    await emit(name, payload);
  } catch (err) {
    logger.warn(`事件发送失败: ${name}`);
    logger.debug(String(err));
  }
}

/** 订阅类型化事件，返回取消订阅函数 */
export async function onEvent<K extends AppEventName>(
  name: K,
  handler: (payload: AppEventMap[K], event: Event<AppEventMap[K]>) => void
): Promise<UnlistenFn> {
  return listen<AppEventMap[K]>(name, (event) => handler(event.payload, event));
}
