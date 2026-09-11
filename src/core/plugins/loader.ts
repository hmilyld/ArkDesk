/**
 * 插件装载器：构建期扫描 + 开发期校验，全自动注册。
 *
 * 四路 import.meta.glob（Vite 构建期静态收集，无运行时遍历），
 * 扫描仓库根 `plugins/<id>/frontend/`：plugin.json（清单，eager）、views/（工具页面，
 * 懒加载表）、settings/（设置面板，懒加载表）、setup.ts（生命周期，eager）。
 *
 * 开发期校验（console.error 后跳过该项，不阻断应用）：
 * 清单解析失败、工具 id 跨插件重复、entry/settings 路径不存在、icon 名无效。
 * 下划线开头的目录不参与注册（如 _template）。
 *
 * 后端（Rust）同样构建期自动注册：见 `src-tauri/build.rs` 扫描 `plugins/<id>/backend/mod.rs`。
 */

import type { Component } from 'vue';
import { icons, type LucideIcon } from '@lucide/vue';

import type { PluginContext } from './context';
import { registerPluginGroup, registerTool } from './registry';
import type { PluginGroupInfo, PluginSetup, ToolManifest, ToolPlugin } from './types';

const PLUGINS_DIR_RE = /\/plugins\/([^/]+)\//;
const FALLBACK_ICON = 'Puzzle';

const manifests = import.meta.glob<ToolManifest>('/plugins/*/plugin.json', {
  eager: true,
  import: 'default',
});
const views = import.meta.glob<{ default: Component }>('/plugins/*/frontend/views/*.vue');
const settingPanels = import.meta.glob<{ default: Component }>(
  '/plugins/*/frontend/settings/*.vue'
);
const setups = import.meta.glob<PluginSetup>('/plugins/*/frontend/setup.ts', {
  eager: true,
  import: 'default',
});

function pluginDirOf(path: string): string {
  return PLUGINS_DIR_RE.exec(path)?.[1] ?? '';
}

const ICON_TABLE = icons as unknown as Record<string, LucideIcon>;

function resolveIcon(name: string | undefined, pluginDir: string): LucideIcon {
  if (!name) {
    console.error(`[plugins] ${pluginDir}: 未指定图标，使用兜底图标`);
    return ICON_TABLE[FALLBACK_ICON];
  }
  const icon = ICON_TABLE[name];
  if (!icon) {
    console.error(`[plugins] ${pluginDir}: 未知 lucide 图标 "${name}"，使用兜底图标`);
    return ICON_TABLE[FALLBACK_ICON];
  }
  return icon;
}

/** 解析全部清单（按插件 order → 目录名字母序 → tools 数组序排列） */
function resolvePlugins(): { tools: ToolPlugin[]; groups: PluginGroupInfo[] } {
  const sortedManifests = Object.entries(manifests).sort(
    ([pathA, manifestA], [pathB, manifestB]) => {
      const orderDiff = (manifestA.order ?? 100) - (manifestB.order ?? 100);
      if (orderDiff !== 0) return orderDiff;
      return pluginDirOf(pathA).localeCompare(pluginDirOf(pathB));
    }
  );

  const seenToolIds = new Set<string>();
  const tools: ToolPlugin[] = [];
  const groups: PluginGroupInfo[] = [];

  for (const [manifestPath, manifest] of sortedManifests) {
    const dir = pluginDirOf(manifestPath);
    if (!dir || dir.startsWith('_')) continue;

    if (!manifest.name) {
      console.error(`[plugins] ${dir}: 清单缺少 name 字段（插件显示名），回退为 id`);
    }
    const pluginName = manifest.name || manifest.id;

    const settingsPanel = manifest.settings
      ? settingPanels[`/plugins/${dir}/${manifest.settings.entry}`]
      : undefined;
    if (manifest.settings && !settingsPanel) {
      console.error(
        `[plugins] ${dir}: 设置面板入口不存在: ${manifest.settings.entry}（已忽略插件级设置）`
      );
    }

    const groupTools: ToolPlugin[] = [];

    manifest.tools.forEach((tool, index) => {
      if (seenToolIds.has(tool.id)) {
        console.error(`[plugins] 工具 id 重复，已跳过: ${tool.id}（${dir}）`);
        return;
      }
      const component = views[`/plugins/${dir}/${tool.entry}`];
      if (!component) {
        console.error(
          `[plugins] ${dir}: 工具 "${tool.id}" 的入口不存在: ${tool.entry}（已跳过该工具）`
        );
        return;
      }
      seenToolIds.add(tool.id);

      const resolved: ToolPlugin = {
        meta: {
          id: tool.id,
          name: tool.name,
          description: tool.description,
          pluginId: manifest.id,
          icon: resolveIcon(tool.icon ?? manifest.icon, dir),
          group: manifest.group,
          // 组间顺序由插件 order 决定，组内顺序由 tools 数组序决定
          order: (manifest.order ?? 100) * 1000 + index,
          keywords: manifest.keywords,
        },
        component,
        keepAlive: tool.keepAlive ?? true,
        // 设置面板为插件级，挂在插件首个工具项上（设置页 tab 以插件名展示）
        settings:
          index === 0 && settingsPanel
            ? { label: pluginName, component: settingsPanel }
            : undefined,
      };
      tools.push(resolved);
      groupTools.push(resolved);
    });

    if (groupTools.length > 0) {
      groups.push({
        id: manifest.id,
        name: pluginName,
        description: manifest.description,
        icon: manifest.icon ? resolveIcon(manifest.icon, dir) : undefined,
        tools: groupTools,
      });
    }
  }

  return { tools, groups };
}

/** 解析并注册全部插件（工具 + 分组，应用启动期调用一次） */
export function registerPlugins(): void {
  const { tools, groups } = resolvePlugins();
  tools.forEach(registerTool);
  groups.forEach(registerPluginGroup);
}

/** 执行所有插件的 setup 钩子（注册之后调用一次） */
export async function runPluginSetups(ctx: PluginContext): Promise<void> {
  for (const [path, setup] of Object.entries(setups)) {
    if (typeof setup !== 'function') continue;
    try {
      await setup(ctx);
    } catch (err) {
      console.error(`[plugins] setup 执行失败: ${pluginDirOf(path)}`, err);
    }
  }
}
