/**
 * 工具插件注册表。
 *
 * - loader 完成解析后调用 registerTool / registerPluginGroup 完成注册（启动期一次性）
 * - 侧边导航、路由均由注册表驱动生成
 */
import type { PluginGroupInfo, ToolPlugin } from './types';

const DEFAULT_GROUP = '其他';
const DEFAULT_ORDER = 100;

const tools = new Map<string, ToolPlugin>();
const pluginGroups = new Map<string, PluginGroupInfo>();

/** 注册工具插件（重复 id 视为开发错误，仅警告并保留先注册者） */
export function registerTool(tool: ToolPlugin): void {
  const { id } = tool.meta;
  if (tools.has(id)) {
    console.warn(`[registry] 工具 id 重复，已忽略: ${id}`);
    return;
  }
  tools.set(id, tool);
}

/** 注册插件分组信息（重复注册后到者覆盖：工具列表随最后注册更新） */
export function registerPluginGroup(group: PluginGroupInfo): void {
  pluginGroups.set(group.id, group);
}

/** 获取全部已注册工具（分组序 = 组内最小 order → 组名 → 工具序 → 名称） */
export function getAllTools(): ToolPlugin[] {
  // 分组序取组内最小 order，与 getPluginGroups（按首工具 order）保持一致
  const groupOrder = new Map<string, number>();
  for (const tool of tools.values()) {
    const group = tool.meta.group ?? DEFAULT_GROUP;
    const order = tool.meta.order ?? DEFAULT_ORDER;
    const current = groupOrder.get(group);
    if (current === undefined || order < current) {
      groupOrder.set(group, order);
    }
  }

  return [...tools.values()].sort((a, b) => {
    const ga = a.meta.group ?? DEFAULT_GROUP;
    const gb = b.meta.group ?? DEFAULT_GROUP;
    const groupDiff = (groupOrder.get(ga) ?? DEFAULT_ORDER) - (groupOrder.get(gb) ?? DEFAULT_ORDER);
    if (groupDiff !== 0) return groupDiff;
    if (ga !== gb) return ga.localeCompare(gb, 'zh-Hans-CN');

    const orderDiff = (a.meta.order ?? DEFAULT_ORDER) - (b.meta.order ?? DEFAULT_ORDER);
    if (orderDiff !== 0) return orderDiff;

    return a.meta.name.localeCompare(b.meta.name, 'zh-Hans-CN');
  });
}

export function getToolById(id: string): ToolPlugin | undefined {
  return tools.get(id);
}

/** 收集声明了设置面板的工具（排序规则同 getAllTools） */
export function getToolsWithSettings(): ToolPlugin[] {
  return getAllTools().filter((tool) => tool.settings !== undefined);
}

/** 插件分组（树形数据源：插件 → 工具，含禁用工具），排序规则同 getAllTools */
export function getPluginGroups(): PluginGroupInfo[] {
  return [...pluginGroups.values()].sort((a, b) => {
    const orderDiff =
      (a.tools[0]?.meta.order ?? DEFAULT_ORDER) - (b.tools[0]?.meta.order ?? DEFAULT_ORDER);
    if (orderDiff !== 0) return orderDiff;
    return a.id.localeCompare(b.id);
  });
}

export function getToolRoutePath(id: string): string {
  return `/tool/${id}`;
}
