/**
 * 全局日志模块。
 *
 * - Rust 侧通过 tauri-plugin-log 输出到 Stdout + 日志文件
 * - 前端通过 @tauri-apps/plugin-log 将日志写入同一管道
 * - 接管 console.error / warn / log，保证第三方库的输出也落盘
 *
 * 注意（回环防护）：日志插件的 Webview target 会把**所有** record（含前端
 * 发来的）经 `log://log` 事件回推给 webview。若 `attachConsole` 直接使用被
 * 接管的全局 console，会形成「回显 → 再次转发 → 再回显」的无限循环。
 * 因此这里不用 attachConsole，改用 attachLogger 自定义监听器 + `echoing`
 * 重入护栏：回显期间 console 仅打印、不再转发，从而打断循环且保留双通道。
 */
import {
  attachLogger,
  debug as logDebug,
  error as logError,
  info as logInfo,
  LogLevel,
  trace as logTrace,
  warn as logWarn,
} from '@tauri-apps/plugin-log';

let initialized = false;

/** 回显中标志：attachLogger 回调正在写回 webview 控制台时置位 */
let echoing = false;

/** 将任意值序列化为可读字符串（用于日志正文） */
function formatValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (value instanceof Error) {
    return value.stack ?? `${value.name}: ${value.message}`;
  }
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

function formatArgs(args: unknown[]): string {
  return args.map(formatValue).join(' ');
}

/** 接管 console，让所有前端输出统一进入日志管道（回显期间跳过转发） */
function hijackConsole(): void {
  const originalError = console.error.bind(console);
  const originalWarn = console.warn.bind(console);
  const originalLog = console.log.bind(console);

  console.error = (...args: unknown[]) => {
    originalError(...args);
    if (echoing) return;
    void logError(formatArgs(args));
  };
  console.warn = (...args: unknown[]) => {
    originalWarn(...args);
    if (echoing) return;
    void logWarn(formatArgs(args));
  };
  console.log = (...args: unknown[]) => {
    originalLog(...args);
    if (echoing) return;
    void logInfo(formatArgs(args));
  };
}

/** 订阅 Rust 侧日志事件并写入 webview 控制台（attachConsole 的受控替代） */
async function attachConsoleGuarded(): Promise<void> {
  await attachLogger(({ level, message }) => {
    echoing = true;
    try {
      switch (level) {
        case LogLevel.Trace:
          console.log(message);
          break;
        case LogLevel.Debug:
          console.debug(message);
          break;
        case LogLevel.Info:
          console.info(message);
          break;
        case LogLevel.Warn:
          console.warn(message);
          break;
        case LogLevel.Error:
          console.error(message);
          break;
        default:
          console.log(message);
      }
    } finally {
      echoing = false;
    }
  });
}

/** 应用启动时调用一次。重复调用是安全的（幂等）。 */
export async function initLogger(): Promise<void> {
  if (initialized) return;
  initialized = true;

  // 先订阅回显，再接管 console，保证回显期间（echoing=true）不转发
  await attachConsoleGuarded();
  hijackConsole();
  logDebug('logger initialized');
}

export const logger = {
  trace: (message: string) => void logTrace(message),
  debug: (message: string) => void logDebug(message),
  info: (message: string) => void logInfo(message),
  warn: (message: string) => void logWarn(message),
  error: (message: string) => void logError(message),
} as const;

export { formatValue };
