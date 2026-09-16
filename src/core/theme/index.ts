/**
 * 主题管理（亮 / 暗 / 跟随系统 + 主题色 + 根字号）。
 *
 * - 偏好存 localStorage（同步读取，供 index.html 内联脚本防 FOUC 闪白）
 * - 暗色实现：documentElement 的 dark class；主题色实现：accent-<id> class
 *   （变量定义见 src/assets/index.css，新增主题色需同步三处：CSS / ACCENTS / 内联脚本）
 * - 默认暗色（暗色指挥台基调）：无偏好 / 非法值时不再跟随系统
 * - 根字号实现：documentElement 上的 --app-font-size 变量（rem 间距随动缩放）
 * - 原生同步（macOS）：applyTheme 会同步 NSWindow.appearance 与 NSWindow.backgroundColor
 *   （lib.rs 的 set_window_appearance / set_window_background）——二者决定 resize / 最大化
 *   时新暴露区域的颜色，必须跟随主题，否则暗色下闪白
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { ipc } from '@/core/ipc';

export type ThemeMode = 'light' | 'dark' | 'system';

/** 默认主题（暗色指挥台基调）：脚手架脚本精确替换此行以切换默认档 */
const DEFAULT_THEME: ThemeMode = 'dark';

/** localStorage 键（单一事实源；index.html 内联脚本与 preview-bridge 需保持一致） */
export const STORAGE_KEYS = {
  theme: 'arkdesk.theme',
  accent: 'arkdesk.accent',
  accentCustom: 'arkdesk.accentCustom',
  fontSize: 'arkdesk.fontSize',
} as const;

const THEME_KEY = STORAGE_KEYS.theme;
const ACCENT_KEY = STORAGE_KEYS.accent;
const ACCENT_CUSTOM_KEY = STORAGE_KEYS.accentCustom;
const FONT_SIZE_KEY = STORAGE_KEYS.fontSize;

/** 自定义主色的 accent id（hex 另存于 ACCENT_CUSTOM_KEY） */
export const CUSTOM_ACCENT_ID = 'custom';

/** 自定义主色需覆盖的语义变量（与 index.css 的 accent-* class 保持一致） */
const CUSTOM_ACCENT_VARS = [
  '--primary',
  '--ring',
  '--sidebar-primary',
  '--sidebar-ring',
  '--primary-foreground',
  '--sidebar-primary-foreground',
] as const;

function normalizeHex(value: string): string | null {
  const hex = value.trim();
  return /^#[0-9a-fA-F]{6}$/.test(hex) ? hex.toLowerCase() : null;
}

/** 依据背景亮度选择前景色（白/近黑），保证主色上的文字可读 */
function contrastForeground(hex: string): string {
  const r = Number.parseInt(hex.slice(1, 3), 16);
  const g = Number.parseInt(hex.slice(3, 5), 16);
  const b = Number.parseInt(hex.slice(5, 7), 16);
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance > 0.6 ? '#111111' : '#ffffff';
}

function applyCustomVars(hex: string): void {
  const foreground = contrastForeground(hex);
  const style = document.documentElement.style;
  style.setProperty('--primary', hex);
  style.setProperty('--ring', hex);
  style.setProperty('--sidebar-primary', hex);
  style.setProperty('--sidebar-ring', hex);
  style.setProperty('--primary-foreground', foreground);
  style.setProperty('--sidebar-primary-foreground', foreground);
}

function clearCustomVars(): void {
  const style = document.documentElement.style;
  for (const name of CUSTOM_ACCENT_VARS) style.removeProperty(name);
}

function clearAccentClasses(): void {
  const root = document.documentElement;
  for (const accent of ACCENTS) root.classList.remove(`accent-${accent.id}`);
}

/** 原生窗口背景色（与 index.css 的 --background 对齐）：
 *  resize / 拖拽时新露出的区域显示该色，避免暗色模式下闪白 */
const NATIVE_BG = {
  dark: [22, 22, 28] as const,
  light: [247, 247, 249] as const,
};

/** 字号档位（语义化：小 / 正常 / 大）。存储值为对应根字号 px（13/14/15），
 *  控件与文字刻度全部按根字号等比派生（见 index.css 的字号刻度注释） */
export const FONT_SIZES = [
  { value: 13, label: '小' },
  { value: 14, label: '正常' },
  { value: 15, label: '大' },
] as const;

export type FontSize = (typeof FONT_SIZES)[number]['value'];

const DEFAULT_FONT_SIZE: FontSize = 14;

export interface Accent {
  id: string;
  label: string;
  /** 设置页色块预览色（与 index.css 中该 accent 的 primary 对应） */
  preview: string;
}

/** 可选主题色（indigo 为默认，无独立 class；其余与 index.css 中 class 一一对应） */
export const ACCENTS: Accent[] = [
  { id: 'indigo', label: '靛蓝', preview: '#6366f1' },
  { id: 'teal', label: '青碧', preview: '#14b8a6' },
  { id: 'amber', label: '琥珀', preview: '#f59e0b' },
  { id: 'rose', label: '玫红', preview: '#f43f5e' },
  { id: 'blue', label: '湛蓝', preview: '#3b82f6' },
  { id: 'grid-green', label: '国家电网绿', preview: '#009944' },
];

const DEFAULT_ACCENT = 'indigo';

const prefersDark =
  typeof window !== 'undefined' && typeof window.matchMedia === 'function'
    ? window.matchMedia('(prefers-color-scheme: dark)')
    : null;

let currentMode: ThemeMode = DEFAULT_THEME;
let currentAccent: string = DEFAULT_ACCENT;

function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // 忽略存储失败，主题仍可在当前会话生效
  }
}

/** 从 localStorage 读取主题偏好，非法值回退 dark（默认基调） */
export function readStoredTheme(): ThemeMode {
  const value = readStorage(THEME_KEY);
  if (value === 'light' || value === 'dark' || value === 'system') return value;
  return DEFAULT_THEME;
}

/** 读取主题色偏好，非法值回退默认（含自定义主色） */
export function readStoredAccent(): string {
  const value = readStorage(ACCENT_KEY);
  if (value === CUSTOM_ACCENT_ID) return CUSTOM_ACCENT_ID;
  return ACCENTS.some((accent) => accent.id === value) ? (value as string) : DEFAULT_ACCENT;
}

/** 读取自定义主色 hex（无 / 非法返回 null） */
export function readStoredCustomAccent(): string | null {
  const value = readStorage(ACCENT_CUSTOM_KEY);
  return value ? normalizeHex(value) : null;
}

/** 读取根字号偏好，非法值回退默认 */
export function readStoredFontSize(): FontSize {
  const value = Number.parseInt(readStorage(FONT_SIZE_KEY) ?? '', 10);
  const valid = FONT_SIZES.some((option) => option.value === value);
  return valid ? (value as FontSize) : DEFAULT_FONT_SIZE;
}

/** 应用主题色（内置 accent class 或自定义 hex，幂等） */
export function applyAccent(accentId: string): void {
  if (accentId === CUSTOM_ACCENT_ID) {
    const hex = readStoredCustomAccent() ?? '#6366f1';
    clearAccentClasses();
    applyCustomVars(hex);
    currentAccent = CUSTOM_ACCENT_ID;
    writeStorage(ACCENT_KEY, CUSTOM_ACCENT_ID);
    return;
  }
  if (!ACCENTS.some((accent) => accent.id === accentId)) return;
  clearCustomVars();
  currentAccent = accentId;
  writeStorage(ACCENT_KEY, accentId);
  const root = document.documentElement;
  for (const accent of ACCENTS) {
    root.classList.toggle(`accent-${accent.id}`, accent.id === accentId);
  }
}

/** 应用自定义主色（hex） */
export function applyCustomAccent(hexInput: string): void {
  const hex = normalizeHex(hexInput);
  if (!hex) return;
  writeStorage(ACCENT_CUSTOM_KEY, hex);
  applyAccent(CUSTOM_ACCENT_ID);
}

/** 应用根字号（幂等）：写入 --app-font-size 变量并持久化 */
export function applyFontSize(size: FontSize): void {
  if (!FONT_SIZES.some((option) => option.value === size)) return;
  writeStorage(FONT_SIZE_KEY, String(size));
  document.documentElement.style.setProperty('--app-font-size', `${size}px`);
}

/** 同步原生背景色（窗口层 + webview 层；失败静默：仅影响 resize 边缘颜色）。
 *  webview 层（禁用 WKWebView 白底）依赖 wry/transparent——经 tauri 的
 *  macos-private-api feature 传递启用 */
async function applyNativeBackground(resolved: 'light' | 'dark'): Promise<void> {
  try {
    const [red, green, blue] = NATIVE_BG[resolved];
    await getCurrentWebview().setBackgroundColor([red, green, blue]);
    await getCurrentWindow().setBackgroundColor([red, green, blue]);
  } catch {
    // 非致命：仅影响窗口缩放时的边缘颜色
  }
}

/** 原生侧同步：窗口外观（resize 暴露区色带颜色）+ NSWindow 背景色（webview 之下兜底） */
function syncNativeAppearance(resolved: 'light' | 'dark'): void {
  const dark = resolved === 'dark';
  void ipc('set_window_appearance', { dark }).catch(() => {
    // 非致命：仅影响 macOS resize 暴露区的色带颜色
  });
  void ipc('set_window_background', { dark }).catch(() => {
    // 非致命：仅影响 macOS resize 暴露区的底色
  });
}

/** 应用亮/暗主题到 DOM 与原生窗口（幂等） */
export function applyTheme(mode: ThemeMode): void {
  currentMode = mode;
  writeStorage(THEME_KEY, mode);

  // 强制模式：解析值确定，立即应用 DOM，原生侧异步跟随
  if (mode !== 'system') {
    document.documentElement.classList.toggle('dark', mode === 'dark');
    void applyNativeBackground(mode);
    syncNativeAppearance(mode);
    return;
  }

  // 跟随系统：必须先清空窗口外观覆盖（NSWindow.appearance 强制值会把 webview 的
  // prefers-color-scheme 钉死在上一次主题上，导致跟随失效），再由原生侧返回
  // 真实系统外观，最后应用 DOM
  void (async () => {
    try {
      const dark = await ipc<boolean>('set_window_appearance', { dark: null });
      const resolved: 'light' | 'dark' = dark ? 'dark' : 'light';
      document.documentElement.classList.toggle('dark', resolved === 'dark');
      await applyNativeBackground(resolved);
      await ipc('set_window_background', { dark: resolved === 'dark' }).catch(() => {});
    } catch {
      // 原生读取失败：退回 media query 兜底（外观可能仍被钉死，属可接受降级）
      const resolved: 'light' | 'dark' = prefersDark?.matches ? 'dark' : 'light';
      document.documentElement.classList.toggle('dark', resolved === 'dark');
      void applyNativeBackground(resolved);
    }
  })();
}

/** 初始化：应用已存储偏好，并监听系统主题变化 */
export function initTheme(): void {
  applyTheme(readStoredTheme());
  applyAccent(readStoredAccent());
  applyFontSize(readStoredFontSize());
  prefersDark?.addEventListener('change', () => {
    if (currentMode === 'system') {
      applyTheme('system');
    }
  });
}

export function getCurrentThemeMode(): ThemeMode {
  return currentMode;
}

export function getCurrentAccent(): string {
  return currentAccent;
}
