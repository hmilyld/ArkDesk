#!/usr/bin/env node
/**
 * 文档一致性 lint（见 `docs/README.md` 的「维护约定」）。
 *
 *   R-D1 `docs/README.md` 索引必须列出 docs/ 下每份文档（README.md 自身除外）
 *   R-D2 全仓 Markdown 相对链接必须可解析（跳过 http(s)/mailto/锚点）
 *   R-D3 不允许残留旧根级文档名（DESIGN 系列 / START / RELEASE / LOCAL / MIGRATION 等 .md）
 *
 * 严格模式：发现违规即退出码 1（`pnpm lint` 已接入）。
 */
import { readdirSync, readFileSync, existsSync, statSync } from 'node:fs';
import { join, dirname, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(fileURLToPath(import.meta.url), '..', '..');
const DOCS = join(ROOT, 'docs');
const SKIP_DIRS = new Set(['node_modules', 'dist', '.git', '.opencode', 'target']);
// 已迁移的旧根级文档名：除 docs/ 下的新名外，不应再作为路径出现
const STALE_NAMES = [
  'DESIGN.md',
  'DESIGN-macos.md',
  'DESIGN-windows.md',
  'DESIGN-appendix.md',
  'START.md',
  'RELEASE.md',
  'LOCAL.md',
  'MIGRATION.md',
];
const STALE_RE = new RegExp(`\\b(${STALE_NAMES.map(escapeRe).join('|')})\\b`);

const findings = [];
function report(rule, file, message) {
  findings.push({ rule, file, message });
}

function escapeRe(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function walk(dir, onFile) {
  for (const name of readdirSync(dir)) {
    if (SKIP_DIRS.has(name)) continue;
    const full = join(dir, name);
    const info = statSync(full);
    if (info.isDirectory()) walk(full, onFile);
    else onFile(full);
  }
}

const rel = (file) => relative(ROOT, file).split(sep).join('/');

/** R-D1：docs/ 下每份文档都要在索引里出现 */
function checkIndex() {
  const index = join(DOCS, 'README.md');
  if (!existsSync(index)) {
    report('R-D1', 'docs/README.md', '缺少文档索引文件');
    return;
  }
  const text = readFileSync(index, 'utf8');
  for (const name of readdirSync(DOCS).sort()) {
    if (name === 'README.md') continue;
    const full = join(DOCS, name);
    if (!statSync(full).isFile()) continue;
    if (!text.includes(`(${name})`)) {
      report('R-D1', 'docs/README.md', `未在索引中登记：docs/${name}`);
    }
  }
}

// R-D2 / R-D3：遍历全仓文本文件
const LINK_RE = /\]\(([^)\s]+)\)/g;
function checkFiles() {
  walk(ROOT, (file) => {
    if (!/\.(md|mjs|ts|vue|rs|yml|json)$/.test(file)) return;
    const text = readFileSync(file, 'utf8');

    // R-D3 旧路径残留（docs/ 下已是新名，不会再命中）
    if (!file.startsWith(DOCS + sep) && !rel(file).startsWith('scripts/check-docs.mjs')) {
      const hit = text.match(STALE_RE);
      if (hit) report('R-D3', rel(file), `残留旧文档名：${hit[1]}`);
    }

    // R-D2 相对链接可解析
    if (!file.endsWith('.md')) return;
    for (const m of text.matchAll(LINK_RE)) {
      const raw = m[1];
      if (/^(https?:|mailto:|#)/.test(raw)) continue;
      const target = raw.split('#')[0];
      if (!target) continue;
      const resolved = join(dirname(file), target);
      if (!existsSync(resolved)) {
        report('R-D2', rel(file), `链接目标不存在：${raw}`);
      }
    }
  });
}

checkIndex();
checkFiles();

if (findings.length === 0) {
  console.log('[check-docs] 未发现违规 ✅');
  process.exit(0);
}
const byRule = new Map();
for (const f of findings) byRule.set(f.rule, [...(byRule.get(f.rule) ?? []), f]);
console.log(`[check-docs] 发现 ${findings.length} 处违规（规范见 docs/README.md）\n`);
for (const [rule, list] of [...byRule].sort()) {
  console.log(`  ${rule} · ${list.length} 处`);
  for (const f of list) console.log(`    ${f.file} — ${f.message}`);
  console.log('');
}
process.exit(1);
