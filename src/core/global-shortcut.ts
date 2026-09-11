/**
 * 全局快捷键（框架能力）。
 *
 * 当前用于「唤起主窗口」：设置项 `globalShortcut` 为空表示禁用。
 * 变更时先全部注销再重新注册，避免残留。
 */
import { ref } from 'vue';
import { register, unregisterAll, type ShortcutEvent } from '@tauri-apps/plugin-global-shortcut';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { logger } from '@/core/logger';

let current: string | null = null;

/** 最近一次注册失败信息（供设置页展示），成功后清空 */
export const globalShortcutError = ref<string | null>(null);

const MODIFIER_KEYS = new Set([
  'ControlLeft',
  'ControlRight',
  'MetaLeft',
  'MetaRight',
  'AltLeft',
  'AltRight',
  'ShiftLeft',
  'ShiftRight',
  'CapsLock',
]);

/** global-hotkey 解析器支持的键位（KeyboardEvent.code 命名） */
const SUPPORTED_CODE_RE =
  /^(Key[A-Z]|Digit[0-9]|F([1-9]|1[0-9]|2[0-4])|Arrow(Up|Down|Left|Right)|Backquote|Backslash|Bracket(Left|Right)|Comma|Equal|Minus|Period|Quote|Semicolon|Slash|Backspace|CapsLock|Enter|Space|Tab|Delete|End|Home|Insert|Page(Up|Down)|PrintScreen|ScrollLock|Pause)$/;

/**
 * 由键盘事件构造 Tauri 加速键字符串（如 `CommandOrControl+Shift+KeyP`）。
 * 必须带至少一个非 Shift 修饰键；纯修饰键 / 无修饰键返回 null。
 * `isMac` 决定 Cmd↔Ctrl 是否归一为 `CommandOrControl`（跨平台语义一致）。
 */
export function acceleratorFromEvent(event: KeyboardEvent, isMac: boolean): string | null {
  if (MODIFIER_KEYS.has(event.code)) return null;

  const mods: string[] = [];
  if (isMac) {
    if (event.metaKey) mods.push('CommandOrControl');
    if (event.ctrlKey) mods.push('Control');
  } else {
    if (event.ctrlKey) mods.push('CommandOrControl');
    if (event.metaKey) mods.push('Super');
  }
  if (event.altKey) mods.push('Alt');
  if (event.shiftKey) mods.push('Shift');

  const hasPrimary = mods.some((mod) => mod !== 'Shift');
  if (!event.code || !SUPPORTED_CODE_RE.test(event.code)) return null;
  if (!hasPrimary) return null;
  return [...mods, event.code].join('+');
}

/** 展示用格式化（`CommandOrControl+Shift+KeyP` → macOS `⇧⌘P` / 其它 `Ctrl+Shift+P`） */
export function formatAccelerator(accel: string, isMac: boolean): string {
  if (!accel) return '';
  const parts = accel.split('+');
  const key = parts[parts.length - 1] ?? '';
  const label = key.replace(/^Key/, '').replace(/^Digit/, '');
  const modLabels: Record<string, string> = isMac
    ? { CommandOrControl: '⌘', Super: '⌘', Control: '⌃', Alt: '⌥', Shift: '⇧' }
    : { CommandOrControl: 'Ctrl', Super: 'Win', Control: 'Ctrl', Alt: 'Alt', Shift: 'Shift' };
  const mods = parts.slice(0, -1).map((mod) => modLabels[mod] ?? mod);
  if (isMac) return mods.join('').concat(label.toUpperCase());
  return [...mods, label.toUpperCase()].join('+');
}

/** 应用全局快捷键设置（空字符串 = 禁用） */
export async function applyGlobalShortcut(shortcut: string): Promise<void> {
  const value = shortcut.trim();
  if (value === current && globalShortcutError.value === null) return;
  try {
    await unregisterAll();
    current = null;
    globalShortcutError.value = null;
    if (!value) return;

    const win = getCurrentWindow();
    await register(value, (event: ShortcutEvent) => {
      if (event.state === 'Pressed') {
        void win.show();
        void win.setFocus();
      }
    });
    current = value;
    logger.debug(`全局快捷键已注册: ${value}`);
  } catch (err) {
    globalShortcutError.value = '注册失败：该组合可能已被系统或其它应用占用';
    logger.warn(`全局快捷键注册失败: ${value}（可能已被占用）`);
    logger.debug(String(err));
  }
}
