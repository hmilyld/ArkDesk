/**
 * 开机自启（框架能力）。
 *
 * 真相源在操作系统：启动时读取真实状态回填设置项，用户切换时写回系统。
 */
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import { logger } from '@/core/logger';

/** 读取系统当前的开机自启状态（失败按关闭处理） */
export async function readAutoStart(): Promise<boolean> {
  try {
    return await isEnabled();
  } catch (err) {
    logger.debug(`读取开机自启状态失败: ${String(err)}`);
    return false;
  }
}

/** 应用开机自启设置到系统 */
export async function applyAutoStart(enabled: boolean): Promise<void> {
  try {
    await (enabled ? enable() : disable());
  } catch (err) {
    logger.warn(`开机自启设置失败（${enabled ? '启用' : '禁用'}）`);
    logger.debug(String(err));
  }
}
