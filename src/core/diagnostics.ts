/**
 * 诊断报告导出（框架能力）。
 *
 * 生成环境信息 + 最近日志的文本报告，供用户反馈时附上。
 */
import { save } from '@tauri-apps/plugin-dialog';
import { ipc } from '@/core/ipc';
import { logger } from '@/core/logger';

/** 导出诊断报告，返回保存路径（用户取消返回 null） */
export async function exportDiagnostics(): Promise<string | null> {
  const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
  const path = await save({
    defaultPath: `pocketark-diagnostics-${stamp}.txt`,
    filters: [{ name: 'Text', extensions: ['txt'] }],
  });
  if (!path) return null;
  const saved = await ipc<string>('diagnostics_export', { path });
  logger.info(`诊断报告已导出: ${saved}`);
  return saved;
}
