#!/usr/bin/env node
/**
 * 按需下载内置 OCR 模型（PaddleOCR PP-OCRv6 small，MNN）到
 * src-tauri/local-resources/ocr-models/。（本地层 fork-owned；base 不含本文件。）
 *
 * 模型约 15MB，不纳入 git（.mnn 已忽略；charset 文本极小仍入库）：
 * 下载并校验 SHA-256，已存在且通过则跳过。来源为 ocr-rs 上游仓库
 * rust-paddle-ocr 的 models/（tag 固定）。GitHub 直连失败时经代理
 * （见 scripts/local/lib/fetch.mjs），可用 OCR_MODEL_SOURCE_BASE 指定镜像。
 *
 * 用法：pnpm ocr-models
 */
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ensureFile } from './lib/fetch.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const MODEL_DIR = join(ROOT, 'src-tauri', 'local-resources', 'ocr-models');
const BASE = (
  process.env.OCR_MODEL_SOURCE_BASE ||
  'https://raw.githubusercontent.com/zibo-chen/rust-paddle-ocr/v2.4.1/models'
).replace(/\/+$/, '');

const MODELS = [
  {
    file: 'PP-OCRv6_small_det.mnn',
    sha256: '2c6277abbbddb4c77a790f4650cdd7f8ab33512db38fac372ed5471538070619',
  },
  {
    file: 'PP-OCRv6_small_rec.mnn',
    sha256: 'ed59cc294fe2d564bd64f929b5356b70abd0977f99e7e60e3b08cfeef4ef72be',
  },
  {
    file: 'ppocr_keys_v6_small.txt',
    sha256: 'b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d',
  },
];

let ok = true;
for (const model of MODELS) {
  const done = await ensureFile({
    url: `${BASE}/${model.file}`,
    dest: join(MODEL_DIR, model.file),
    sha256: model.sha256,
    label: model.file,
  });
  ok = ok && done;
}

if (!ok) {
  console.error('OCR 模型未就绪：可设置 GITHUB_PROXY / OCR_MODEL_SOURCE_BASE 后重试');
  process.exit(1);
}
console.log('OCR 模型就绪。');
