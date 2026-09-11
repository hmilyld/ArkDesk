#!/usr/bin/env node
/**
 * 开发/构建前置脚本：生成命令名 + 可选的本地层准备。
 *
 * - 命令名：scripts/gen-commands.mjs（框架 + 插件命令名联合类型）
 * - 本地层准备：scripts/local/prepare.mjs（存在时执行，用于下载字体/OCR 等；base 无此文件）
 *
 * 由 package.json 的 dev / build 脚本调用：`node scripts/prepare.mjs <dev|build>`。
 */
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const mode = process.argv[2] ?? 'dev';

function run(script) {
  const result = spawnSync(process.execPath, [script, mode], { stdio: 'inherit', cwd: ROOT });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

run(join(ROOT, 'scripts', 'gen-commands.mjs'));

const localPrepare = join(ROOT, 'scripts', 'local', 'prepare.mjs');
if (existsSync(localPrepare)) {
  run(localPrepare);
}
