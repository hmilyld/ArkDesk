#!/usr/bin/env node
/**
 * 设计规范 lint（见 DESIGN.md §6 反例清单）。
 *
 * 默认 **告警模式**：打印违规统计与文件清单，退出码 0（迁移期不阻塞开发）。
 * `--strict` 时退出码 1，供迁移完成后接入 CI 硬门禁。
 *
 * 规则（可机器判定部分；其余靠 DESIGN.md §7 评审清单）：
 *   R1 旧卡片配方：`border-border bg-card` / `border border-border bg-card`
 *   R2 任意透明度表面：`bg-muted/NN`、`bg-card/NN`、`bg-background/NN`、`bg-sidebar/NN`
 *   R3 非浮层使用 `rounded-xl`（仅 ui/ 浮层组件与 native/ 可用）
 *   R4 写死控件尺寸的任意值：`h-[NNpx]`、`w-[NNpx]`（不含 max-h/min-h 等内容视口尺寸）
 *   R5 全局动效压制回退：`transition-duration: 75ms`
 *   R6 焦点环未按规范：非 3px 或非 ring/60 的 focus-visible ring
 *
 * 豁免：行内 `design-lint-ignore` 注释（说明理由）；`src/components/ui/**` 由 shadcn CLI 管理，不参与 R1/R2/R3。
 */
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(fileURLToPath(import.meta.url), '..', '..');
const STRICT = process.argv.includes('--strict');

const SCAN_DIRS = ['src', 'plugins'];
const EXTS = new Set(['.vue', '.ts', '.css']);
const UI_PREFIX = `src${sep}components${sep}ui${sep}`;

const RULES = [
  {
    id: 'R1',
    name: '旧卡片配方',
    test: (text) => /border-border\s+bg-card|border\s+border-border\s+bg-card/.test(text),
    hint: '统一 `rounded-lg border bg-card`（DESIGN.md §2.2/§2.6）',
    skipUi: true,
  },
  {
    id: 'R2',
    name: '任意透明度的表面色',
    test: (text) => /\bbg-(muted|card|background|sidebar)\/\d+/.test(text),
    hint: '走层次枚举 bg-muted / bg-sunken / bg-accent（DESIGN.md §2.5）',
    skipUi: true,
  },
  {
    id: 'R3',
    name: '非浮层使用 rounded-xl',
    test: (text) => /\brounded-xl\b/.test(text),
    hint: 'rounded-md=控件 / rounded-lg=容器 / rounded-xl=浮层（DESIGN.md §2.2）',
    skipUi: true,
  },
  {
    id: 'R4',
    name: '写死 px 尺寸',
    // 只匹配控件尺寸；`max-h-[…]` / `min-h-[…]` 是内容视口尺寸，属允许项
    test: (text) => /(?<![a-z-])(?:h|w)-\[\d+px\]/.test(text),
    hint: '控件高度用 rem 档（h-7/h-8/h-9）随三档字号缩放（DESIGN.md §2.4）',
    skipUi: true,
  },
  {
    id: 'R6',
    name: '焦点环未按规范',
    // 规范：focus-visible:border-ring + ring-3 + ring-ring/60（DESIGN.md §2.9）
    test: (text) =>
      /focus-visible:ring-(1|2|4)\b|focus-visible:ring-\[|focus-visible:ring-ring\/(?!60)\d+/.test(
        text
      ),
    hint: '统一 focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60',
    skipUi: false,
  },
  {
    id: 'R5',
    name: '全局动效压制',
    test: (text) => /transition-duration:\s*75ms/.test(text),
    hint: '使用动效 token（DESIGN.md §2.7）',
    skipUi: false,
  },
];

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    if (name === 'node_modules' || name === 'dist' || name.startsWith('.')) continue;
    const path = join(dir, name);
    const stat = statSync(path);
    if (stat.isDirectory()) walk(path, out);
    else if (EXTS.has(path.slice(path.lastIndexOf('.')))) out.push(path);
  }
  return out;
}

const files = SCAN_DIRS.flatMap((d) => walk(join(ROOT, d)));
const findings = [];

for (const file of files) {
  const rel = relative(ROOT, file);
  const text = readFileSync(file, 'utf8');
  if (text.includes('design-lint-ignore')) continue;
  const isUi = rel.startsWith(UI_PREFIX);
  for (const rule of RULES) {
    if (rule.skipUi && isUi) continue;
    if (!rule.test(text)) continue;
    const lines = text.split('\n');
    lines.forEach((line, index) => {
      // 跳过注释行；带 backdrop-blur 的行属材质区域，半透明是规范内用法
      const isComment = /^\s*(\*|\/\*|#|--)|\/\*/.test(line) && !/class=|:class=/.test(line);
      if (isComment || /backdrop-blur/.test(line)) return;
      if (rule.test(line)) {
        findings.push({ rule, file: rel, line: index + 1, text: line.trim().slice(0, 100) });
      }
    });
  }
}

const byRule = new Map();
for (const f of findings) {
  if (!byRule.has(f.rule.id)) byRule.set(f.rule.id, { rule: f.rule, items: [] });
  byRule.get(f.rule.id).items.push(f);
}

const total = findings.length;
if (total === 0) {
  console.log('[lint-design] 未发现违规 ✅');
} else {
  console.log(
    `[lint-design] 发现 ${total} 处待收敛（${STRICT ? '严格模式' : '告警模式'}，见 DESIGN.md §6）`
  );
  for (const { rule, items } of [...byRule.values()].sort(
    (a, b) => b.items.length - a.items.length
  )) {
    console.log(`\n  ${rule.id} ${rule.name} · ${items.length} 处 — ${rule.hint}`);
    const preview = items.slice(0, 8);
    for (const item of preview) console.log(`    ${item.file}:${item.line}`);
    if (items.length > preview.length) console.log(`    …另有 ${items.length - preview.length} 处`);
  }
  console.log('\n  豁免：行内加 `design-lint-ignore` 注释并说明理由。');
}

process.exit(STRICT && total > 0 ? 1 : 0);
