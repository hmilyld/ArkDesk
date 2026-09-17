#!/usr/bin/env node
/**
 * 插件目录结构 lint（见 `plugins/README.md` 的「目录规范」）。
 *
 * 与视觉规范的 `lint-design.mjs` 分开：本脚本只管**结构与命名**。
 * 严格模式：发现违规即退出码 1（`pnpm lint` 已接入）。
 * 豁免：行内或文件内写 `lint-plugins-ignore` 注释。
 *
 *   R-P1 frontend/ 顶层只允许白名单条目
 *   R-P2 plugin.json / README.md / AGENTS.md 必需（`_`、`.` 前缀目录跳过）
 *   R-P3 plugin.json 的 entry 形状（tools → frontend/views/*.vue，settings → frontend/settings/*.vue）
 *   R-P4 #[tauri::command] 只能出现在 backend/mod.rs
 *   R-P5 命名：views|settings 下 PascalCase.vue；composables 下 use*.ts；
 *        lib/**.ts 与 backend/**.rs（除 mod.rs / migrations.rs）为 kebab-case / snake_case
 *   R-P6 模板一致性：`plugins/_template/` 同样按上述规则校验（脚手架产物即规范样本）
 */
import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(fileURLToPath(import.meta.url), '..', '..');
const PLUGINS = join(ROOT, 'plugins');

const FRONTEND_WHITELIST = new Set([
  'views',
  'settings',
  'components',
  'composables',
  'lib',
  'shared.ts',
  'schema.ts',
  'setup.ts',
]);

const PASCAL = /^[A-Z][A-Za-z0-9]*\.vue$/;
const USE_HOOK = /^use[A-Z][A-Za-z0-9]*\.ts$/;
const KEBAB = /^[a-z0-9]+(-[a-z0-9]+)*\.ts$/;
const SNAKE = /^[a-z0-9]+(_[a-z0-9]+)*\.rs$/;
const SNAKE_DIR = /^[a-z0-9]+(_[a-z0-9]+)*$/;
const MOD_RS = new Set(['mod.rs', 'migrations.rs']);

const findings = [];
/** sourcePath：用于 `lint-plugins-ignore` 豁免（目录会被忽略读取失败，视为不豁免） */
function report(rule, file, message, sourcePath) {
  if (sourcePath && ignored(sourcePath)) return;
  findings.push({ rule, file, message });
}

function ignored(file) {
  try {
    return readFileSync(file, 'utf8').includes('lint-plugins-ignore');
  } catch {
    return false;
  }
}

function entries(dir) {
  return existsSync(dir) ? readdirSync(dir).sort() : [];
}

function* walk(dir) {
  for (const name of entries(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) yield* walk(path);
    else yield path;
  }
}

for (const plugin of entries(PLUGINS)) {
  const pluginDir = join(PLUGINS, plugin);
  if (!statSync(pluginDir).isDirectory()) continue;

  const isTemplate = plugin.startsWith('_');
  const rel = (path) => relative(ROOT, path);

  // R-P2：必需文件（模板目录豁免 AGENTS/README 之外的注册文件）
  if (!isTemplate) {
    for (const required of ['plugin.json', 'README.md', 'AGENTS.md']) {
      if (!existsSync(join(pluginDir, required))) {
        report('R-P2', `${rel(pluginDir)}/`, `缺少必需文件 ${required}`);
      }
    }
  }

  const frontendDir = join(pluginDir, 'frontend');

  // R-P1：frontend 顶层白名单
  for (const name of entries(frontendDir)) {
    if (!FRONTEND_WHITELIST.has(name)) {
      report(
        'R-P1',
        rel(join(frontendDir, name)),
        'frontend/ 顶层不在白名单内（见 plugins/README.md 目录规范）',
        join(frontendDir, name)
      );
    }
  }

  // R-P5：命名
  for (const path of walk(frontendDir)) {
    const name = path.slice(path.lastIndexOf(sep) + 1);
    const parent = path.slice(0, path.lastIndexOf(sep));
    if (parent.endsWith(`${sep}views`) || parent.endsWith(`${sep}settings`)) {
      if (name.endsWith('.vue') && !PASCAL.test(name)) {
        report('R-P5', rel(path), 'views/settings 下的组件应为 PascalCase.vue', path);
      }
    }
    if (parent.endsWith(`${sep}composables`)) {
      if (name.endsWith('.ts') && !USE_HOOK.test(name)) {
        report('R-P5', rel(path), 'composables 下的文件应为 useXxx.ts', path);
      }
    }
    if (path.includes(`${sep}lib${sep}`) || path.includes(`${sep}lib`) === false) {
      // lib 下（含子目录）的 TS 用 kebab-case
      if (parent.includes(`${sep}lib`) && name.endsWith('.ts') && !KEBAB.test(name)) {
        report('R-P5', rel(path), 'lib 下的文件应为 kebab-case.ts', path);
      }
    }
  }

  // R-P3：plugin.json entry 形状
  const manifestPath = join(pluginDir, 'plugin.json');
  if (existsSync(manifestPath)) {
    try {
      const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
      for (const tool of manifest.tools ?? []) {
        if (
          typeof tool.entry === 'string' &&
          !/^frontend\/views\/[A-Za-z0-9]+\.vue$/.test(tool.entry)
        ) {
          report(
            'R-P3',
            rel(manifestPath),
            `tools[].entry 应为 frontend/views/<PascalCase>.vue，收到 ${tool.entry}`,
            manifestPath
          );
        }
        if (tool.entry && !existsSync(join(pluginDir, tool.entry))) {
          report('R-P3', rel(manifestPath), `tools[].entry 指向的文件不存在：${tool.entry}`);
        }
      }
      const settingsEntry = manifest.settings?.entry;
      if (
        typeof settingsEntry === 'string' &&
        !/^frontend\/settings\/[A-Za-z0-9]+\.vue$/.test(settingsEntry)
      ) {
        report(
          'R-P3',
          rel(manifestPath),
          `settings.entry 应为 frontend/settings/<PascalCase>.vue，收到 ${settingsEntry}`
        );
      }
    } catch (error) {
      report('R-P3', rel(manifestPath), `plugin.json 解析失败：${String(error)}`);
    }
  }

  // R-P4 / R-P5：后端
  const backendDir = join(pluginDir, 'backend');
  for (const path of walk(backendDir)) {
    const name = path.slice(path.lastIndexOf(sep) + 1);
    if (name.endsWith('.rs') && !MOD_RS.has(name) && !SNAKE.test(name)) {
      report('R-P5', rel(path), 'backend 下的 .rs 应为 snake_case.rs', path);
    }
    if (name !== 'mod.rs' && readFileSync(path, 'utf8').includes('#[tauri::command]')) {
      report('R-P4', rel(path), '#[tauri::command] 只能写在 backend/mod.rs', path);
    }
  }
  for (const name of entries(backendDir)) {
    const path = join(backendDir, name);
    if (statSync(path).isDirectory() && !SNAKE_DIR.test(name)) {
      report('R-P5', rel(path), 'backend 功能域目录应为 snake_case');
    }
  }
}

const summary = new Map();
for (const finding of findings) {
  if (!summary.has(finding.rule)) summary.set(finding.rule, []);
  summary.get(finding.rule).push(finding);
}

if (findings.length === 0) {
  console.log('[lint-plugins] 未发现违规 ✅');
} else {
  console.log(`[lint-plugins] 发现 ${findings.length} 处结构违规（规范见 plugins/README.md）`);
  for (const [rule, items] of [...summary.entries()].sort()) {
    console.log(`\n  ${rule} · ${items.length} 处`);
    for (const item of items.slice(0, 10)) console.log(`    ${item.file} — ${item.message}`);
    if (items.length > 10) console.log(`    …另有 ${items.length - 10} 处`);
  }
  console.log('\n  豁免：在文件内写 `lint-plugins-ignore` 注释并说明理由。');
}

process.exit(findings.length > 0 ? 1 : 0);
