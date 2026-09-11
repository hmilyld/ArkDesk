#!/usr/bin/env node
/**
 * 本地层准备（fork 专属）：下载个人工具所需资源到 src-tauri/local-resources/。
 * base 不含本文件；CI 环境默认跳过下载。
 *
 * 由 scripts/prepare.mjs 在 dev/build 前调用。
 */
import { spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

if (process.env.CI) {
  console.log('[local:prepare] CI 环境跳过资源下载');
  process.exit(0);
}

for (const script of ['fetch-fonts.mjs', 'fetch-ocr-models.mjs']) {
  const result = spawnSync(process.execPath, [join(ROOT, 'scripts', 'local', script)], {
    stdio: 'inherit',
    cwd: ROOT,
  });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
