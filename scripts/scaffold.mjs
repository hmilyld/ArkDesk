#!/usr/bin/env node
/**
 * PocketArk 脚手架：clone 后一键把项目改名为你自己的应用。
 *
 * 用法：pnpm scaffold（交互式问答，回车保持默认值）
 * 覆盖：tauri.conf（productName/identifier/title）、package.json、Cargo.toml（包名/lib 名）、
 *       main.rs 调用、前端与文档中的应用名、默认主题色 / 默认主题。
 * 不覆盖：图标（改 src-tauri/icons/icon.svg 后跑 pnpm tauri icon）、
 *         示例插件删除（见 docs/start.md——Rust 模块绑定迁移历史，建议保留）。
 * 完成后：pnpm install（刷新 lockfile）→ pnpm tauri dev。
 */
import { readFile, writeFile, readdir, lstat } from 'node:fs/promises';
import { dirname, extname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import readline from 'node:readline/promises';
import { stdin as input, stdout as output } from 'node:process';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');

/** 不参与改写的目录（依赖/产物/二进制资源） */
const SKIP_DIRS = new Set([
  '.git',
  'node_modules',
  'dist',
  'target',
  'gen',
  'icons',
  'scripts',
  '.vscode',
]);
/** 不参与改写的文件（lockfile 由包管理器维护，二进制资源非文本） */
const SKIP_FILES = new Set(['pnpm-lock.yaml', 'Cargo.lock', 'tauri-dev-watcher.gitignore']);
/** 视为文本的扩展名 */
const TEXT_EXTS = new Set(['.ts', '.vue', '.rs', '.json', '.md', '.toml', '.html', '.css', '.sh']);

const ACCENTS = ['indigo', 'teal', 'amber', 'rose', 'blue', 'grid-green'];
const KEBAB_RE = /^[a-z][a-z0-9-]*$/;

const rl = readline.createInterface({ input, output });
const ask = async (question, fallback) => {
  const answer = (await rl.question(`${question}（回车 = ${fallback}）：`)).trim();
  return answer || fallback;
};

/** 非交互模式：全部经命令行 flag 提供（--name/--kebab/--id/--accent/--theme），供脚本化调用 */
const argv = process.argv.slice(2);
const flag = (name) => {
  const index = argv.indexOf(`--${name}`);
  return index >= 0 ? argv[index + 1] : undefined;
};
const interactive = !argv.some((arg) => arg.startsWith('--'));

async function main() {
  console.log(
    interactive ? '\nPocketArk 脚手架——把项目改名为你自己的应用（回车保持默认值）\n' : ''
  );

  // ── 收集配置 ──────────────────────────────────────────────
  let displayName = flag('name');
  if (displayName === undefined) {
    do {
      displayName = await ask('软件显示名（中文/英文均可）', 'PocketArk');
    } while (!displayName.trim());
    displayName = displayName.trim();
  }

  let kebab = flag('kebab');
  if (kebab === undefined) {
    do {
      kebab = await ask('英文标识（kebab-case，用于包名/二进制名）', 'pocketark');
      if (!KEBAB_RE.test(kebab)) console.log('  ✗ 需以小写字母开头，仅含小写字母/数字/连字符');
    } while (!KEBAB_RE.test(kebab));
  }

  const libName = kebab.replace(/-/g, '_') + '_lib';
  let identifier =
    flag('id') ??
    (interactive
      ? await ask('应用标识（identifier，发布后不可再改）', `com.pocketark.${kebab}`)
      : `com.pocketark.${kebab}`);
  if (!/^[a-z][a-z0-9.-]*$/.test(identifier)) {
    console.log('  ✗ identifier 需以小写字母开头，仅含小写字母/数字/点/连字符，退出');
    process.exitCode = 1;
    return;
  }

  let accent =
    flag('accent') ??
    (interactive ? await ask(`默认主题色（${ACCENTS.join(' / ')}）`, 'indigo') : 'indigo');
  accent = accent.trim();
  if (!ACCENTS.includes(accent)) {
    console.log(`  ✗ 未知主题色 ${accent}，保持 indigo`);
    accent = 'indigo';
  }

  let theme =
    flag('theme') ?? (interactive ? await ask('默认主题（dark / light）', 'dark') : 'dark');
  theme = theme.trim();
  if (theme !== 'dark' && theme !== 'light') {
    console.log(`  ✗ 未知主题 ${theme}，保持 dark`);
    theme = 'dark';
  }

  console.log(
    interactive ? '' : `配置：${displayName} / ${kebab} / ${identifier} / ${accent} / ${theme}`
  );
  if (interactive) rl.close();

  // ── 构造替换规则（长模式优先，避免短模式误伤长标识符） ────
  const pairs = [
    ['pocketark_lib', libName],
    ['com.hmilyld.pocketark', identifier],
    ['PocketArk', displayName],
    ['pocketark', kebab],
  ];
  // 非默认配置追加精确锚点替换（锚点为 Phase 清扫时抽取的常量行）
  if (theme !== 'dark') {
    pairs.push(
      ["const DEFAULT_THEME: ThemeMode = 'dark';", `const DEFAULT_THEME: ThemeMode = '${theme}';`],
      [
        "if (m !== 'light' && m !== 'dark') m = 'dark';",
        `if (m !== 'light' && m !== 'dark') m = '${theme}';`,
      ]
    );
  }
  if (accent !== 'indigo') {
    pairs.push(["const DEFAULT_ACCENT = 'indigo';", `const DEFAULT_ACCENT = '${accent}';`]);
  }
  pairs.sort((a, b) => b[0].length - a[0].length);

  // ── 遍历改写 ──────────────────────────────────────────────
  let changedFiles = 0;
  const walk = async (dir) => {
    for (const entry of await readdir(dir)) {
      const full = join(dir, entry);
      const rel = relative(ROOT, full);
      const s = await lstat(full);
      if (s.isDirectory()) {
        if (!SKIP_DIRS.has(entry)) await walk(full);
        continue;
      }
      if (SKIP_FILES.has(entry) || !TEXT_EXTS.has(extname(entry))) continue;

      const source = await readFile(full, 'utf8');
      const rewritten = pairs.reduce((text, [from, to]) => text.split(from).join(to), source);
      if (rewritten !== source) {
        await writeFile(full, rewritten);
        changedFiles += 1;
        console.log('  ✓', rel);
      }
    }
  };
  await walk(ROOT);

  // ── 收尾提示 ──────────────────────────────────────────────
  console.log(
    [
      '',
      `✅ 改写完成：${changedFiles} 个文件`,
      '后续步骤：',
      '  1. pnpm install        # 刷新 lockfile 中的包名',
      `  2. pnpm tauri dev      # 以新名字运行（identifier 一经发布不可再改）`,
      '  3. 换图标：改 icon-macos.svg 与 icon-windows.svg 两份源 → pnpm icons',
      '  4. 删除示例插件：可移除 plugins/hello-world 目录；',
      '     Rust 侧 hello_world 模块绑定迁移历史（v1/v2），建议保留不删',
      '  5. 更新 src/content/about.md 与 README.md 的自我介绍',
      '',
    ].join('\n')
  );
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
