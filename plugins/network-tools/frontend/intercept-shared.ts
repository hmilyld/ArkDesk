/**
 * 请求拦截：前端类型 / 纯逻辑（过滤、捕获流量 → 请求草稿）。
 *
 * 纯逻辑，无 Tauri 依赖，便于 vitest 单测。
 */

import {
  createRequestSpec,
  kv,
  urlPath,
  type HttpRequestSpec,
  type HttpResponseView,
} from './shared';

export interface HeaderPair {
  name: string;
  value: string;
}

export interface FlowSummary {
  id: number;
  startedAt: number;
  method: string;
  scheme: string;
  host: string;
  url: string;
  status: number | null;
  durationMs: number | null;
  reqSize: number;
  resSize: number;
  error: string | null;
  isUpgrade: boolean;
  contentType: string | null;
}

export interface BodyCapture {
  text: string | null;
  base64: string | null;
  isBinary: boolean;
  size: number;
  truncated: boolean;
  contentType: string | null;
  decoded: boolean;
  note: string | null;
}

export interface FlowRecord {
  summary: FlowSummary;
  clientAddr: string;
  reqHeaders: HeaderPair[];
  resHeaders: HeaderPair[];
  reqBody: BodyCapture | null;
  resBody: BodyCapture | null;
}

export interface WsFrame {
  dir: string;
  opcode: string;
  len: number;
  text: string | null;
  at: number;
}

export interface WsSummary {
  id: number;
  host: string;
  url: string;
  frameCount: number;
  startedAt: number;
  truncated: boolean;
}

export interface WsRecord {
  summary: WsSummary;
  frames: WsFrame[];
}

export interface CaInfo {
  exists: boolean;
  fingerprint: string | null;
  notAfter: string | null;
  certPath: string | null;
}

export interface ProxyStatus {
  running: boolean;
  port: number | null;
  flowCount: number;
  wsCount: number;
  ca: CaInfo;
}

export interface SystemProxyStatus {
  supported: boolean;
  enabled: boolean;
  detail: string;
}

export interface ProxyConfig {
  recordBodies: boolean;
  maxBodyKb: number;
  maxFlows: number;
  maxWsFrames: number;
}

export interface FlowFilter {
  host: string;
  method: string;
  status: string;
  type: string;
  url: string;
}

export type ResourceKind = 'doc' | 'js' | 'css' | 'img' | 'font' | 'media' | 'ws' | 'xhr' | 'other';

export const KIND_LABELS: Record<ResourceKind, string> = {
  doc: 'html',
  js: 'js',
  css: 'css',
  img: 'img',
  font: 'font',
  media: 'media',
  ws: 'ws',
  xhr: 'json',
  other: 'other',
};

export const ALL_METHODS = '__all__';
export const ALL_STATUS = '__all__';
export const ALL_TYPES = '__all__';
export const EMPTY_FILTER: FlowFilter = {
  host: '',
  method: ALL_METHODS,
  status: ALL_STATUS,
  type: ALL_TYPES,
  url: '',
};

export const STATUS_OPTIONS = [
  { value: ALL_STATUS, label: '全部状态' },
  { value: '2xx', label: '2xx' },
  { value: '3xx', label: '3xx' },
  { value: '4xx', label: '4xx' },
  { value: '5xx', label: '5xx' },
  { value: 'err', label: '错误' },
];

export const TYPE_OPTIONS: { value: string; label: string }[] = [
  { value: ALL_TYPES, label: '全部类型' },
  { value: 'doc', label: 'Doc (HTML)' },
  { value: 'js', label: 'JS' },
  { value: 'css', label: 'CSS' },
  { value: 'img', label: '图片' },
  { value: 'font', label: '字体' },
  { value: 'media', label: '媒体' },
  { value: 'xhr', label: 'XHR/JSON' },
  { value: 'ws', label: 'WebSocket' },
  { value: 'other', label: '其他' },
];

/** 按 Content-Type / 扩展名对流量分类（对齐 DevTools 常见分类） */
export function classifyFlow(flow: FlowSummary): ResourceKind {
  if (flow.isUpgrade) return 'ws';
  const ct = (flow.contentType ?? '').toLowerCase();
  if (ct) {
    if (ct.includes('text/html')) return 'doc';
    if (ct.includes('text/css')) return 'css';
    if (ct.includes('javascript') || ct.includes('ecmascript')) return 'js';
    if (ct.startsWith('image/')) return 'img';
    if (ct.includes('font') || ct.includes('woff')) return 'font';
    if (ct.startsWith('audio/') || ct.startsWith('video/')) return 'media';
    if (
      ct.includes('json') ||
      ct.includes('xml') ||
      ct.includes('x-www-form-urlencoded') ||
      ct.includes('graphql')
    ) {
      return 'xhr';
    }
    // text/plain 之类不归入 JSON/XHR，避免重定向体等被误分
    return 'other';
  }
  const path = (urlPath(flow.url).split('?')[0] ?? flow.url).toLowerCase();
  if (/\.(html?|php|asp)$/.test(path)) return 'doc';
  if (/\.(js|mjs|cjs)$/.test(path)) return 'js';
  if (/\.css$/.test(path)) return 'css';
  if (/\.(png|jpe?g|gif|webp|svg|ico|bmp|avif)$/.test(path)) return 'img';
  if (/\.(woff2?|ttf|otf|eot)$/.test(path)) return 'font';
  if (/\.(mp4|webm|mp3|wav|ogg|m4a|m3u8|ts)$/.test(path)) return 'media';
  return 'other';
}

function statusMatches(status: number | null, error: string | null, filter: string): boolean {
  if (!filter || filter === ALL_STATUS) return true;
  if (filter === 'err') return !!error;
  if (filter.endsWith('xx')) {
    const base = Number(filter[0]);
    if (!Number.isFinite(base) || status === null) return false;
    return Math.floor(status / 100) === base;
  }
  return String(status) === filter;
}

/** 依据过滤器筛选流量摘要 */
export function filterFlows(flows: FlowSummary[], filter: FlowFilter): FlowSummary[] {
  const host = filter.host.trim().toLowerCase();
  const method =
    filter.method === ALL_METHODS || !filter.method.trim()
      ? ''
      : filter.method.trim().toUpperCase();
  const url = filter.url.trim().toLowerCase();
  const type = filter.type;
  return flows.filter((flow) => {
    if (host && !flow.host.toLowerCase().includes(host)) return false;
    if (method && flow.method.toUpperCase() !== method) return false;
    if (url && !flow.url.toLowerCase().includes(url)) return false;
    if (type && type !== ALL_TYPES && classifyFlow(flow) !== type) return false;
    return statusMatches(flow.status, flow.error, filter.status);
  });
}

/// 重放时丢弃的头部：逐跳 / 长度 / 编码，交由客户端重新计算
const SKIPPED_HEADERS = new Set([
  'host',
  'content-length',
  'connection',
  'proxy-connection',
  'keep-alive',
  'transfer-encoding',
  'upgrade',
  'te',
  'trailer',
  'accept-encoding',
]);

/** 文本体已按 Content-Encoding 解码，重放该头会重复解压 */
function skipHeader(name: string, record: FlowRecord): boolean {
  const lower = name.trim().toLowerCase();
  if (SKIPPED_HEADERS.has(lower)) return true;
  if (lower === 'content-encoding' && !record.reqBody?.isBinary) return true;
  return false;
}

/** 捕获的请求 → 可编辑草稿（二进制体由调用方另行处理） */
export function flowToSpec(record: FlowRecord): HttpRequestSpec {
  const spec = createRequestSpec();
  spec.method = record.summary.method;
  spec.url = record.summary.url;
  spec.headers = record.reqHeaders
    .filter((header) => !skipHeader(header.name, record))
    .map((header) => kv(header.name, header.value));
  const body = record.reqBody;
  if (body && body.text !== null) {
    spec.body.type = 'raw';
    spec.body.raw = body.text;
    const ct = (body.contentType ?? '').toLowerCase();
    spec.body.rawLang = ct.includes('json')
      ? 'json'
      : ct.includes('xml')
        ? 'xml'
        : ct.includes('html')
          ? 'html'
          : 'text';
  }
  return spec;
}

/** 捕获的请求体是否为二进制（需落临时文件后重放） */
export function isBinaryRequest(record: FlowRecord): boolean {
  return !!record.reqBody?.isBinary;
}

/** 捕获的响应 → ResponsePanel 可用的视图 */
export function flowToResponseView(record: FlowRecord): HttpResponseView {
  const status = record.summary.status ?? 0;
  const body = record.resBody;
  return {
    status,
    statusText: '',
    ok: status >= 200 && status < 300,
    finalUrl: record.summary.url,
    headers: record.resHeaders,
    contentType: body?.contentType ?? undefined,
    bodyText: body && !body.isBinary ? (body.text ?? undefined) : undefined,
    bodyBase64: body?.isBinary ? (body.base64 ?? undefined) : undefined,
    isBinary: body?.isBinary ?? false,
    sizeBytes: body?.size ?? record.summary.resSize,
    elapsedMs: record.summary.durationMs ?? 0,
    truncated: body?.truncated ?? false,
  };
}

/** 时间戳（ms）→ HH:MM:SS */
export function formatClock(ms: number): string {
  const date = new Date(ms);
  const pad = (value: number) => String(value).padStart(2, '0');
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}
