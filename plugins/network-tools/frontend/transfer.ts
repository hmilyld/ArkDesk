/**
 * 自有 JSON 导入导出格式（单请求 / 集合 / 全部）。
 *
 * 纯逻辑，无 Tauri 依赖，可在 node 单测环境运行；数据库读写由调用方负责。
 */

import type { HttpRequestSpec } from './shared';

export const BUNDLE_TYPE = 'network-tools/bundle';
export const BUNDLE_VERSION = 1;

export interface BundleRequest {
  name: string;
  spec: HttpRequestSpec;
}

export interface BundleCollection {
  name: string;
  requests: BundleRequest[];
  children: BundleCollection[];
}

export interface BundleVar {
  key: string;
  value: string;
  enabled: number;
  isSecret: number;
}

export interface BundleEnvironment {
  name: string;
  vars: BundleVar[];
}

export interface Bundle {
  type: typeof BUNDLE_TYPE;
  version: number;
  requests?: BundleRequest[];
  collections?: BundleCollection[];
  environments?: BundleEnvironment[];
}

export function createBundle(partial: Omit<Bundle, 'type' | 'version'>): Bundle {
  return { type: BUNDLE_TYPE, version: BUNDLE_VERSION, ...partial };
}

/** 解析并校验导入文件；格式不符时抛错 */
export function parseBundle(text: string): Bundle {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    throw new Error('不是合法的 JSON 文件');
  }
  if (!data || typeof data !== 'object') throw new Error('文件内容不是对象');
  const bundle = data as Partial<Bundle>;
  if (bundle.type !== BUNDLE_TYPE) throw new Error('不是 network-tools 导出文件');
  return {
    type: BUNDLE_TYPE,
    version: bundle.version ?? BUNDLE_VERSION,
    requests: bundle.requests ?? [],
    collections: bundle.collections ?? [],
    environments: bundle.environments ?? [],
  };
}

/** 序列化为带缩进的 JSON 文本 */
export function stringifyBundle(bundle: Bundle): string {
  return `${JSON.stringify(bundle, null, 2)}\n`;
}
