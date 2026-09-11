/**
 * 应用内快捷键注册表（框架能力）。
 *
 * combo 形如 `mod+k`：`mod` = macOS 的 Cmd / 其它平台的 Ctrl，可组合 `alt` / `shift`。
 * 页面/插件可注册快捷键，返回取消注册函数。全局仅一个 keydown 监听。
 */
import { isMac } from '@/core/platform';

type Handler = (event: KeyboardEvent) => void;

const registry = new Map<string, Handler>();
let installed = false;

function normalize(event: KeyboardEvent): string {
  const parts: string[] = [];
  if (isMac ? event.metaKey : event.ctrlKey) parts.push('mod');
  if (event.altKey) parts.push('alt');
  if (event.shiftKey) parts.push('shift');
  parts.push(event.key.toLowerCase());
  return parts.join('+');
}

function onKeydown(event: KeyboardEvent): void {
  const handler = registry.get(normalize(event));
  if (handler) {
    event.preventDefault();
    handler(event);
  }
}

/** 注册快捷键，返回取消注册函数 */
export function registerShortcut(combo: string, handler: Handler): () => void {
  if (!installed) {
    window.addEventListener('keydown', onKeydown);
    installed = true;
  }
  const key = combo.toLowerCase();
  registry.set(key, handler);
  return () => registry.delete(key);
}
