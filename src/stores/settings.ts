/**
 * 全局设置 store。
 *
 * - 持久化：tauri-plugin-store（settings.json，自动保存）
 * - 主题例外：存 localStorage（防 FOUC 需同步读取，见 core/theme）
 * - 日志级别：通过 ipc 命令运行时生效（Rust 侧 log::set_max_level）
 */
import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { Store } from '@tauri-apps/plugin-store';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { applyTheme, readStoredTheme, type ThemeMode } from '@/core/theme';
import { applyAutoStart, readAutoStart } from '@/core/autostart';
import { applyGlobalShortcut } from '@/core/global-shortcut';
import { logger } from '@/core/logger';
import { ipc } from '@/core/ipc';

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

const isLogLevel = (value: unknown): value is LogLevel =>
  typeof value === 'string' && ['trace', 'debug', 'info', 'warn', 'error'].includes(value);

export const useSettingsStore = defineStore('settings', () => {
  const ready = ref(false);
  const themeMode = ref<ThemeMode>(readStoredTheme());
  const closeToTray = ref(true);
  const logLevel = ref<LogLevel>('debug');
  /** 侧栏收起（图标 rail）：收起态无需防闪白，走 settings.json 持久化 */
  const sidebarCollapsed = ref(false);
  /** 各工具禁用状态（未记录的工具视为启用） */
  const toolDisabled = ref<Record<string, boolean>>({});

  // AI 接口配置（全局，供 AI 能力复用；明文存储于 settings.json）
  const aiBaseUrl = ref('');
  const aiApiKey = ref('');
  const aiModel = ref('gpt-4o-mini');

  // 在线更新（框架级能力，见 core/updater）
  const updateEnabled = ref(true);
  const updateServerUrl = ref('https://arkdesk.hmilyld.com');
  /** 启动后自动检查一次（总开关开启时生效） */
  const updateAutoCheck = ref(true);
  const updateLastCheckAt = ref<number | null>(null);

  // 系统集成（框架能力）
  /** 系统通知总开关（见 core/notify） */
  const notificationEnabled = ref(true);
  /** 开机自启（真相源在系统，启动时回填） */
  const autoStart = ref(false);
  /** 全局快捷键（唤起主窗口；空 = 禁用） */
  const globalShortcut = ref('');
  /** HTTP 代理（空 = 直连），如 http://127.0.0.1:7890 */
  const proxyUrl = ref('');

  let store: Store | null = null;

  async function persist(key: string, value: unknown): Promise<void> {
    if (!store) return;
    try {
      await store.set(key, value);
    } catch (err) {
      logger.warn(`设置持久化失败: ${key}`);
      logger.debug(String(err));
    }
  }

  function isToolEnabled(toolId: string): boolean {
    return !toolDisabled.value[toolId];
  }

  function setToolEnabled(toolId: string, enabled: boolean): void {
    toolDisabled.value = { ...toolDisabled.value, [toolId]: !enabled };
  }

  /** 导出：settings.json 全部键值快照 */
  async function snapshot(): Promise<Record<string, unknown>> {
    if (!store) return {};
    const entries = await store.entries();
    return Array.isArray(entries)
      ? Object.fromEntries(entries)
      : (entries as Record<string, unknown>);
  }

  /** 导入：写入全部键值（重启后生效） */
  async function importRaw(data: Record<string, unknown>): Promise<void> {
    if (!store) return;
    for (const [key, value] of Object.entries(data)) {
      await store.set(key, value);
    }
  }

  /** 应用启动时调用一次：加载持久化设置并绑定副作用 */
  async function init(): Promise<void> {
    if (ready.value) return;

    /** 全局副作用（自启/全局快捷键/代理）仅在主窗口执行，避免多窗口重复注册 */
    const isMainWindow = getCurrentWindow().label === 'main';

    try {
      store = await Store.load('settings.json');
    } catch (err) {
      // 设置文件损坏时降级为默认值，不阻断应用启动
      logger.warn('设置文件加载失败，使用默认设置');
      logger.debug(String(err));
    }

    if (store) {
      const savedCloseToTray = await store.get<boolean>('closeToTray');
      if (typeof savedCloseToTray === 'boolean') closeToTray.value = savedCloseToTray;

      const savedSidebarCollapsed = await store.get<boolean>('sidebarCollapsed');
      if (typeof savedSidebarCollapsed === 'boolean')
        sidebarCollapsed.value = savedSidebarCollapsed;

      const savedLogLevel = await store.get<string>('logLevel');
      if (isLogLevel(savedLogLevel)) logLevel.value = savedLogLevel;

      const savedToolDisabled = await store.get<Record<string, boolean>>('toolDisabled');
      if (savedToolDisabled && typeof savedToolDisabled === 'object') {
        toolDisabled.value = savedToolDisabled;
      }

      const savedAiBaseUrl = await store.get<string>('aiBaseUrl');
      if (typeof savedAiBaseUrl === 'string') aiBaseUrl.value = savedAiBaseUrl;
      const savedAiApiKey = await store.get<string>('aiApiKey');
      if (typeof savedAiApiKey === 'string') aiApiKey.value = savedAiApiKey;
      const savedAiModel = await store.get<string>('aiModel');
      if (typeof savedAiModel === 'string') aiModel.value = savedAiModel;

      const savedUpdateEnabled = await store.get<boolean>('updateEnabled');
      if (typeof savedUpdateEnabled === 'boolean') updateEnabled.value = savedUpdateEnabled;
      const savedUpdateServerUrl = await store.get<string>('updateServerUrl');
      if (typeof savedUpdateServerUrl === 'string') updateServerUrl.value = savedUpdateServerUrl;
      const savedUpdateAutoCheck = await store.get<boolean>('updateAutoCheck');
      if (typeof savedUpdateAutoCheck === 'boolean') updateAutoCheck.value = savedUpdateAutoCheck;
      const savedUpdateLastCheckAt = await store.get<number>('updateLastCheckAt');
      if (typeof savedUpdateLastCheckAt === 'number')
        updateLastCheckAt.value = savedUpdateLastCheckAt;

      const savedNotificationEnabled = await store.get<boolean>('notificationEnabled');
      if (typeof savedNotificationEnabled === 'boolean')
        notificationEnabled.value = savedNotificationEnabled;

      const savedGlobalShortcut = await store.get<string>('globalShortcut');
      if (typeof savedGlobalShortcut === 'string') globalShortcut.value = savedGlobalShortcut;

      const savedProxyUrl = await store.get<string>('proxyUrl');
      if (typeof savedProxyUrl === 'string') proxyUrl.value = savedProxyUrl;
    }

    // 开机自启：以系统真实状态为准回填（可能被外部修改），须在注册 watcher 前完成
    if (isMainWindow) {
      autoStart.value = await readAutoStart();
      // 全局快捷键：注册已保存的快捷键
      await applyGlobalShortcut(globalShortcut.value);
      // HTTP 代理：应用到 Rust 侧 Client
      void ipc('http_set_proxy', { proxy: proxyUrl.value || null }).catch(() => {
        // 失败静默：代理仍会持久化，下次启动重试
      });
    }

    // 主题：应用偏好（initTheme 已处理首次应用，此处响应运行时变更）
    watch(themeMode, (mode) => applyTheme(mode));

    // 其余设置变更持久化，日志级别即时同步到 Rust 侧
    watch(closeToTray, (value) => void persist('closeToTray', value));
    watch(sidebarCollapsed, (value) => void persist('sidebarCollapsed', value));
    watch(logLevel, (value) => {
      void persist('logLevel', value);
      void ipc('set_log_level', { level: value }).catch(() => {
        // 命令失败时静默：级别仍会持久化，下次启动生效
      });
    });
    watch(toolDisabled, (value) => void persist('toolDisabled', value), { deep: true });

    watch(aiBaseUrl, (value) => void persist('aiBaseUrl', value));
    watch(aiApiKey, (value) => void persist('aiApiKey', value));
    watch(aiModel, (value) => void persist('aiModel', value));

    watch(updateEnabled, (value) => void persist('updateEnabled', value));
    watch(updateServerUrl, (value) => void persist('updateServerUrl', value));
    watch(updateAutoCheck, (value) => void persist('updateAutoCheck', value));
    watch(updateLastCheckAt, (value) => void persist('updateLastCheckAt', value));

    watch(notificationEnabled, (value) => void persist('notificationEnabled', value));
    watch(autoStart, (value) => {
      void persist('autoStart', value);
      if (isMainWindow) void applyAutoStart(value);
    });
    watch(globalShortcut, (value) => {
      void persist('globalShortcut', value);
      if (isMainWindow) void applyGlobalShortcut(value);
    });
    watch(proxyUrl, (value) => {
      void persist('proxyUrl', value);
      if (isMainWindow) {
        void ipc('http_set_proxy', { proxy: value || null }).catch(() => {
          // 失败静默
        });
      }
    });

    ready.value = true;
    logger.debug('settings store initialized');
  }

  return {
    ready,
    themeMode,
    closeToTray,
    sidebarCollapsed,
    logLevel,
    toolDisabled,
    aiBaseUrl,
    aiApiKey,
    aiModel,
    updateEnabled,
    updateServerUrl,
    updateAutoCheck,
    updateLastCheckAt,
    notificationEnabled,
    autoStart,
    globalShortcut,
    proxyUrl,
    isToolEnabled,
    setToolEnabled,
    snapshot,
    importRaw,
    init,
  };
});
