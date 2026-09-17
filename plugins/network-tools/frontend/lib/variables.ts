/**
 * 环境变量替换：`{{var}}` 占位符。
 *
 * - 发送前对激活环境的变量做替换；未解析的变量会被收集，供调用方阻止发送
 * - 仅解析**已启用**的行/字段，禁用的内容不参与替换也不报缺失
 */

import type { HttpRequestSpec } from '../shared';
import { cloneSpec } from '../shared';

export interface EnvVarLike {
  key: string;
  value: string;
  enabled: boolean | number;
}

/** `{{ name }}`（name 不含花括号与空白） */
export const VARIABLE_RE = /\{\{\s*([^{}\s]+)\s*\}\}/g;

function isEnabled(value: boolean | number): boolean {
  return value === true || value === 1;
}

/** 启用变量 → key/value 映射（后者覆盖前者） */
export function collectVars(vars: EnvVarLike[]): Record<string, string> {
  const map: Record<string, string> = {};
  for (const item of vars) {
    if (!isEnabled(item.enabled)) continue;
    const key = item.key.trim();
    if (key) map[key] = item.value;
  }
  return map;
}

/** 单串替换；返回缺失变量名（去重） */
export function substitute(
  text: string,
  vars: Record<string, string>
): { text: string; missing: string[] } {
  const missing = new Set<string>();
  const output = text.replace(VARIABLE_RE, (match, name: string) => {
    if (Object.prototype.hasOwnProperty.call(vars, name)) return vars[name] ?? '';
    missing.add(name);
    return match;
  });
  return { text: output, missing: [...missing] };
}

export interface ResolveResult {
  spec: HttpRequestSpec;
  /** 未解析到的变量名（去重、排序）；非空应阻止发送 */
  missing: string[];
}

/** 深拷贝并替换草稿中所有启用字段的变量 */
export function resolveSpec(spec: HttpRequestSpec, vars: Record<string, string>): ResolveResult {
  const resolved = cloneSpec(spec);
  const missing = new Set<string>();
  const sub = (text: string): string => {
    const result = substitute(text, vars);
    result.missing.forEach((name) => missing.add(name));
    return result.text;
  };

  resolved.url = sub(resolved.url);

  for (const row of resolved.query) {
    if (row.enabled) {
      row.name = sub(row.name);
      row.value = sub(row.value);
    }
  }
  for (const row of resolved.headers) {
    if (row.enabled) {
      row.name = sub(row.name);
      row.value = sub(row.value);
    }
  }
  for (const row of resolved.cookies) {
    if (row.enabled) {
      row.name = sub(row.name);
      row.value = sub(row.value);
    }
  }

  const body = resolved.body;
  if (body.type === 'raw') {
    body.raw = sub(body.raw);
  } else if (body.type === 'form') {
    for (const field of body.form) {
      if (field.enabled) {
        field.name = sub(field.name);
        field.value = sub(field.value);
      }
    }
  } else if (body.type === 'multipart') {
    for (const part of body.multipart) {
      if (part.enabled) {
        part.name = sub(part.name);
        part.value = sub(part.value);
        part.fileName = sub(part.fileName);
        part.contentType = sub(part.contentType);
      }
    }
  } else if (body.type === 'binary') {
    body.binaryPath = sub(body.binaryPath);
    body.binaryContentType = sub(body.binaryContentType);
  }

  const auth = resolved.auth;
  if (auth.type === 'basic') {
    auth.basicUsername = sub(auth.basicUsername);
    auth.basicPassword = sub(auth.basicPassword);
  } else if (auth.type === 'bearer') {
    auth.bearerToken = sub(auth.bearerToken);
  } else if (auth.type === 'apikey') {
    auth.apiKeyName = sub(auth.apiKeyName);
    auth.apiKeyValue = sub(auth.apiKeyValue);
  }

  return { spec: resolved, missing: [...missing].sort() };
}
