/**
 * 系统通知（框架能力）。
 *
 * - 发送前自动检查/请求通知权限
 * - 受设置项 `notificationEnabled` 控制（关闭时静默跳过）
 * - `notifyIfBackground()`：仅窗口不在前台时发送，适合长任务完成提示
 * - 失败仅记日志，不打断业务
 */
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
  type Options,
} from '@tauri-apps/plugin-notification';
import { useSettingsStore } from '@/stores/settings';
import { logger } from '@/core/logger';

async function ensurePermission(): Promise<boolean> {
  try {
    if (await isPermissionGranted()) return true;
    return (await requestPermission()) === 'granted';
  } catch (err) {
    logger.warn('通知权限检查失败');
    logger.debug(String(err));
    return false;
  }
}

/** 发送系统通知（未启用或未授权时静默跳过） */
export async function notify(title: string, body?: string, options?: Options): Promise<void> {
  const settings = useSettingsStore();
  if (!settings.notificationEnabled) return;
  if (!(await ensurePermission())) return;
  try {
    sendNotification({ title, body, ...options });
  } catch (err) {
    logger.warn('系统通知发送失败');
    logger.debug(String(err));
  }
}

/**
 * 仅当窗口不在前台时发送系统通知。
 * 用于「长任务完成」等场景：用户正在看应用时不打扰，切走后才提示。
 */
export async function notifyIfBackground(
  title: string,
  body?: string,
  options?: Options
): Promise<void> {
  if (typeof document !== 'undefined' && document.hasFocus()) return;
  await notify(title, body, options);
}
