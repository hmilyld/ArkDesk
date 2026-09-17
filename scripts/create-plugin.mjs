#!/usr/bin/env node
/**
 * 新建插件脚手架：交互式生成 `plugins/<id>/`（前端 + 后端），构建期自动注册。
 *
 * 用法：
 *   pnpm create-plugin                    # 交互式，回车保持默认值
 *   pnpm create-plugin -- --id my-tool --name "我的工具" --backend --no-settings ...
 *
 * 实现：以 `plugins/_template` 为唯一事实源复制后替换/裁剪；生成即为可用骨架。
 * 说明：前端由 Vite 扫描 `plugins/<id>/frontend/**`，后端由 src-tauri/build.rs 扫描
 *       `plugins/<id>/backend/mod.rs`，均无需任何手动注册。
 */
import { readFile, writeFile, readdir, mkdir, rm, stat } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, extname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import readline from 'node:readline/promises';
import { stdin as input, stdout as output } from 'node:process';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const PLUGINS_DIR = join(ROOT, 'plugins');
const TEMPLATE_DIR = join(PLUGINS_DIR, '_template');
const KEBAB_RE = /^[a-z][a-z0-9-]*$/;
const TEXT_EXTS = new Set(['.ts', '.vue', '.rs', '.md', '.json']);

// ── CLI flags ─────────────────────────────────────────────────
const argv = process.argv.slice(2);
const hasFlag = (name) => argv.includes(`--${name}`);
const flagValue = (name) => {
  const index = argv.indexOf(`--${name}`);
  return index >= 0 ? argv[index + 1] : undefined;
};
const interactive = !argv.some((arg) => arg.startsWith('--'));

function boolFlag(name, fallback) {
  if (hasFlag(name)) return true;
  if (hasFlag(`no-${name}`)) return false;
  return fallback;
}

function fail(message) {
  console.error(`✗ ${message}`);
  process.exit(1);
}

// ── 交互输入 ──────────────────────────────────────────────────
const rl = interactive ? readline.createInterface({ input, output }) : null;

const ask = async (question, fallback = '') => {
  const answer = (
    await rl.question(`${question}${fallback ? `（回车 = ${fallback}）` : ''}：`)
  ).trim();
  return answer || fallback;
};

const askBool = async (question, fallback) => {
  const hint = fallback ? 'Y/n' : 'y/N';
  const answer = (await rl.question(`${question} [${hint}]：`)).trim().toLowerCase();
  if (!answer) return fallback;
  return ['y', 'yes', '是'].includes(answer);
};

/** 循环询问直到校验通过（validator 返回错误信息或 null；可返回 Promise） */
async function promptValid(label, fallback, validator) {
  for (;;) {
    const value = (await rl.question(`${label}（回车 = ${fallback}）：`)).trim() || fallback;
    const error = await validator(value);
    if (!error) return value;
    console.log(`  ✗ ${error}`);
  }
}

// ── 图标校验（@lucide/vue），失败降级放行 ─────────────────────
let iconTable = null;
async function isValidIcon(name) {
  if (iconTable === null) {
    try {
      const mod = await import('@lucide/vue');
      iconTable = mod.icons ?? {};
    } catch (err) {
      console.log(`  ! 无法加载 @lucide/vue 校验图标（${err.message}），跳过校验`);
      iconTable = {};
    }
  }
  if (Object.keys(iconTable).length === 0) return true;
  return Object.prototype.hasOwnProperty.call(iconTable, name);
}

// ── 校验 ──────────────────────────────────────────────────────
function validateId(value) {
  if (!KEBAB_RE.test(value)) return '需以小写字母开头，仅含小写字母/数字/连字符';
  if (value.startsWith('_')) return '不能以 _ 开头（_ 目录不参与注册）';
  if (existsSync(join(PLUGINS_DIR, value))) return `目录已存在：plugins/${value}`;
  return null;
}

/** 遍历已有插件清单，检测工具 id 是否重复（仅提示） */
async function existingToolIds() {
  const ids = new Set();
  if (!existsSync(PLUGINS_DIR)) return ids;
  for (const dir of await readdir(PLUGINS_DIR)) {
    const manifestPath = join(PLUGINS_DIR, dir, 'plugin.json');
    if (!existsSync(manifestPath)) continue;
    try {
      const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
      for (const tool of manifest.tools ?? []) ids.add(tool.id);
    } catch {
      // 忽略无法解析的清单
    }
  }
  return ids;
}

// ── 收集配置 ──────────────────────────────────────────────────
async function collectConfig() {
  const cfg = {};

  cfg.id = flagValue('id');
  if (cfg.id === undefined) {
    cfg.id = await promptValid('插件 id（kebab-case，= 目录名，全局唯一）', 'my-tool', validateId);
  } else {
    const error = validateId(cfg.id);
    if (error) fail(error);
  }

  cfg.name = flagValue('name');
  if (cfg.name === undefined) {
    if (!interactive) fail('非交互模式需提供 --name');
    cfg.name = await ask('插件显示名', cfg.id);
  }

  cfg.description = flagValue('desc');
  if (cfg.description === undefined) {
    cfg.description = interactive ? await ask('插件描述（可选）', '') : '';
  }

  cfg.group = flagValue('group');
  if (cfg.group === undefined) cfg.group = interactive ? await ask('导航分组', '其他') : '其他';

  cfg.toolId = flagValue('tool-id');
  if (cfg.toolId === undefined) {
    if (interactive) {
      cfg.toolId = await promptValid('工具 id（kebab-case）', cfg.id, (v) =>
        KEBAB_RE.test(v) ? null : '需为 kebab-case'
      );
    } else {
      cfg.toolId = cfg.id;
    }
  } else if (!KEBAB_RE.test(cfg.toolId)) {
    fail(`非法工具 id：${cfg.toolId}`);
  }

  cfg.toolName = flagValue('tool-name');
  if (cfg.toolName === undefined) {
    cfg.toolName = interactive ? await ask('工具显示名', cfg.name) : cfg.name;
  }

  cfg.toolDesc = flagValue('tool-desc');
  if (cfg.toolDesc === undefined) {
    cfg.toolDesc = interactive ? await ask('工具描述（可选）', cfg.description) : cfg.description;
  }

  cfg.icon = flagValue('icon');
  if (cfg.icon === undefined) {
    cfg.icon = interactive
      ? await promptValid('导航图标（lucide 名）', 'Hammer', async (v) =>
          (await isValidIcon(v)) ? null : `未知 lucide 图标：${v}`
        )
      : 'Hammer';
  }
  if (!(await isValidIcon(cfg.icon))) {
    console.log(`  ! 未知图标 ${cfg.icon}，回退 Hammer`);
    cfg.icon = 'Hammer';
  }

  cfg.keepAlive = boolFlag(
    'keep-alive',
    interactive ? await askBool('切换工具时保留页面状态', true) : true
  );
  cfg.backend = boolFlag(
    'backend',
    interactive ? await askBool('生成后端 Rust 命令（backend/）', true) : true
  );
  cfg.settings = boolFlag(
    'settings',
    interactive ? await askBool('生成设置面板（frontend/settings/）', false) : false
  );
  cfg.schema = boolFlag(
    'schema',
    interactive ? await askBool('生成数据库表占位（schema.ts + migrations.rs）', false) : false
  );
  cfg.setup = boolFlag(
    'setup',
    interactive ? await askBool('生成生命周期钩子（frontend/setup.ts）', false) : false
  );

  if (cfg.schema && !cfg.backend) {
    console.log('  ! 数据库迁移需要后端，已自动启用 backend');
    cfg.backend = true;
  }

  return cfg;
}

// ── 文件操作 ──────────────────────────────────────────────────
async function copyDir(src, dest) {
  await mkdir(dest, { recursive: true });
  for (const entry of await readdir(src)) {
    const from = join(src, entry);
    const to = join(dest, entry);
    const info = await stat(from);
    if (info.isDirectory()) await copyDir(from, to);
    else await writeFile(to, await readFile(from));
  }
}

async function rewriteTree(dir, pairs) {
  for (const entry of await readdir(dir)) {
    const full = join(dir, entry);
    const info = await stat(full);
    if (info.isDirectory()) {
      await rewriteTree(full, pairs);
      continue;
    }
    if (!TEXT_EXTS.has(extname(entry))) continue;
    const source = await readFile(full, 'utf8');
    const next = pairs.reduce((text, [from, to]) => text.split(from).join(to), source);
    if (next !== source) await writeFile(full, next);
  }
}

function renderReadme(cfg) {
  const snake = cfg.id.replace(/-/g, '_');
  const lines = [
    `# ${cfg.name}`,
    '',
    cfg.description || '',
    '',
    `- 插件 id：\`${cfg.id}\``,
    `- 工具 id：\`${cfg.toolId}\`（路由 \`/tool/${cfg.toolId}\`）`,
  ];
  if (cfg.backend)
    lines.push(`- 后端命令前缀：\`${snake}_\`（\`backend/mod.rs\`，构建期自动登记）`);
  if (cfg.schema) lines.push(`- 数据库迁移 scope：\`${cfg.id}\``);
  lines.push(
    '',
    '## 开发',
    '',
    '- 界面：`frontend/views/Tool.vue`',
    '- 纯逻辑：`frontend/lib/`（无 Vue 依赖）；组合式函数：`frontend/composables/`（`useXxx.ts`）'
  );
  if (cfg.backend) lines.push('- 命令：`backend/mod.rs`（`#[tauri::command]`，构建期自动登记）');
  if (cfg.settings)
    lines.push('- 设置：`frontend/settings/Settings.vue`（`useToolSettings` 读写）');
  if (cfg.schema) lines.push('- 表定义：`frontend/schema.ts` + 迁移 `backend/migrations.rs`');
  lines.push('- 启动：`pnpm tauri dev`（前后端均构建期自动注册，无需手动登记）');
  lines.push(
    '',
    '> 目录规范见 [`plugins/README.md`](../README.md)；UI 规范见 [`docs/design.md`](../../docs/design.md)。',
    ''
  );
  return lines.join('\n');
}

async function generate(cfg) {
  const dest = join(PLUGINS_DIR, cfg.id);
  const snake = cfg.id.replace(/-/g, '_');
  await copyDir(TEMPLATE_DIR, dest);

  // 文本替换（长模式优先）：模板占位 → 实际值
  const pairs = [
    ['template_create_items', `${snake}_create_items`],
    ['template_items', `${snake}_items`],
    ['template_hello', `${snake}_hello`],
    ['本插件 id 为 template', `本插件 id 为 ${cfg.id}`],
    ['`template_`', '`' + snake + '_`'],
    ['"template"', `"${cfg.id}"`],
    ['模板插件', cfg.name],
    ['模板工具', cfg.toolName],
    ['Hammer', cfg.icon],
    ['新工具模板（复制本目录后按此清单改写）', cfg.description || cfg.name],
    ['一句话描述这个工具做什么', cfg.toolDesc || cfg.toolName],
    ['工具名称', cfg.toolName],
    ['一句话说明这个工具做什么', cfg.toolDesc || ''],
  ]
    .filter(([from, to]) => from !== to)
    .sort((a, b) => b[0].length - a[0].length);
  await rewriteTree(dest, pairs);

  // plugin.json：对象生成（避免正则裁剪出错）
  const manifestPath = join(dest, 'plugin.json');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  manifest.id = cfg.id;
  manifest.name = cfg.name;
  if (cfg.description) manifest.description = cfg.description;
  else delete manifest.description;
  if (cfg.group) manifest.group = cfg.group;
  else delete manifest.group;
  delete manifest.keywords;
  delete manifest.legacyMigrations;
  manifest.tools = [
    {
      id: cfg.toolId,
      name: cfg.toolName,
      ...(cfg.toolDesc ? { description: cfg.toolDesc } : {}),
      icon: cfg.icon,
      entry: 'frontend/views/Tool.vue',
      keepAlive: cfg.keepAlive,
    },
  ];
  if (cfg.settings) manifest.settings = { entry: 'frontend/settings/Settings.vue' };
  else delete manifest.settings;
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

  // README：按插件信息生成
  await writeFile(join(dest, 'README.md'), renderReadme(cfg));

  // 裁剪未选部分并同步引用
  if (!cfg.schema) {
    await rm(join(dest, 'frontend/schema.ts'), { force: true });
    await rm(join(dest, 'backend/migrations.rs'), { force: true });
    const modPath = join(dest, 'backend/mod.rs');
    if (existsSync(modPath)) {
      const modText = (await readFile(modPath, 'utf8')).replace(
        /\n*pub mod migrations;\n*/,
        '\n\n'
      );
      await writeFile(modPath, modText);
    }
  }
  if (!cfg.backend) await rm(join(dest, 'backend'), { recursive: true, force: true });
  if (!cfg.settings) await rm(join(dest, 'frontend/settings'), { recursive: true, force: true });
  if (!cfg.setup) await rm(join(dest, 'frontend/setup.ts'), { force: true });

  return dest;
}

// ── 主流程 ────────────────────────────────────────────────────
async function main() {
  if (!existsSync(TEMPLATE_DIR)) fail(`未找到插件模板：plugins/_template`);

  console.log(interactive ? '\n新建插件——交互式生成 plugins/<id>/（回车保持默认值）\n' : '');

  const cfg = await collectConfig();
  if (interactive) rl.close();

  const ids = await existingToolIds();
  if (ids.has(cfg.toolId)) {
    console.log(`  ! 警告：工具 id「${cfg.toolId}」已存在，重复注册时后者会被跳过（建议更换）`);
  }

  await generate(cfg);

  console.log(
    [
      '',
      `✅ 已生成插件：plugins/${cfg.id}`,
      '后续步骤：',
      '  1. pnpm format          # 统一格式（可选）',
      '  2. pnpm tauri dev       # 自动出现于侧栏，无需手动注册',
      `  3. 编辑 plugins/${cfg.id}/frontend/views/Tool.vue 实现界面`,
      '',
    ].join('\n')
  );
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
