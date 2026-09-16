/**
 * 错误信息格式化（依赖核心错误模块，供组件/composable 使用）。
 * 单独成文件，避免纯逻辑模块（shared/curl/variables）引入 Tauri 依赖，便于单测在 node 环境运行。
 */

import { normalizeError } from '@/core/errors';

export function errorMessage(err: unknown): string {
  const error = normalizeError(err);
  return `[${error.code}] ${error.message}`;
}
