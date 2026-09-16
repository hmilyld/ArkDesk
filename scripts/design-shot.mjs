#!/usr/bin/env node
/**
 * 设计走查截图（本地层工具，fork-owned）。
 *
 * 用无头 Chromium（Edge/Chrome）+ CDP 打开 vite dev 的页面，可选执行一段 JS
 * （点击、输入、聚焦等）后再截图，供设计走查与规范核对使用。
 *
 * 前置：`npx vite` 已在 1420 端口运行（浏览器预览桥会自动模拟 Tauri API）。
 *
 * 用法：
 *   node scripts/local/design-shot.mjs --url "#/tool/json-table" --out /tmp/a.png
 *   node scripts/local/design-shot.mjs --url "#/settings" --out b.png \
 *     --eval "document.querySelectorAll('[role=tab]')[2].click()"
 *   --width 1512 --height 982 --wait 900 --theme dark --font 14 --platform win
 */
import { spawn } from 'node:child_process';
import { existsSync, writeFileSync } from 'node:fs';
import { setTimeout as delay } from 'node:timers/promises';

const BROWSERS = [
  '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium',
];

function arg(name, fallback) {
  const index = process.argv.indexOf(`--${name}`);
  return index === -1 ? fallback : process.argv[index + 1];
}

const browser = BROWSERS.find(existsSync);
if (!browser) {
  console.error('未找到 Chromium 系浏览器（Edge/Chrome）');
  process.exit(1);
}

const base = `http://localhost:${arg('port', '1420')}`;
const width = Number(arg('width', 1512));
const height = Number(arg('height', 982));
const waitMs = Number(arg('wait', 900));
const out = arg('out');
if (!out) {
  console.error('缺少 --out <png 路径>');
  process.exit(1);
}

let routePath = (arg('url', '/') || '/').replace(/^#/, '');
if (!routePath.startsWith('/')) routePath = `/${routePath}`;
const hash = `#${routePath}`;
const query = new URLSearchParams();
for (const key of ['theme', 'font', 'accent', 'platform']) {
  const value = arg(key);
  if (value) query.set(key, value);
}
const target = `${base}/${hash}${query.toString() ? `?${query}` : ''}`;

const cdpPort = 9200 + (process.pid % 400);
const child = spawn(
  browser,
  [
    '--headless=new',
    '--disable-gpu',
    '--hide-scrollbars',
    `--remote-debugging-port=${cdpPort}`,
    `--window-size=${width},${height}`,
    '--no-first-run',
    '--user-data-dir=/tmp/design-shot-profile',
    'about:blank',
  ],
  { stdio: 'ignore' }
);

async function endpoint() {
  for (let i = 0; i < 40; i += 1) {
    try {
      const res = await fetch(`http://127.0.0.1:${cdpPort}/json/list`);
      const list = await res.json();
      const page = list.find((t) => t.type === 'page');
      if (page?.webSocketDebuggerUrl) return page.webSocketDebuggerUrl;
    } catch {
      // 浏览器还没起来
    }
    await delay(150);
  }
  throw new Error('无法连接浏览器调试端口');
}

const wsUrl = await endpoint();
const ws = new WebSocket(wsUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});

let seq = 0;
const pending = new Map();
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg.result ?? {});
    pending.delete(msg.id);
  }
};

function send(method, params = {}) {
  const id = ++seq;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });
}

try {
  await send('Page.enable');
  await send('Runtime.enable');
  await send('Page.navigate', { url: target });
  await delay(Number(arg('boot', 2500)));

  const script = arg('eval');
  if (script) {
    await send('Runtime.evaluate', { expression: script, awaitPromise: true });
    await delay(waitMs);
  }

  const shot = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(out, Buffer.from(shot.data, 'base64'));
  console.log(`已截图 ${out} ← ${target}${script ? `（并执行了 --eval）` : ''}`);
} finally {
  ws.close();
  child.kill('SIGKILL');
}
