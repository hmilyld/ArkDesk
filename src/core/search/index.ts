/**
 * 全局搜索（框架能力）：聚合导航与工具，做模糊匹配。供命令面板使用。
 */
import type { Router } from 'vue-router';
import Fuse from 'fuse.js';
import { getAllTools, getToolRoutePath } from '@/core/plugins';
import { useSettingsStore } from '@/stores/settings';

export interface SearchItem {
  id: string;
  title: string;
  subtitle?: string;
  group: string;
  run: (router: Router) => void | Promise<void>;
}

let items: SearchItem[] | null = null;
let fuse: Fuse<SearchItem> | null = null;

function buildItems(): SearchItem[] {
  const list: SearchItem[] = [
    { id: 'nav:home', title: '首页', group: '导航', run: (router) => void router.push('/') },
    {
      id: 'nav:settings',
      title: '设置',
      group: '导航',
      run: (router) => void router.push('/settings'),
    },
  ];
  const settings = useSettingsStore();
  for (const tool of getAllTools()) {
    if (!settings.isToolEnabled(tool.meta.id)) continue;
    list.push({
      id: `tool:${tool.meta.id}`,
      title: tool.meta.name,
      subtitle: tool.meta.description,
      group: tool.meta.group ?? '工具',
      run: (router) => void router.push(getToolRoutePath(tool.meta.id)),
    });
  }
  return list;
}

/** 索引在首次搜索时构建（此时插件已完成注册） */
function ensureIndex(): SearchItem[] {
  if (!items) {
    items = buildItems();
    fuse = new Fuse(items, {
      keys: ['title', 'subtitle', 'group'],
      threshold: 0.4,
      ignoreLocation: true,
    });
  }
  return items;
}

export function search(query: string): SearchItem[] {
  const all = ensureIndex();
  const q = query.trim();
  if (!q) return all.slice(0, 50);
  return (fuse?.search(q).map((result) => result.item) ?? []).slice(0, 50);
}
