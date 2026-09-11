/**
 * 插件系统类型定义。
 *
 * 两层形态：
 * - ToolManifest / ManifestTool：plugin.json 的纯数据形态（清单，由 loader 解析）
 * - ToolPlugin / ToolMeta：解析后的运行时形态（icon 已解析为组件、组件已挂载）
 *
 * 目录约定（`plugins/<id>/`，plugin.json 与 frontend/、backend/ 同级）：
 *   frontend/views/<entry>     工具页面（manifest.tools[].entry 指向）
 *   frontend/settings/<entry>  设置面板（manifest.settings.entry 指向）
 *   frontend/setup.ts          可选：生命周期钩子（默认导出，自动扫描）
 *   frontend/schema.ts         可选：数据库表定义（自动聚合进 core/db）
 *   frontend/shared.ts         插件内共享常量/工具
 *   frontend/components/       插件私有组件
 *   backend/mod.rs             Rust 命令（构建期自动扫描 @tauri::command）
 *   backend/migrations.rs      Rust 迁移（构建期自动聚合）
 * `_` 开头的目录不参与注册（如 _template）。
 */
import type { Component } from 'vue';
import type { LucideIcon } from '@lucide/vue';
import type { PluginContext } from './context';

/** plugin.json 中的单个工具项声明 */
export interface ManifestTool {
  /** 工具 id，全局唯一（= 路由 /tool/:id） */
  id: string;
  /** 显示名（侧边导航、标题栏） */
  name: string;
  /** 工具描述（设置页树形列表展示） */
  description?: string;
  /** 页面入口，相对插件目录（如 "frontend/views/Tool.vue"） */
  entry: string;
  /** 导航图标（lucide 图标名，如 "Rocket"）；缺省回退清单顶层 icon */
  icon?: string;
  /** 切走后是否保留状态（默认 true，经 KeepAlive 缓存） */
  keepAlive?: boolean;
}

/** plugin.json 清单（插件唯一事实源，纯数据） */
export interface ToolManifest {
  /** 插件 id（= 目录名约定）；工具项 id 需以此为前缀避免跨插件冲突 */
  id: string;
  /** 插件显示名（设置页树形列表的插件行） */
  name: string;
  /** 插件描述（设置页树形列表展示） */
  description?: string;
  /** 插件级缺省图标（lucide 图标名；工具项未指定时回退） */
  icon?: string;
  /** 导航分组名（缺省归入「其他」） */
  group?: string;
  /** 插件排序权重（升序，缺省按目录名字母序）；组内顺序 = tools 数组序 */
  order?: number;
  /** 搜索关键词（预留给全局搜索） */
  keywords?: string[];
  /**
   * 旧库迁移桥接（可选）：旧全局版本号 → 本插件作用域内版本号。
   * 仅由 `src-tauri/build.rs` 读取，用于把历史 `_sqlx_migrations` 登记进 `plugin_migrations`。
   */
  legacyMigrations?: Record<string, number>;
  /** 工具项（单工具插件也写一项） */
  tools: ManifestTool[];
  /** 设置面板声明（可选；面板为插件级，tab 名 = 插件显示名），entry 如 "frontend/settings/Settings.vue" */
  settings?: { entry: string };
}

/** 插件生命周期钩子：启动期执行一次，ctx 提供框架能力（日志、数据库等） */
export type PluginSetup = (ctx: PluginContext) => void | Promise<void>;

/** 解析后的工具项（注册表运行时形态） */
export interface ToolMeta {
  id: string;
  name: string;
  /** 工具描述（设置页树形列表展示） */
  description?: string;
  /** 归属插件 id（树形分组依据） */
  pluginId: string;
  icon: LucideIcon;
  group?: string;
  order?: number;
  keywords?: string[];
}

export interface ToolPlugin {
  meta: ToolMeta;
  /** 页面组件（懒加载） */
  component: () => Promise<{ default: Component }>;
  /** 切走后是否保留状态（默认 true，经 KeepAlive 缓存） */
  keepAlive?: boolean;
  /** 设置面板（插件级，挂在该插件首个工具项上；设置页自动生成 tab） */
  settings?: { label?: string; component: () => Promise<{ default: Component }> };
}

/** 插件分组信息（设置页树形列表的数据源，含全部工具不论启停） */
export interface PluginGroupInfo {
  id: string;
  name: string;
  description?: string;
  icon?: LucideIcon;
  tools: ToolPlugin[];
}
