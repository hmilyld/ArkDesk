#!/usr/bin/env node
/**
 * 生成应用在线更新清单与校验和。
 *
 * 用法（自动扫描构建产物）：
 *   pnpm release -- --base-url https://host/updates [--version 0.2.0] \
 *     [--notes notes.md | --changelog src/content/changelog.md]
 *
 * 手动指定平台映射（可多次）：
 *   pnpm release -- --base-url https://host/updates \
 *     --platform darwin-aarch64=path/PocketArk.app.tar.gz \
 *     --platform windows-x86_64=path/PocketArk_x.y.z_x64-setup.nsis.zip
 *
 * 产物：
 *   release/latest.json   # 更新服务端清单（version/notes/pub_date/platforms）
 *   release/checksums.txt # 各文件的 SHA-256
 *
 * 说明：签名文件需与更新包同目录同名且以 `.sig` 结尾（tauri build 生成）；
 *       未提供 --notes 时使用空说明。版本缺省取自 tauri.conf.json。
 */
import { createHash } from 'node:crypto';
import { readFile, writeFile, readdir, mkdir, stat } from 'node:fs/promises';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const TARGET_DIR = join(ROOT, 'src-tauri', 'target');
const OUT_DIR = join(ROOT, 'release');

const argv = process.argv.slice(2);
const flag = (name) => {
  const index = argv.indexOf(`--${name}`);
  return index >= 0 ? argv[index + 1] : undefined;
};
const flagAll = (name) => {
  const values = [];
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === `--${name}` && argv[i + 1]) values.push(argv[i + 1]);
  }
  return values;
};

function fail(message) {
  console.error(`✗ ${message}`);
  process.exit(1);
}

/** 依据文件名推断 updater 平台 key */
function inferPlatform(file) {
  if (/\.app\.tar\.gz$/.test(file)) {
    // macOS：无架构信息时按 Apple Silicon 处理（Intel 请用 --platform 显式指定）
    if (/x86_64|intel|x64/i.test(file)) return 'darwin-x86_64';
    return 'darwin-aarch64';
  }
  if (/setup\.exe$/i.test(file)) return 'windows-x86_64'; // Tauri v2：NSIS 安装器即更新包
  if (/\.nsis\.zip$/.test(file)) return 'windows-x86_64'; // 兼容旧格式
  if (/\.msi\.zip$/.test(file)) return 'windows-x86_64';
  if (/\.AppImage\.tar\.gz$/.test(file)) return 'linux-x86_64';
  return null;
}

async function sha256(path) {
  const data = await readFile(path);
  return createHash('sha256').update(data).digest('hex');
}

async function readSignature(artifactPath) {
  const sigPath = `${artifactPath}.sig`;
  try {
    return (await readFile(sigPath, 'utf8')).trim();
  } catch {
    return null;
  }
}

async function collectFromArgs() {
  const mappings = flagAll('platform');
  if (mappings.length === 0) return null;
  const result = [];
  for (const mapping of mappings) {
    const eq = mapping.indexOf('=');
    if (eq < 0) fail(`--platform 需为 key=path 形式：${mapping}`);
    const key = mapping.slice(0, eq);
    const path = resolve(mapping.slice(eq + 1));
    if (!(await stat(path).catch(() => null))) fail(`文件不存在：${path}`);
    result.push({ key, path });
  }
  return result;
}

async function scanBundleDir(dir) {
  const found = [];
  const walk = async (current) => {
    let entries;
    try {
      entries = await readdir(current, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      const full = join(current, entry.name);
      if (entry.isDirectory()) {
        await walk(full);
        continue;
      }
      if (!entry.name.endsWith('.sig')) continue;
      const artifact = full.slice(0, -'.sig'.length);
      const key = inferPlatform(basename(artifact));
      if (!key) continue;
      if (!(await stat(artifact).catch(() => null))) continue;
      found.push({ key, path: artifact });
    }
  };
  await walk(dir);
  return found;
}

/** 候选 bundle 目录：`target/release/bundle` 与 `target/<triple>/release/bundle`（--target 构建） */
async function bundleDirs() {
  const dirs = [];
  const direct = join(TARGET_DIR, 'release', 'bundle');
  if (await stat(direct).catch(() => null)) dirs.push(direct);
  let entries = [];
  try {
    entries = await readdir(TARGET_DIR, { withFileTypes: true });
  } catch {
    entries = [];
  }
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const nested = join(TARGET_DIR, entry.name, 'release', 'bundle');
    if (await stat(nested).catch(() => null)) dirs.push(nested);
  }
  return dirs;
}

/** 从 Keep a Changelog 文档中提取指定版本段落（标题形如 `## [x.y.z]` / `## x.y.z`） */
async function extractChangelog(file, version) {
  const path = resolve(file);
  if (!(await stat(path).catch(() => null))) fail(`changelog 文件不存在：${path}`);
  const lines = (await readFile(path, 'utf8')).split(/\r?\n/);
  const escaped = version.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const heading = new RegExp(`^##\\s+\\[?v?${escaped}\\]?`);
  const start = lines.findIndex((line) => heading.test(line));
  if (start < 0) fail(`changelog 中未找到版本 ${version} 的段落（## [${version}]）`);
  let end = lines.length;
  for (let i = start + 1; i < lines.length; i += 1) {
    if (/^##\s+/.test(lines[i])) {
      end = i;
      break;
    }
  }
  return lines.slice(start, end).join('\n').trim();
}

async function main() {
  const baseUrl = flag('base-url');
  if (!baseUrl) fail('缺少 --base-url（更新服务器基础地址）');

  let version = flag('version');
  if (!version) {
    const conf = JSON.parse(await readFile(join(ROOT, 'src-tauri', 'tauri.conf.json'), 'utf8'));
    version = conf.version;
  }
  if (!/^\d+\.\d+\.\d+/.test(version)) fail(`非法版本号：${version}`);

  let notes = '';
  const notesArg = flag('notes');
  const changelogArg = flag('changelog');
  if (notesArg) {
    // 视为文件路径（存在则读取），否则按字面文本
    const maybeFile = resolve(notesArg);
    notes = (await stat(maybeFile).catch(() => null))
      ? await readFile(maybeFile, 'utf8')
      : notesArg;
  } else if (changelogArg) {
    notes = await extractChangelog(changelogArg, version);
  }

  let artifacts = await collectFromArgs();
  if (!artifacts) {
    artifacts = [];
    for (const dir of await bundleDirs()) {
      artifacts.push(...(await scanBundleDir(dir)));
    }
  }
  if (artifacts.length === 0) {
    fail(`未找到更新产物（.sig）。请先执行带签名的 tauri build，或用 --platform key=path 指定`);
  }

  const platforms = {};
  const checksums = [];
  const normalizedBase = baseUrl.replace(/\/$/, '');

  for (const { key, path } of artifacts) {
    const signature = await readSignature(path);
    if (!signature) {
      console.warn(`  ! 缺少签名（${path}.sig），跳过 ${key}`);
      continue;
    }
    const url = `${normalizedBase}/${basename(path)}`;
    platforms[key] = { signature, url };
    checksums.push(`${await sha256(path)}  ${basename(path)}`);
  }

  if (Object.keys(platforms).length === 0) fail('没有任何带签名的更新产物');

  const manifest = {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms,
  };

  await mkdir(OUT_DIR, { recursive: true });
  await writeFile(join(OUT_DIR, 'latest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  await writeFile(join(OUT_DIR, 'checksums.txt'), `${checksums.join('\n')}\n`);

  console.log(
    `\n✅ 已生成 release/latest.json（版本 ${version}，平台 ${Object.keys(platforms).join(', ')}）`
  );
  console.log('   release/checksums.txt');
  console.log('后续：把各更新包 + latest.json 上传到更新服务器地址。\n');
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
