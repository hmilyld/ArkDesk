#!/usr/bin/env node
/**
 * 全平台图标生成（双源组装）：
 *
 *   icon-macos.svg   HIG 网格留白版 → icon.icns（macOS Dock/访达）+ Store 磁贴（Square 系列 + StoreLogo）
 *   icon-windows.svg 全出血版      → icon.ico（Windows 任务栏/资源管理器）+ 各级 PNG（Windows/Linux）
 *
 * tauri icon 单次运行只接受一个源，故跑两轮并按平台组装产物；
 * icns 与 ico 分属两个平台各自的打包路径，互不影响。
 * 用法：pnpm icons
 */
import { copyFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execSync } from 'node:child_process';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const ICONS = join(ROOT, 'src-tauri', 'icons');
const MACOS_SRC = join(ICONS, 'icon-macos.svg');
const WINDOWS_SRC = join(ICONS, 'icon-windows.svg');

function run(cmd) {
  execSync(cmd, { stdio: 'inherit', cwd: ROOT });
}

/** tauri icon 会产出全部格式；每轮后把需要的产物暂存/落地 */
const stage = mkdtempSync(join(tmpdir(), 'tauri-icons-'));

console.log('▶ macOS 源（HIG 网格）→ icns + Store 磁贴');
run(`pnpm tauri icon ${MACOS_SRC}`);
for (const file of ['icon.icns', 'StoreLogo.png']) {
  copyFileSync(join(ICONS, file), join(stage, file));
}
for (const size of ['30', '44', '71', '89', '107', '142', '150', '284', '310']) {
  copyFileSync(
    join(ICONS, `Square${size}x${size}Logo.png`),
    join(stage, `Square${size}x${size}Logo.png`)
  );
}

console.log('▶ Windows 源（全出血）→ ico + png');
run(`pnpm tauri icon ${WINDOWS_SRC}`);

console.log('▶ 组装：回填 macOS 产物');
for (const file of ['icon.icns', 'StoreLogo.png']) {
  copyFileSync(join(stage, file), join(ICONS, file));
}
for (const size of ['30', '44', '71', '89', '107', '142', '150', '284', '310']) {
  copyFileSync(
    join(stage, `Square${size}x${size}Logo.png`),
    join(ICONS, `Square${size}x${size}Logo.png`)
  );
}
copyFileSync(join(ICONS, 'icon-windows.svg'), join(ROOT, 'public', 'icon.svg'));
rmSync(stage, { recursive: true, force: true });

console.log('✅ 图标组装完成：icns/磁贴 = macOS 版，ico/png = Windows 版，public/icon.svg 已同步');
