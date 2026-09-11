/**
 * 工具设置存储约定。
 *
 * - 所有工具的配置统一存 `settings.json`（store 插件），命名空间 key：`tools.<toolId>`
 * - useToolSettings 返回响应式配置对象，deep watch 修改即自动持久化
 * - 读取时与 defaults 深合并：后期新增配置字段能自动获得默认值
 */
import { ref, watch, type Ref } from 'vue';
import { Store } from '@tauri-apps/plugin-store';
import { logger } from '@/core/logger';

const SETTINGS_FILE = 'settings.json';

function namespaceKey(toolId: string): string {
  return `tools.${toolId}`;
}

/** 简单深合并：saved 优先，defaults 兜底（仅处理普通对象，不含数组特殊合并） */
function deepMerge<T extends object>(defaults: T, saved: unknown): T {
  if (saved === null || typeof saved !== 'object' || Array.isArray(saved)) {
    return defaults;
  }
  const result = { ...defaults } as Record<string, unknown>;
  for (const [key, value] of Object.entries(saved as Record<string, unknown>)) {
    if (!(key in result)) continue; // 丢弃 defaults 中已不存在的遗留字段
    const fallback = result[key];
    if (
      fallback !== null &&
      typeof fallback === 'object' &&
      !Array.isArray(fallback) &&
      value !== null &&
      typeof value === 'object' &&
      !Array.isArray(value)
    ) {
      result[key] = deepMerge(fallback, value);
    } else if (typeof value === typeof fallback) {
      result[key] = value;
    }
  }
  return result as T;
}

const storePromise: Promise<Store> = Store.load(SETTINGS_FILE);

/**
 * 工具设置读写 composable。
 *
 * ```ts
 * const config = useToolSettings("my-tool", { fontSize: 12 });
 * config.value.fontSize = 14; // 修改即自动持久化
 * ```
 */
export function useToolSettings<T extends object>(toolId: string, defaults: T): Ref<T> {
  const config = ref(deepMerge(defaults, null)) as Ref<T>;
  /** 已从存储完成初始填充：期间不触发持久化，避免用合并值覆盖已存配置 */
  let hydrated = false;

  // watch 同步注册（顶层），组件卸载时随作用域自动清理，不依赖异步时序
  watch(
    config,
    async (value) => {
      if (!hydrated) return;
      try {
        const store = await storePromise;
        await store.set(namespaceKey(toolId), JSON.parse(JSON.stringify(value)));
      } catch (err) {
        logger.warn(`工具设置持久化失败: ${toolId}`);
        logger.debug(String(err));
      }
    },
    { deep: true }
  );

  void storePromise
    .then(async (store) => {
      const saved = await store.get<unknown>(namespaceKey(toolId));
      config.value = deepMerge(defaults, saved);
      hydrated = true;
    })
    .catch((err) => {
      // 加载失败时保留默认值，仍允许当前会话内修改
      hydrated = true;
      logger.error(`工具设置加载失败: ${toolId}`);
      logger.debug(String(err));
    });

  return config;
}
