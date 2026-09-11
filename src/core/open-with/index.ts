/**
 * 统一「打开内容」分发（框架能力）。
 *
 * 汇聚三种来源并经 `onOpenFiles` 分发给注册者：
 * - CLI / 文件关联：启动参数（`take_pending_open`）与二次启动参数（`app://open`）
 * - 深链接：`app://open`（source = deep-link）
 * - 拖拽文件到窗口：`onDragDropEvent`
 *
 * 插件/页面可注册处理函数：
 *   const off = onOpenFiles((paths, source) => { ... });
 */
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ipc } from '@/core/ipc';
import { logger } from '@/core/logger';
import { AppEvent, onEvent } from '@/core/events';

export type OpenSource = 'cli' | 'deep-link' | 'drag-drop';
export type OpenHandler = (paths: string[], source: OpenSource) => void | Promise<void>;

const handlers = new Set<OpenHandler>();

/** 注册「打开内容」处理函数，返回取消注册函数 */
export function onOpenFiles(handler: OpenHandler): () => void {
  handlers.add(handler);
  return () => handlers.delete(handler);
}

function dispatch(paths: string[], source: OpenSource): void {
  if (paths.length === 0) return;
  logger.info(`打开内容（${source}）: ${paths.join(', ')}`);
  for (const handler of handlers) {
    try {
      Promise.resolve(handler(paths, source)).catch((err) => {
        logger.error(`打开内容处理失败: ${String(err)}`);
      });
    } catch (err) {
      logger.error(`打开内容处理失败: ${String(err)}`);
    }
  }
}

let initialized = false;

/** 应用启动时调用一次：注册事件、拖拽监听，并消费启动参数 */
export async function initOpenWith(): Promise<void> {
  if (initialized) return;
  initialized = true;

  await onEvent(AppEvent.Open, ({ paths, source }) => dispatch(paths, source));

  try {
    await getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === 'drop') {
        dispatch(event.payload.paths, 'drag-drop');
      }
    });
  } catch (err) {
    logger.debug(`拖拽监听注册失败: ${String(err)}`);
  }

  try {
    const pending = await ipc<string[]>('take_pending_open');
    dispatch(pending, 'cli');
  } catch (err) {
    logger.debug(`读取启动参数失败: ${String(err)}`);
  }
}
