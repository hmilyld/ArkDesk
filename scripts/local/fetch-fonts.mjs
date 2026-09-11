#!/usr/bin/env node
/**
 * 按需下载内置中文字体（Noto Sans CJK SC）到 src-tauri/local-resources/fonts/。
 * （本地层 fork-owned；base 不含本文件。）
 *
 * 字体约 32MB，不纳入 git；下载并校验 SHA-256，已存在且通过则跳过。
 * GitHub 直连失败时经代理（见 scripts/local/lib/fetch.mjs），可用
 * FONT_SOURCE_BASE 指定镜像。
 *
 * 用法：pnpm fonts
 */
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ensureFile } from './lib/fetch.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const FONT_DIR = join(ROOT, 'src-tauri', 'local-resources', 'fonts');
const BASE = (
  process.env.FONT_SOURCE_BASE || 'https://raw.githubusercontent.com/notofonts/noto-cjk/Sans2.004'
).replace(/\/+$/, '');

const FONTS = [
  {
    file: 'NotoSansCJKsc-Regular.otf',
    sha256: '2c76254f6fc379fddfce0a7e84fb5385bb135d3e399294f6eeb6680d0365b74b',
  },
  {
    file: 'NotoSansCJKsc-Bold.otf',
    sha256: 'b5f0d1a190a7f9b43c310a8850630af12553df32c4c050543f9059732d9b4c0a',
  },
];

let ok = true;
for (const font of FONTS) {
  const done = await ensureFile({
    url: `${BASE}/Sans/OTF/SimplifiedChinese/${font.file}`,
    dest: join(FONT_DIR, font.file),
    sha256: font.sha256,
    label: font.file,
  });
  ok = ok && done;
}

if (!ok) {
  console.error('字体未就绪：可设置 GITHUB_PROXY / FONT_SOURCE_BASE 后重试');
  process.exit(1);
}
console.log('字体就绪。');
