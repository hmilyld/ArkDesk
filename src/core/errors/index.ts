/**
 * 全局错误处理模块。
 *
 * - AppError：前后端统一的错误结构（与 Rust 侧 error.rs 的序列化形态对齐）
 * - installErrorHandlers：接管 Vue errorHandler 与未处理的 Promise rejection
 * - setGlobalErrorReporter：由应用入口注册 UI 通知通道（如 toast）
 */
import type { App } from 'vue';
import { logger } from '@/core/logger';

/** 错误码常量，与 Rust 侧 error.rs 约定保持一致 */
export const ErrorCode = {
  Unknown: 'UNKNOWN',
  InvalidInput: 'INVALID_INPUT',
  IoError: 'IO_ERROR',
  DbError: 'DB_ERROR',
  IpcError: 'IPC_ERROR',
  NotFound: 'NOT_FOUND',
  PluginError: 'PLUGIN_ERROR',
  Http: 'HTTP_ERROR',
  Update: 'UPDATE_ERROR',
} as const;

export type ErrorCode = (typeof ErrorCode)[keyof typeof ErrorCode];

/** 与 Rust 侧 AppError 对齐的错误结构 */
export interface AppError {
  code: ErrorCode | string;
  message: string;
  details?: unknown;
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    'message' in value &&
    typeof (value as AppError).code === 'string' &&
    typeof (value as AppError).message === 'string'
  );
}

/** 将任意抛出值规范化为 AppError（Rust 错误、Error、字符串等） */
export function normalizeError(value: unknown): AppError {
  if (isAppError(value)) return value;
  if (value instanceof Error) {
    return { code: ErrorCode.Unknown, message: value.message, details: value.stack };
  }
  if (typeof value === 'string') {
    return { code: ErrorCode.Unknown, message: value };
  }
  return { code: ErrorCode.Unknown, message: '发生未知错误', details: value };
}

type ErrorReporter = (error: AppError) => void;

let reporter: ErrorReporter | null = null;

/** 由应用入口注册全局错误通知通道（toast 等），仅注册一次 */
export function setGlobalErrorReporter(fn: ErrorReporter): void {
  reporter = fn;
}

function report(error: AppError): void {
  logger.error(`[${error.code}] ${error.message}`);
  reporter?.(error);
}

/**
 * 安装全局异常处理：
 * - Vue 渲染/生命周期错误
 * - 未捕获的 Promise rejection
 */
export function installErrorHandlers(app: App): void {
  app.config.errorHandler = (err, instance, info) => {
    const error = normalizeError(err);
    report({
      ...error,
      message: `${error.message}（Vue: ${info}）`,
    });
    if (import.meta.env.DEV) {
      // 开发模式下保留默认行为，方便在控制台定位组件栈
      logger.debug(`errorHandler instance: ${instance?.$options?.name ?? 'anonymous'}`);
    }
  };

  app.config.warnHandler = (msg) => {
    logger.debug(`[vue-warn] ${msg}`);
  };

  window.addEventListener('unhandledrejection', (event) => {
    const error = normalizeError(event.reason);
    report(error);
    event.preventDefault();
  });
}
