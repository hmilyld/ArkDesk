/**
 * network-tools 共享类型 / 常量 / 纯函数。
 *
 * - wire 模型（`HttpRequestSpec` / `HttpResponseView`）与发送、未来的「请求拦截」共用
 * - `buildSendOptions` 把编辑态草稿翻译成框架 `http.send` 参数（含 Auth/Cookie 合成规则）
 * - 数据库行 ↔ 草稿的序列化/反序列化
 */

import type { HttpSendBody, HttpSendOptions, HttpSendResponse } from '@/core/http';

// ─────────────────────────── 数据模型 ───────────────────────────

/** 可启用/重复的键值行 */
export interface HttpNameValue {
  id: string;
  name: string;
  value: string;
  enabled: boolean;
}

export type HttpBodyType = 'none' | 'raw' | 'form' | 'multipart' | 'binary';
export type HttpRawLang = 'json' | 'text' | 'xml' | 'html';

/** multipart 行：文本字段或文件 */
export interface HttpMultipartRow {
  id: string;
  name: string;
  kind: 'text' | 'file';
  value: string;
  filePath: string;
  fileName: string;
  contentType: string;
  enabled: boolean;
}

export interface HttpBody {
  type: HttpBodyType;
  raw: string;
  rawLang: HttpRawLang;
  form: HttpNameValue[];
  multipart: HttpMultipartRow[];
  binaryPath: string;
  binaryContentType: string;
}

export type HttpAuthType = 'none' | 'basic' | 'bearer' | 'apikey';
export type HttpAuthLocation = 'header' | 'query';

export interface HttpAuth {
  type: HttpAuthType;
  basicUsername: string;
  basicPassword: string;
  bearerToken: string;
  apiKeyName: string;
  apiKeyValue: string;
  apiKeyIn: HttpAuthLocation;
}

/** 编辑态请求草稿（发送 / 保存 / 导入导出 / 未来拦截共用） */
export interface HttpRequestSpec {
  method: string;
  url: string;
  query: HttpNameValue[];
  headers: HttpNameValue[];
  cookies: HttpNameValue[];
  body: HttpBody;
  auth: HttpAuth;
}

/** 响应视图（框架 `http.send` 返回结构，保留完整信息） */
export type HttpResponseView = HttpSendResponse;

/** 插件级设置 */
export interface HttpSettings {
  timeoutMs: number;
  followRedirects: boolean;
  verifySsl: boolean;
  /** 响应体截断阈值（KB）；0 = 用框架默认 */
  maxResponseKb: number;
  historyEnabled: boolean;
  historyLimit: number;
  proxyMode: 'global' | 'custom';
  proxyUrl: string;
  defaultUserAgent: string;
  activeEnvironmentId: number | null;
  // 请求拦截
  interceptorPort: number;
  interceptorRecordBodies: boolean;
  interceptorMaxBodyKb: number;
  interceptorMaxFlows: number;
  interceptorMaxWsFrames: number;
  interceptorAutoSystemProxy: boolean;
}

export const DEFAULT_SETTINGS: HttpSettings = {
  timeoutMs: 30000,
  followRedirects: true,
  verifySsl: true,
  maxResponseKb: 10240,
  historyEnabled: true,
  historyLimit: 500,
  proxyMode: 'global',
  proxyUrl: '',
  defaultUserAgent: '',
  activeEnvironmentId: null,
  interceptorPort: 0,
  interceptorRecordBodies: true,
  interceptorMaxBodyKb: 1024,
  interceptorMaxFlows: 300,
  interceptorMaxWsFrames: 500,
  interceptorAutoSystemProxy: true,
};

export const HTTP_METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as const;

export const RAW_LANG_CONTENT_TYPES: Record<HttpRawLang, string> = {
  json: 'application/json',
  text: 'text/plain',
  xml: 'application/xml',
  html: 'text/html',
};

// ─────────────────────────── 构造 / 工具 ───────────────────────────

/** 生成局部唯一 id（仅用于列表 key / 关联，不入库） */
export function uid(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

export function kv(name = '', value = '', enabled = true): HttpNameValue {
  return { id: uid(), name, value, enabled };
}

export function multipartRow(patch: Partial<HttpMultipartRow> = {}): HttpMultipartRow {
  return {
    id: uid(),
    name: '',
    kind: 'text',
    value: '',
    filePath: '',
    fileName: '',
    contentType: '',
    enabled: true,
    ...patch,
  };
}

export function createBody(): HttpBody {
  return {
    type: 'none',
    raw: '',
    rawLang: 'json',
    form: [],
    multipart: [],
    binaryPath: '',
    binaryContentType: '',
  };
}

export function createAuth(): HttpAuth {
  return {
    type: 'none',
    basicUsername: '',
    basicPassword: '',
    bearerToken: '',
    apiKeyName: '',
    apiKeyValue: '',
    apiKeyIn: 'header',
  };
}

export function createRequestSpec(): HttpRequestSpec {
  return {
    method: 'GET',
    url: '',
    query: [],
    headers: [],
    cookies: [],
    body: createBody(),
    auth: createAuth(),
  };
}

export function cloneSpec(spec: HttpRequestSpec): HttpRequestSpec {
  return JSON.parse(JSON.stringify(spec)) as HttpRequestSpec;
}

/** 将任意来源（历史 / 导入文件）的请求对象补全为完整草稿，避免脏数据导致渲染崩溃 */
export function normalizeSpec(value: unknown): HttpRequestSpec {
  const base = createRequestSpec();
  if (!value || typeof value !== 'object') return base;
  const input = value as Partial<HttpRequestSpec>;
  return {
    ...base,
    ...input,
    query: Array.isArray(input.query) ? input.query : [],
    headers: Array.isArray(input.headers) ? input.headers : [],
    cookies: Array.isArray(input.cookies) ? input.cookies : [],
    body: { ...base.body, ...(input.body ?? {}) },
    auth: { ...base.auth, ...(input.auth ?? {}) },
  };
}

// ─────────────────────────── Cookie ───────────────────────────

/** 解析原始 Cookie 头为行 */
export function parseCookieHeader(raw: string): HttpNameValue[] {
  return raw
    .split(';')
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const eq = part.indexOf('=');
      return kv(
        eq >= 0 ? part.slice(0, eq).trim() : part.trim(),
        eq >= 0 ? part.slice(eq + 1).trim() : ''
      );
    });
}

/** 行合成原始 Cookie 头 */
export function buildCookieHeader(rows: HttpNameValue[]): string {
  return rows
    .filter((row) => row.enabled && row.name.trim())
    .map((row) => `${row.name.trim()}=${row.value}`)
    .join('; ');
}

// ─────────────────────────── 发送参数构建 ───────────────────────────

export interface BuildSendResult {
  options: HttpSendOptions;
  /** 非阻断警告（如 Auth 与手写头冲突） */
  warnings: string[];
}

function base64EncodeUtf8(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary);
}

function hasHeader(headers: HttpNameValue[], name: string): boolean {
  const lower = name.trim().toLowerCase();
  return headers.some((header) => header.name.trim().toLowerCase() === lower);
}

function buildBody(body: HttpBody): HttpSendBody | undefined {
  switch (body.type) {
    case 'raw':
      return body.raw
        ? { type: 'raw', text: body.raw, contentType: RAW_LANG_CONTENT_TYPES[body.rawLang] }
        : undefined;
    case 'form': {
      const fields = body.form
        .filter((field) => field.enabled && field.name.trim())
        .map((field) => ({ name: field.name.trim(), value: field.value }));
      return fields.length ? { type: 'form', fields } : undefined;
    }
    case 'multipart': {
      const parts = body.multipart
        .filter((part) => part.enabled && part.name.trim())
        .map((part) =>
          part.kind === 'file'
            ? {
                name: part.name.trim(),
                filePath: part.filePath,
                fileName: part.fileName || undefined,
                contentType: part.contentType || undefined,
              }
            : { name: part.name.trim(), value: part.value }
        );
      return parts.length ? { type: 'multipart', parts } : undefined;
    }
    case 'binary':
      return body.binaryPath
        ? {
            type: 'binary',
            path: body.binaryPath,
            contentType: body.binaryContentType || undefined,
          }
        : undefined;
    default:
      return undefined;
  }
}

/**
 * 草稿 + 设置 → 框架 `http.send` 参数。
 * 规则：CookieEditor 为唯一真源（手动 Cookie 头会并入并移除）；Auth 发送时合成，手写同名头优先。
 */
export function buildSendOptions(
  spec: HttpRequestSpec,
  settings: HttpSettings,
  taskId?: string
): BuildSendResult {
  const warnings: string[] = [];
  const headers = spec.headers
    .filter((header) => header.enabled && header.name.trim())
    .map((header) => ({ ...header }));

  let cookieHeader = buildCookieHeader(spec.cookies);
  const manualCookie = headers.find((header) => header.name.trim().toLowerCase() === 'cookie');
  if (manualCookie) {
    cookieHeader = [manualCookie.value, cookieHeader].filter(Boolean).join('; ');
    const index = headers.indexOf(manualCookie);
    if (index >= 0) headers.splice(index, 1);
    warnings.push('检测到手动 Cookie 头，已与 Cookie 编辑器内容合并发送');
  }

  const auth = spec.auth;
  if (auth.type === 'basic') {
    if (hasHeader(headers, 'authorization'))
      warnings.push('已手写 Authorization 头，忽略 Basic 认证');
    else
      headers.push(
        kv(
          'Authorization',
          `Basic ${base64EncodeUtf8(`${auth.basicUsername}:${auth.basicPassword}`)}`
        )
      );
  } else if (auth.type === 'bearer') {
    if (hasHeader(headers, 'authorization'))
      warnings.push('已手写 Authorization 头，忽略 Bearer 认证');
    else headers.push(kv('Authorization', `Bearer ${auth.bearerToken}`));
  } else if (auth.type === 'apikey' && auth.apiKeyIn === 'header' && auth.apiKeyName.trim()) {
    if (hasHeader(headers, auth.apiKeyName))
      warnings.push(`已手写 ${auth.apiKeyName} 头，忽略 API Key 认证`);
    else headers.push(kv(auth.apiKeyName.trim(), auth.apiKeyValue));
  }

  const query = spec.query
    .filter((row) => row.enabled && row.name.trim())
    .map((row) => ({ ...row }));
  if (auth.type === 'apikey' && auth.apiKeyIn === 'query' && auth.apiKeyName.trim()) {
    query.push(kv(auth.apiKeyName.trim(), auth.apiKeyValue));
  }

  return {
    options: {
      method: spec.method,
      url: spec.url,
      headers: headers.map(({ name, value }) => ({ name: name.trim(), value })),
      query: query.map(({ name, value }) => ({ name: name.trim(), value })),
      cookies: cookieHeader || undefined,
      body: buildBody(spec.body),
      redirect: settings.followRedirects ? 'follow' : 'manual',
      verifySsl: settings.verifySsl,
      timeoutMs: settings.timeoutMs,
      userAgent: settings.defaultUserAgent.trim() || undefined,
      proxy: settings.proxyMode === 'custom' ? settings.proxyUrl : undefined,
      cookieMode: 'none',
      maxBodyBytes: settings.maxResponseKb > 0 ? settings.maxResponseKb * 1024 : undefined,
      taskId,
    },
    warnings,
  };
}

// ─────────────────────────── 数据库行 ↔ 草稿 ───────────────────────────

export interface RequestRow {
  id: number;
  collection_id: number | null;
  name: string;
  method: string;
  url: string;
  query: string;
  headers: string;
  cookies: string;
  body_type: string;
  body_text: string;
  body_lang: string;
  form_fields: string;
  multipart_fields: string;
  binary_path: string;
  auth_type: string;
  auth_config: string;
  sort_order: number;
}

export interface RequestMeta {
  id: number | null;
  name: string;
  collectionId: number | null;
}

function parseJson<T>(text: string | null | undefined, fallback: T): T {
  if (!text) return fallback;
  try {
    return JSON.parse(text) as T;
  } catch {
    return fallback;
  }
}

export function specToRecord(spec: HttpRequestSpec, meta: RequestMeta): Record<string, unknown> {
  return {
    collection_id: meta.collectionId,
    name: meta.name,
    method: spec.method,
    url: spec.url,
    query: JSON.stringify(spec.query),
    headers: JSON.stringify(spec.headers),
    cookies: JSON.stringify(spec.cookies),
    body_type: spec.body.type,
    body_text: spec.body.raw,
    body_lang: spec.body.rawLang,
    form_fields: JSON.stringify(spec.body.form),
    multipart_fields: JSON.stringify(spec.body.multipart),
    binary_path: spec.body.binaryPath,
    auth_type: spec.auth.type,
    auth_config: JSON.stringify(spec.auth),
    updated_at: nowSql(),
  };
}

export function recordToSpec(row: RequestRow): HttpRequestSpec {
  const spec = createRequestSpec();
  spec.method = row.method || 'GET';
  spec.url = row.url ?? '';
  spec.query = parseJson(row.query, []);
  spec.headers = parseJson(row.headers, []);
  spec.cookies = parseJson(row.cookies, []);
  spec.body.type = (row.body_type as HttpBodyType) || 'none';
  spec.body.raw = row.body_text ?? '';
  spec.body.rawLang = (row.body_lang as HttpRawLang) || 'json';
  spec.body.form = parseJson(row.form_fields, []);
  spec.body.multipart = parseJson(row.multipart_fields, []);
  spec.body.binaryPath = row.binary_path ?? '';
  spec.auth = { ...createAuth(), ...parseJson(row.auth_config, {} as Partial<HttpAuth>) };
  return spec;
}

/** SQLite localtime 字符串（与迁移默认值格式一致） */
export function nowSql(): string {
  const date = new Date();
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(
    date.getHours()
  )}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

// ─────────────────────────── 展示辅助 ───────────────────────────

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) return '—';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

export function formatDuration(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return '—';
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(2)} s`;
}

/** HTTP 方法的语义色 class */
export function httpMethodClass(method: string): string {
  switch (method.toUpperCase()) {
    case 'GET':
      return 'text-success';
    case 'POST':
      return 'text-info';
    case 'PUT':
    case 'PATCH':
      return 'text-warning';
    case 'DELETE':
      return 'text-destructive';
    default:
      return 'text-muted-foreground';
  }
}

export type BadgeVariant = 'default' | 'destructive' | 'secondary';

/** HTTP 状态码 → Badge 变体（null 未完成 / 0 失败） */
export function statusBadgeVariant(status: number | null): BadgeVariant {
  if (status === null) return 'secondary';
  if (status === 0) return 'destructive';
  if (status >= 200 && status < 300) return 'default';
  if (status >= 400) return 'destructive';
  return 'secondary';
}

/** URL 的 path + query（解析失败原样返回） */
export function urlPath(url: string): string {
  try {
    const parsed = new URL(url);
    return `${parsed.pathname}${parsed.search}`;
  } catch {
    return url;
  }
}

/** 复制文本到剪贴板（浏览器环境） */
export async function copyToClipboard(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}

/** JSON 美化（失败原样返回） */
function prettyJson(text: string | undefined): string {
  if (!text) return '';
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

const MARKUP_VOID = new Set([
  'area',
  'base',
  'br',
  'col',
  'embed',
  'hr',
  'img',
  'input',
  'link',
  'meta',
  'param',
  'source',
  'track',
  'wbr',
]);
const MARKUP_RAW = new Set(['pre', 'script', 'style', 'textarea']);

/** XML / HTML 缩进美化（基于 DOMParser；失败或非浏览器环境原样返回） */
export function prettyMarkup(text: string, xml: boolean): string {
  if (typeof DOMParser === 'undefined') return text;
  let doc: Document;
  try {
    doc = new DOMParser().parseFromString(text, xml ? 'application/xml' : 'text/html');
  } catch {
    return text;
  }
  if (xml && (doc.getElementsByTagName('parsererror').length > 0 || !doc.documentElement)) {
    return text;
  }
  // 整棵 DOM（含 head/script/style），而非仅 body
  const root = doc.documentElement ?? doc.body;
  if (!root) return text;

  const lines: string[] = [];
  if (!xml && doc.doctype) lines.push('<!DOCTYPE html>');
  const indent = (depth: number) => '  '.repeat(depth);
  const escape = (value: string) =>
    value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

  const walk = (node: Node, depth: number): void => {
    if (node.nodeType === Node.TEXT_NODE) {
      const content = (node.textContent ?? '').trim();
      if (content) lines.push(indent(depth) + escape(content));
      return;
    }
    if (node.nodeType === Node.COMMENT_NODE) {
      lines.push(`${indent(depth)}<!--${node.textContent ?? ''}-->`);
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return;

    const el = node as Element;
    const tag = xml ? el.tagName : el.tagName.toLowerCase();
    if (!xml && MARKUP_RAW.has(tag)) {
      lines.push(indent(depth) + el.outerHTML.trim());
      return;
    }
    const attrs = Array.from(el.attributes)
      .map((attr) => ` ${attr.name}="${escape(attr.value)}"`)
      .join('');
    const children = Array.from(el.childNodes);
    const hasElementChildren = children.some(
      (child) => child.nodeType === Node.ELEMENT_NODE || child.nodeType === Node.COMMENT_NODE
    );
    if (!hasElementChildren) {
      const content = (el.textContent ?? '').trim();
      if (!xml && MARKUP_VOID.has(tag) && content === '') {
        lines.push(indent(depth) + `<${tag}${attrs}>`);
      } else if (content === '') {
        lines.push(indent(depth) + `<${tag}${attrs}/>`);
      } else {
        lines.push(indent(depth) + `<${tag}${attrs}>${escape(content)}</${tag}>`);
      }
      return;
    }
    lines.push(indent(depth) + `<${tag}${attrs}>`);
    for (const child of children) walk(child, depth + 1);
    lines.push(indent(depth) + `</${tag}>`);
  };

  walk(root, 0);
  const output = lines.join('\n');
  // 任何异常情况都退回原文，避免展示成“空响应体”
  return output || text;
}

function prettyMarkupSafe(text: string, xml: boolean): string {
  try {
    return prettyMarkup(text, xml);
  } catch {
    return text;
  }
}

/** 按内容类型/内容特征自动美化：JSON / XML / HTML，其余原样返回 */
export function prettyBody(text: string, contentType?: string): string {
  if (!text) return '';
  const ct = (contentType ?? '').toLowerCase();
  const trimmed = text.trimStart();

  if (ct.includes('json') || trimmed.startsWith('{') || trimmed.startsWith('[')) {
    const formatted = prettyJson(text);
    if (formatted !== text) return formatted;
  }
  if (ct.includes('xml') || /^<\?xml[\s>]/i.test(trimmed)) {
    return prettyMarkupSafe(text, true);
  }
  if (ct.includes('html') || /^<!doctype html|^<html[\s>]/i.test(trimmed)) {
    return prettyMarkupSafe(text, false);
  }
  return text;
}
