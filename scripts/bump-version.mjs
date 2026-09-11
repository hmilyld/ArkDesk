#!/usr/bin/env node
/**
 * 版本号一键同步：以 src-tauri/tauri.conf.json 为唯一事实源，同步写入
 * src-tauri/Cargo.toml 与 package.json，避免三处漂移导致更新逻辑异常。
 *
 * 用法：pnpm version:bump <x.y.z>    （也接受 v x.y.z 前缀）
 *
 * 注意：更新选择采用严格 semver 比较（远端 > 本地才提示），版本号必须单调递增。
 */
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const raw = process.argv.slice(2).find((arg) => !arg.startsWith('-'));
if (!raw) {
  console.error('用法：pnpm version:bump <x.y.z>');
  process.exit(1);
}

const version = raw.replace(/^v/, '');
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z-.]+)?$/.test(version)) {
  console.error(`非法版本号：${raw}（需为 semver，如 0.2.0）`);
  process.exit(1);
}

const CONF = join(ROOT, 'src-tauri/tauri.conf.json');
const CARGO = join(ROOT, 'src-tauri/Cargo.toml');
const PKG = join(ROOT, 'package.json');

// 先读取并构造全部新内容，任一校验失败都不会写盘（避免半写损坏文件）。
// 用正则替换而非 JSON 解析重排，以保留 prettier 的原始排版、避免无关 diff。
async function buildJsonOutput(path) {
  const text = await readFile(path, 'utf8');
  const pattern = /^  "version": "[^"]+"/m;
  if (!pattern.test(text)) {
    throw new Error(`${path} 未找到顶层 version 字段`);
  }
  return text.replace(pattern, `  "version": "${version}"`);
}

async function buildCargoOutput() {
  const text = await readFile(CARGO, 'utf8');
  const pattern = /^version = "[^"]+"/m;
  if (!pattern.test(text)) {
    throw new Error('Cargo.toml 未找到 [package] version 字段');
  }
  return text.replace(pattern, `version = "${version}"`);
}

const [confOut, pkgOut, cargoOut] = await Promise.all([
  buildJsonOutput(CONF),
  buildJsonOutput(PKG),
  buildCargoOutput(),
]);

// 校验全部通过后再依次写盘
await writeFile(CONF, confOut, 'utf8');
await writeFile(PKG, pkgOut, 'utf8');
await writeFile(CARGO, cargoOut, 'utf8');

console.log(`版本已同步为 ${version}：tauri.conf.json / Cargo.toml / package.json`);
