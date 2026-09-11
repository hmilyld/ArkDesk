/**
 * 应用在线更新（框架级能力）。
 *
 * - 检查 / 安装 / 重启均走 Rust 命令（见 src-tauri/src/updater.rs）
 * - 签名公钥固化在内置配置，前端只提供更新服务器地址
 * - 默认仅在启动后自动检查一次；另提供手动检查
 * - 下载进度经 Tauri 事件 `updater://progress` 回传
 */
import { reactive, readonly } from 'vue';
import { onEvent, UpdaterEvent } from '@/core/events';
import { ipc } from '@/core/ipc';
import { logger } from '@/core/logger';
import { normalizeError, ErrorCode } from '@/core/errors';
import { useSettingsStore } from '@/stores/settings';

export interface UpdateInfo {
  /** 服务器公告的新版本号 */
  version: string;
  /** 当前安装版本号 */
  currentVersion: string;
  /** 更新说明（Markdown） */
  notes?: string | null;
  /** 发布日期（RFC 3339） */
  date?: string | null;
}

interface UpdaterState {
  /** 正在检查更新 */
  checking: boolean;
  /** 正在下载 / 安装 */
  installing: boolean;
  /** 安装完成，等待重启 */
  installed: boolean;
  /** 可用更新信息（无更新时为 null） */
  info: UpdateInfo | null;
  downloaded: number;
  total: number | null;
  error: string | null;
  dialogOpen: boolean;
}

const state = reactive<UpdaterState>({
  checking: false,
  installing: false,
  installed: false,
  info: null,
  downloaded: 0,
  total: null,
  error: null,
  dialogOpen: false,
});

export const updaterState = readonly(state);

let listenersReady = false;
let inFlight = false;

/** 启动自动检查延迟（ms）：等待界面就绪后再发请求 */
const STARTUP_CHECK_DELAY = 5000;

async function ensureListeners(): Promise<void> {
  if (listenersReady) return;
  listenersReady = true;
  try {
    await onEvent(UpdaterEvent.Progress, (payload) => {
      state.downloaded = payload.downloaded;
      state.total = payload.total;
    });
    await onEvent(UpdaterEvent.Installed, () => {
      state.installing = false;
      state.installed = true;
    });
  } catch (err) {
    listenersReady = false;
    logger.warn('更新进度事件监听注册失败');
    logger.debug(String(err));
  }
}

function currentEndpoint(): string {
  return useSettingsStore().updateServerUrl.trim();
}

/** 手动触发检查：无论结果如何都反馈（无更新返回 null，异常抛出） */
export async function checkForUpdates(): Promise<UpdateInfo | null> {
  const settings = useSettingsStore();
  if (!settings.updateEnabled) {
    throw { code: ErrorCode.InvalidInput, message: '更新功能未启用' };
  }
  const endpoint = currentEndpoint();
  if (!endpoint) {
    throw { code: ErrorCode.InvalidInput, message: '未配置更新服务器地址' };
  }

  if (inFlight) return state.info;
  inFlight = true;
  state.checking = true;
  state.error = null;
  try {
    const info = await ipc<UpdateInfo | null>('updater_check', { endpoint });
    settings.updateLastCheckAt = Date.now();
    state.info = info;
    if (info) {
      state.installed = false;
      state.downloaded = 0;
      state.total = null;
      state.dialogOpen = true;
    }
    return info;
  } catch (err) {
    const error = normalizeError(err);
    state.error = error.message;
    throw error;
  } finally {
    state.checking = false;
    inFlight = false;
  }
}

/** 下载并安装已检测到的更新（进度经 updaterState 反映） */
export async function installUpdate(): Promise<void> {
  const endpoint = currentEndpoint();
  if (!endpoint) {
    throw { code: ErrorCode.InvalidInput, message: '未配置更新服务器地址' };
  }
  if (!state.info) {
    throw { code: ErrorCode.InvalidInput, message: '没有可安装的更新' };
  }

  state.installing = true;
  state.error = null;
  state.downloaded = 0;
  state.total = null;
  try {
    await ipc('updater_install', { endpoint });
    // Windows 会在安装器启动后结束进程；其余平台命令正常返回
    state.installed = true;
  } catch (err) {
    const error = normalizeError(err);
    state.error = error.message;
    throw error;
  } finally {
    state.installing = false;
  }
}

/** 重启应用以运行新版本（命令执行后进程即重启，不再返回） */
export async function restartApp(): Promise<void> {
  await ipc('updater_restart');
}

export function dismissUpdateDialog(): void {
  state.dialogOpen = false;
}

export function openUpdateDialog(): void {
  if (state.info) state.dialogOpen = true;
}

/** 应用启动时调用一次：注册事件监听，并按设置触发一次自动检查 */
export async function initUpdater(): Promise<void> {
  await ensureListeners();

  const settings = useSettingsStore();
  if (!settings.updateEnabled || !settings.updateAutoCheck || !currentEndpoint()) return;

  setTimeout(() => {
    void checkForUpdates().catch((err) => {
      // 启动静默检查：失败仅记日志，不打扰用户
      logger.warn(`启动检查更新失败: ${normalizeError(err).message}`);
    });
  }, STARTUP_CHECK_DELAY);
}
