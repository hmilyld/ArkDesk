/**
 * 运行平台探测（仅用于窗口装饰差异）。
 *
 * Tauri webview 的 UA 可靠：macOS 含 "Mac"，Windows 含 "Windows NT"。
 */

export const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent);
