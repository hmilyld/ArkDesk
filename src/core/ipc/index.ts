/**
 * IPC 调用统一封装。
 *
 * - 调用前后写 debug / error 日志
 * - 将 Rust 侧错误统一转换为 AppError 再抛出
 * - 未被业务层捕获的错误会由全局 unhandledrejection 兜底提示
 */
import { invoke } from '@tauri-apps/api/core';
import { logger } from '@/core/logger';
import { normalizeError } from '@/core/errors';
import type { CommandName } from './commands.gen';

export type { CommandName };

/** 所有前端 → Rust 的调用都应走此函数，禁止直接使用裸 invoke。
 *  `command` 受 `commands.gen.ts`（构建期生成）约束，非注册命令名会在类型检查期报错。 */
export async function ipc<T>(command: CommandName, args?: Record<string, unknown>): Promise<T> {
  logger.debug(`ipc → ${command}`);
  try {
    const result = await invoke<T>(command, args);
    return result;
  } catch (err) {
    const error = normalizeError(err);
    logger.error(`ipc ← ${command} failed: [${error.code}] ${error.message}`);
    throw error;
  }
}
