/**
 * cURL 互转：从当前请求生成 curl，或粘贴浏览器 F12「Copy as cURL」反向填充。
 *
 * 解析覆盖常用参数：`-X` `-H` `-b` `-d`/`--data-raw`/`--data-binary` `-F` `-u` `-A` `-e` `--url`
 * 及 `--flag=value` 形式；忽略 `--compressed -L -k -s -o` 等无关参数。
 */

import type { HttpRawLang, HttpRequestSpec } from './shared';
import {
  RAW_LANG_CONTENT_TYPES,
  buildCookieHeader,
  createRequestSpec,
  kv,
  multipartRow,
  parseCookieHeader,
} from './shared';

const SAFE_SHELL_RE = /^[A-Za-z0-9_./:@%+=,-]+$/;

function shellQuote(value: string): string {
  if (value === '') return "''";
  if (SAFE_SHELL_RE.test(value)) return value;
  return `'${value.replace(/'/g, `'\\''`)}'`;
}

function guessRawLang(text: string, contentType: string | undefined): HttpRawLang {
  const ct = (contentType ?? '').toLowerCase();
  const trimmed = text.trim();
  if (ct.includes('json') || trimmed.startsWith('{') || trimmed.startsWith('[')) {
    try {
      JSON.parse(trimmed);
      return 'json';
    } catch {
      if (ct.includes('json')) return 'json';
    }
  }
  if (ct.includes('html')) return 'html';
  if (ct.includes('xml')) return 'xml';
  return 'text';
}

/** 生成 curl 命令（多行，`\` 续行） */
export function buildCurl(spec: HttpRequestSpec): string {
  const parts: string[] = ['curl'];
  const method = spec.method.toUpperCase();

  const queryRows = spec.query
    .filter((row) => row.enabled && row.name.trim())
    .map((row) => `${encodeURIComponent(row.name.trim())}=${encodeURIComponent(row.value)}`);
  if (
    spec.auth.type === 'apikey' &&
    spec.auth.apiKeyIn === 'query' &&
    spec.auth.apiKeyName.trim()
  ) {
    queryRows.push(
      `${encodeURIComponent(spec.auth.apiKeyName.trim())}=${encodeURIComponent(spec.auth.apiKeyValue)}`
    );
  }
  const query = queryRows.join('&');
  let url = spec.url;
  if (query) url += (url.includes('?') ? '&' : '?') + query;

  if (method !== 'GET') parts.push(`-X ${method}`);
  parts.push(shellQuote(url));

  const headers = spec.headers.filter((row) => row.enabled && row.name.trim());
  const hasContentType = headers.some((row) => row.name.trim().toLowerCase() === 'content-type');
  for (const row of headers) parts.push(`-H ${shellQuote(`${row.name.trim()}: ${row.value}`)}`);

  const body = spec.body;
  if (body.type === 'raw' && body.raw) {
    if (!hasContentType)
      parts.push(`-H ${shellQuote(`Content-Type: ${RAW_LANG_CONTENT_TYPES[body.rawLang]}`)}`);
    parts.push(`--data-raw ${shellQuote(body.raw)}`);
  } else if (body.type === 'form') {
    const encoded = body.form
      .filter((row) => row.enabled && row.name.trim())
      .map((row) => `${encodeURIComponent(row.name.trim())}=${encodeURIComponent(row.value)}`)
      .join('&');
    if (encoded) {
      if (!hasContentType) parts.push("-H 'Content-Type: application/x-www-form-urlencoded'");
      parts.push(`--data-raw ${shellQuote(encoded)}`);
    }
  } else if (body.type === 'multipart') {
    for (const part of body.multipart) {
      if (!part.enabled || !part.name.trim()) continue;
      if (part.kind === 'file' && part.filePath)
        parts.push(`-F ${shellQuote(`${part.name.trim()}=@${part.filePath}`)}`);
      else parts.push(`-F ${shellQuote(`${part.name.trim()}=${part.value}`)}`);
    }
  } else if (body.type === 'binary' && body.binaryPath) {
    parts.push(`--data-binary ${shellQuote(`@${body.binaryPath}`)}`);
  }

  const cookieHeader = buildCookieHeader(spec.cookies);
  if (cookieHeader) parts.push(`-b ${shellQuote(cookieHeader)}`);

  const auth = spec.auth;
  if (auth.type === 'basic')
    parts.push(`-u ${shellQuote(`${auth.basicUsername}:${auth.basicPassword}`)}`);
  else if (auth.type === 'bearer')
    parts.push(`-H ${shellQuote(`Authorization: Bearer ${auth.bearerToken}`)}`);
  else if (auth.type === 'apikey' && auth.apiKeyIn === 'header' && auth.apiKeyName.trim())
    parts.push(`-H ${shellQuote(`${auth.apiKeyName.trim()}: ${auth.apiKeyValue}`)}`);

  return parts.join(' \\\n  ');
}

/** curl 分词：处理引号、反斜杠转义与行尾续行（导出供测试） */
export function tokenizeCurl(input: string): string[] {
  const normalized = input.replace(/\\\r?\n/g, ' ');
  const tokens: string[] = [];
  let current = '';
  let quote: '"' | "'" | null = null;
  let i = 0;

  while (i < normalized.length) {
    const ch = normalized[i] as string;
    if (quote) {
      if (ch === quote) {
        quote = null;
        i += 1;
        continue;
      }
      if (ch === '\\' && quote === '"') {
        const next = normalized[i + 1];
        if (next !== undefined) {
          current += next;
          i += 2;
          continue;
        }
      }
      current += ch;
      i += 1;
      continue;
    }
    // 兼容 Chrome 的 $'...' ANSI-C 引用
    if (ch === '$' && normalized[i + 1] === "'") {
      quote = "'";
      i += 2;
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      i += 1;
      continue;
    }
    if (ch === '\\') {
      const next = normalized[i + 1];
      if (next !== undefined) {
        current += next;
        i += 2;
        continue;
      }
    }
    if (/\s/.test(ch)) {
      if (current) {
        tokens.push(current);
        current = '';
      }
      i += 1;
      continue;
    }
    current += ch;
    i += 1;
  }
  if (current) tokens.push(current);
  return tokens;
}

function flagValue(token: string): string | undefined {
  const eq = token.indexOf('=');
  return eq >= 0 ? token.slice(eq + 1) : undefined;
}

function addHeader(spec: HttpRequestSpec, raw: string): void {
  const colon = raw.indexOf(':');
  if (colon < 0) return;
  const name = raw.slice(0, colon).trim();
  const value = raw.slice(colon + 1).trim();
  if (!name) return;
  if (name.toLowerCase() === 'cookie') {
    spec.cookies.push(...parseCookieHeader(value));
    return;
  }
  spec.headers.push(kv(name, value));
}

function addCookie(spec: HttpRequestSpec, raw: string): void {
  spec.cookies.push(...parseCookieHeader(raw));
}

function addFormPart(spec: HttpRequestSpec, raw: string): void {
  const eq = raw.indexOf('=');
  const name = eq >= 0 ? raw.slice(0, eq) : raw;
  const value = eq >= 0 ? raw.slice(eq + 1) : '';
  spec.body.type = 'multipart';
  if (value.startsWith('@') || value.startsWith('<')) {
    // 形如 file=@/path;type=image/png;filename=x.png
    const [filePath, ...params] = value.slice(1).split(';');
    const contentType = params.find((item) => item.startsWith('type='))?.slice(5) ?? '';
    const fileName = params.find((item) => item.startsWith('filename='))?.slice(9) ?? '';
    spec.body.multipart.push(multipartRow({ name, kind: 'file', filePath, contentType, fileName }));
  } else {
    spec.body.multipart.push(multipartRow({ name, kind: 'text', value }));
  }
}

/** 已知「带值但被忽略」的 curl 参数：跳过其值，避免被误判为 URL */
const IGNORED_VALUE_FLAGS = new Set([
  '-o',
  '--output',
  '-c',
  '--cookie-jar',
  '-m',
  '--max-time',
  '--connect-timeout',
  '-x',
  '--proxy',
  '-w',
  '--write-out',
  '-T',
  '--upload-file',
  '--cacert',
  '--cert',
  '--key',
  '--resolve',
  '--retry',
  '--interface',
  '--limit-rate',
  '--proto',
]);

function isUrlLike(token: string): boolean {
  return (
    /^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(token) ||
    token.startsWith('/') ||
    token.startsWith('localhost')
  );
}

/** 解析 curl 文本为请求草稿 */
export function parseCurl(input: string): HttpRequestSpec {
  const tokens = tokenizeCurl(input);
  const spec = createRequestSpec();
  let methodSet = false;
  let hasBody = false;

  for (let i = 0; i < tokens.length; i += 1) {
    const token = tokens[i] as string;
    if (token === 'curl') continue;

    if (token === '-X' || token === '--request') {
      spec.method = (tokens[++i] ?? 'GET').toUpperCase();
      methodSet = true;
      continue;
    }
    if (token === '-H' || token === '--header') {
      addHeader(spec, tokens[++i] ?? '');
      continue;
    }
    if (token === '-b' || token === '--cookie') {
      addCookie(spec, tokens[++i] ?? '');
      continue;
    }
    if (
      token === '-d' ||
      token === '--data' ||
      token === '--data-raw' ||
      token === '--data-ascii' ||
      token === '--data-binary'
    ) {
      const value = tokens[++i] ?? '';
      if (token === '--data-binary' && value.startsWith('@')) {
        spec.body.type = 'binary';
        spec.body.binaryPath = value.slice(1);
      } else {
        spec.body.type = 'raw';
        spec.body.raw = value;
      }
      hasBody = true;
      continue;
    }
    if (token === '-F' || token === '--form') {
      addFormPart(spec, tokens[++i] ?? '');
      hasBody = true;
      continue;
    }
    if (token === '-u' || token === '--user') {
      const value = tokens[++i] ?? '';
      const colon = value.indexOf(':');
      spec.auth.type = 'basic';
      spec.auth.basicUsername = colon >= 0 ? value.slice(0, colon) : value;
      spec.auth.basicPassword = colon >= 0 ? value.slice(colon + 1) : '';
      continue;
    }
    if (token === '-A' || token === '--user-agent') {
      addHeader(spec, `User-Agent: ${tokens[++i] ?? ''}`);
      continue;
    }
    if (token === '-e' || token === '--referer') {
      addHeader(spec, `Referer: ${tokens[++i] ?? ''}`);
      continue;
    }
    if (token === '--url') {
      spec.url = tokens[++i] ?? '';
      continue;
    }
    if (token.startsWith('-X') && token.length > 2) {
      spec.method = token.slice(2).toUpperCase();
      methodSet = true;
      continue;
    }
    if (token.startsWith('--request=')) {
      spec.method = (flagValue(token) ?? 'GET').toUpperCase();
      methodSet = true;
      continue;
    }
    if (token.startsWith('--url=')) {
      spec.url = flagValue(token) ?? '';
      continue;
    }
    if (token.startsWith('--header=')) {
      addHeader(spec, flagValue(token) ?? '');
      continue;
    }
    if (token.startsWith('--cookie=')) {
      addCookie(spec, flagValue(token) ?? '');
      continue;
    }
    if (
      token.startsWith('--data-raw=') ||
      token.startsWith('--data=') ||
      token.startsWith('--data-binary=')
    ) {
      const value = flagValue(token) ?? '';
      if (token.startsWith('--data-binary=') && value.startsWith('@')) {
        spec.body.type = 'binary';
        spec.body.binaryPath = value.slice(1);
      } else {
        spec.body.type = 'raw';
        spec.body.raw = value;
      }
      hasBody = true;
      continue;
    }
    if (token.startsWith('--form=')) {
      addFormPart(spec, flagValue(token) ?? '');
      hasBody = true;
      continue;
    }
    if (token.startsWith('--user=')) {
      const value = flagValue(token) ?? '';
      const colon = value.indexOf(':');
      spec.auth.type = 'basic';
      spec.auth.basicUsername = colon >= 0 ? value.slice(0, colon) : value;
      spec.auth.basicPassword = colon >= 0 ? value.slice(colon + 1) : '';
      continue;
    }
    if (token.startsWith('--user-agent=')) {
      addHeader(spec, `User-Agent: ${flagValue(token) ?? ''}`);
      continue;
    }
    // 已知带值但忽略的参数：跳过其值
    if (IGNORED_VALUE_FLAGS.has(token)) {
      i += 1;
      continue;
    }
    if (token.startsWith('-')) continue;
    if (!spec.url && isUrlLike(token)) spec.url = token;
  }

  if (!methodSet) spec.method = hasBody ? 'POST' : 'GET';

  // 推断 raw 语言；表单编码体尽量还原为 form 行
  const contentType = spec.headers.find((row) => row.name.toLowerCase() === 'content-type')?.value;
  if (spec.body.type === 'raw') {
    if (
      contentType?.toLowerCase().includes('application/x-www-form-urlencoded') &&
      spec.body.raw.includes('=')
    ) {
      spec.body.type = 'form';
      spec.body.form = spec.body.raw
        .split('&')
        .filter(Boolean)
        .map((pair) => {
          const eq = pair.indexOf('=');
          const name = eq >= 0 ? pair.slice(0, eq) : pair;
          const value = eq >= 0 ? pair.slice(eq + 1) : '';
          return kv(safeDecode(name), safeDecode(value));
        });
    } else {
      spec.body.rawLang = guessRawLang(spec.body.raw, contentType);
    }
  }

  return spec;
}

function safeDecode(value: string): string {
  try {
    return decodeURIComponent(value.replace(/\+/g, ' '));
  } catch {
    return value;
  }
}
