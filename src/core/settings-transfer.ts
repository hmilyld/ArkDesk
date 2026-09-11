/**
 * 设置导入 / 导出（框架能力）。
 *
 * 导出：settings.json 全部键值 + 外观偏好（主题/主题色/字号存于 localStorage）。
 * 导入：读取文件并写回；因内存中的响应式设置需重新加载，故导入需重启应用生效。
 */
import { open, save } from '@tauri-apps/plugin-dialog';
import { ipc } from '@/core/ipc';
import { logger } from '@/core/logger';
import { useSettingsStore } from '@/stores/settings';

const FILE_FILTERS = [{ name: 'JSON', extensions: ['json'] }];

/** 外观偏好（存于 localStorage，需随设置一同迁移） */
const APPEARANCE_KEYS = [
  'arkdesk.theme',
  'arkdesk.accent',
  'arkdesk.accentCustom',
  'arkdesk.fontSize',
] as const;

function readAppearance(): Record<string, string> {
  const result: Record<string, string> = {};
  for (const key of APPEARANCE_KEYS) {
    try {
      const value = localStorage.getItem(key);
      if (value !== null) result[key] = value;
    } catch {
      // 忽略 localStorage 不可用
    }
  }
  return result;
}

function writeAppearance(appearance: Record<string, string> | undefined): void {
  if (!appearance) return;
  for (const key of APPEARANCE_KEYS) {
    const value = appearance[key];
    if (typeof value === 'string') {
      try {
        localStorage.setItem(key, value);
      } catch {
        // 忽略
      }
    }
  }
}

/** 导出设置到文件，返回路径（用户取消返回 null） */
export async function exportSettings(): Promise<string | null> {
  const path = await save({ defaultPath: 'arkdesk-settings.json', filters: FILE_FILTERS });
  if (!path) return null;
  const payload = {
    _version: 1,
    settings: await useSettingsStore().snapshot(),
    appearance: readAppearance(),
  };
  await ipc('file_write_text', { path, contents: JSON.stringify(payload, null, 2) });
  logger.info(`设置已导出: ${path}`);
  return path;
}

/** 从文件导入设置（写回存储；需重启生效），成功返回 true */
export async function importSettings(): Promise<boolean> {
  const selected = await open({ multiple: false, filters: FILE_FILTERS });
  if (typeof selected !== 'string') return false;
  const text = await ipc<string>('file_read_text', { path: selected });
  const parsed = JSON.parse(text) as Record<string, unknown>;
  const data = (parsed.settings as Record<string, unknown> | undefined) ?? parsed;
  await useSettingsStore().importRaw(data);
  writeAppearance(parsed.appearance as Record<string, string> | undefined);
  logger.info(`设置已导入: ${selected}`);
  return true;
}

/** 重启应用（导入设置 / 恢复数据后生效） */
export async function restartApp(): Promise<void> {
  await ipc('app_restart');
}
