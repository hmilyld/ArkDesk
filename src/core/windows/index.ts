/**
 * 多窗口（框架能力）。
 *
 * 次级窗口 label 统一为 `win-<id>`，加载同一前端并按 hash 路由定位。
 * 已存在同名窗口时聚焦而非重复创建。
 */
import { getAllWebviewWindows, WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { logger } from '@/core/logger';

export interface OpenWindowOptions {
  /** 唯一 id（窗口 label = win-<id>） */
  id: string;
  title: string;
  /** 应用内 hash 路由，如 '/tool/xxx' */
  route: string;
  width?: number;
  height?: number;
}

/** 打开（或聚焦）一个应用窗口 */
export async function openAppWindow(options: OpenWindowOptions): Promise<WebviewWindow | null> {
  const label = `win-${options.id}`;

  const windows = await getAllWebviewWindows();
  const found = windows.find((win) => win.label === label);
  if (found) {
    await found.show();
    await found.unminimize();
    await found.setFocus();
    return found;
  }

  const win = new WebviewWindow(label, {
    url: `index.html#${options.route}`,
    title: options.title,
    width: options.width ?? 900,
    height: options.height ?? 640,
    center: true,
  });

  win.once('tauri://error', (event) => {
    logger.error(`窗口创建失败（${label}）: ${String(event.payload)}`);
  });

  return win;
}
