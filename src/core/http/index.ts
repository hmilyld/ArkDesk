/**
 * 通用 HTTP 客户端（网页采集 / API 调用的统一入口）。
 *
 * - 请求由 Rust 侧 reqwest 发起：无 CORS 限制、共享 cookie 会话、
 *   统一 UA / 超时 / 重定向 / 响应大小限制
 * - 错误经 ipc 封装统一转换（AppError），未捕获时自动日志 + toast
 * - JSON 优先场景直接使用 getJson / postJson
 */

import { ipc } from '@/core/ipc';
import { ErrorCode, normalizeError } from '@/core/errors';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { HttpEvent, onEvent } from '@/core/events';

export interface HttpRequestOptions {
  headers?: Record<string, string>;
  query?: Record<string, string>;
  /** 请求体文本（JSON 请用 getJson / postJson，或自行 JSON.stringify） */
  body?: string;
  /** 单次请求超时（毫秒），缺省 30s */
  timeoutMs?: number;
}

export interface HttpResponse {
  status: number;
  ok: boolean;
  /** 重定向后的最终 URL */
  finalUrl: string;
  headers: Record<string, string>;
  body: string;
  elapsedMs: number;
}

/** 底层通用请求：所有便捷方法的最终出口 */
export async function request(
  method: string,
  url: string,
  options: HttpRequestOptions = {}
): Promise<HttpResponse> {
  return ipc<HttpResponse>('http_request', {
    args: {
      method,
      url,
      headers: options.headers,
      query: options.query,
      body: options.body,
      timeoutMs: options.timeoutMs,
    },
  });
}

function parseJson<T>(response: HttpResponse): T {
  try {
    return JSON.parse(response.body) as T;
  } catch {
    throw normalizeError({
      code: ErrorCode.InvalidInput,
      message: `响应不是合法 JSON（status=${response.status}）`,
      details: response.body.slice(0, 500),
    });
  }
}

/** 名称-值对（保序，允许重复键） */
export interface HttpNameValue {
  name: string;
  value: string;
}

/** multipart 单个 part（文本字段或文件） */
export interface HttpMultipartPart {
  name: string;
  value?: string;
  filePath?: string;
  fileName?: string;
  contentType?: string;
}

/** 高级请求体（对应 Rust `HttpSendBody`，按 type 打标签） */
export type HttpSendBody =
  | { type: 'raw'; text: string; contentType?: string }
  | { type: 'form'; fields: HttpNameValue[] }
  | { type: 'multipart'; parts: HttpMultipartPart[] }
  | { type: 'binary'; path: string; contentType?: string };

/** `http.send` 请求参数（对应 Rust `HttpSendArgs`） */
export interface HttpSendOptions {
  method?: string;
  url: string;
  headers?: HttpNameValue[];
  query?: HttpNameValue[];
  /** 原始 Cookie 头（可选便捷项） */
  cookies?: string;
  body?: HttpSendBody;
  /** `follow`（默认） | `manual` */
  redirect?: 'follow' | 'manual';
  verifySsl?: boolean;
  timeoutMs?: number;
  userAgent?: string;
  /** 代理覆盖：不传 = 沿用框架全局；空串 = 直连；其他 = 覆盖 */
  proxy?: string;
  /** `none`（默认，不自动管理） | `jar`（共享 cookie jar） */
  cookieMode?: 'none' | 'jar';
  maxBodyBytes?: number;
  /** 取消任务 id（配合 `http.cancel`） */
  taskId?: string;
}

export interface HttpHeaderEntry {
  name: string;
  value: string;
}

/** `http.send` 响应（对应 Rust `HttpSendResult`） */
export interface HttpSendResponse {
  status: number;
  statusText: string;
  ok: boolean;
  finalUrl: string;
  headers: HttpHeaderEntry[];
  contentType?: string;
  bodyText?: string;
  bodyBase64?: string;
  isBinary: boolean;
  sizeBytes: number;
  contentLength?: number;
  elapsedMs: number;
  truncated: boolean;
  redirectLocation?: string;
}

/** 高级通用请求：参数化重定向 / SSL / 超时 / 代理 / cookie，支持 multipart 与二进制响应 */
export async function send(options: HttpSendOptions): Promise<HttpSendResponse> {
  return ipc<HttpSendResponse>('http_send', { args: options });
}

/** 取消进行中的 `http.send`（依据 `taskId`） */
export async function cancel(taskId: string): Promise<void> {
  await ipc('task_cancel', { taskId });
}

export interface DownloadOptions {
  headers?: Record<string, string>;
  onProgress?: (progress: { downloaded: number; total: number | null }) => void;
}

/** 流式下载到文件（进度经回调回传），返回保存路径 */
export async function download(
  url: string,
  path: string,
  options: DownloadOptions = {}
): Promise<string> {
  let unlisten: UnlistenFn | null = null;
  if (options.onProgress) {
    unlisten = await onEvent(HttpEvent.DownloadProgress, (payload) => {
      if (payload.url === url) {
        options.onProgress?.({ downloaded: payload.downloaded, total: payload.total });
      }
    });
  }
  try {
    return await ipc<string>('http_download', {
      args: { url, path, headers: options.headers },
    });
  } finally {
    unlisten?.();
  }
}

export const http = {
  request,
  send,
  cancel,
  download,

  get: (url: string, options?: HttpRequestOptions): Promise<HttpResponse> =>
    request('GET', url, options),

  post: (url: string, body: string, options?: HttpRequestOptions): Promise<HttpResponse> =>
    request('POST', url, { ...options, body }),

  /** GET 并将响应解析为 JSON */
  getJson: async <T>(url: string, options?: HttpRequestOptions): Promise<T> => {
    const response = await http.get(url, options);
    return parseJson<T>(response);
  },

  /** 以 JSON 发送请求体，并将响应解析为 JSON */
  postJson: async <T>(url: string, data: unknown, options?: HttpRequestOptions): Promise<T> => {
    const response = await request('POST', url, {
      ...options,
      headers: { 'Content-Type': 'application/json', ...options?.headers },
      body: JSON.stringify(data),
    });
    return parseJson<T>(response);
  },
} as const;
