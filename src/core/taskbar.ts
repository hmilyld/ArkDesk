/**
 * 任务栏进度 / Dock 徽标（框架能力）。
 *
 * 供后台任务（core/tasks）与插件展示进度、未读数量等。失败仅记日志。
 */
import { getCurrentWindow, ProgressBarStatus } from '@tauri-apps/api/window';
import { logger } from '@/core/logger';

/** 设置任务栏/Dock 进度（0–100；传 null 清除） */
export async function setTaskbarProgress(progress: number | null): Promise<void> {
  try {
    const win = getCurrentWindow();
    if (progress === null) {
      await win.setProgressBar({ status: ProgressBarStatus.None });
    } else {
      await win.setProgressBar({
        status: ProgressBarStatus.Normal,
        progress: Math.max(0, Math.min(100, progress)),
      });
    }
  } catch (err) {
    logger.debug(`设置任务栏进度失败: ${String(err)}`);
  }
}

/** 设置 Dock 徽标数字（0 清除） */
export async function setAppBadgeCount(count: number): Promise<void> {
  try {
    await getCurrentWindow().setBadgeCount(count);
  } catch (err) {
    logger.debug(`设置应用徽标失败: ${String(err)}`);
  }
}
