import { describe, expect, it } from 'vitest';
import { getAllTools, getToolById, registerTool } from '@/core/plugins/registry';
import type { ToolPlugin } from '@/core/plugins/types';

const icon = (() => ({})) as unknown as ToolPlugin['meta']['icon'];

function tool(id: string, name: string, group: string, order: number): ToolPlugin {
  return {
    meta: { id, name, group, order, pluginId: 'test', icon },
    component: async () => ({ default: {} as never }),
  };
}

describe('插件注册表', () => {
  it('注册后可按 id 取回', () => {
    registerTool(tool('reg-a', 'A', '组1', 10));
    expect(getToolById('reg-a')?.meta.name).toBe('A');
  });

  it('分组序取组内最小 order', () => {
    registerTool(tool('reg-b', 'B', '组2', 5));
    registerTool(tool('reg-c', 'C', '组1', 20));
    const ids = getAllTools().map((item) => item.meta.id);
    // 组2 最小 order=5 < 组1 最小 order=10 → 组2 的工具应排在前面
    expect(ids.indexOf('reg-b')).toBeLessThan(ids.indexOf('reg-c'));
  });

  it('重复 id 忽略后者', () => {
    registerTool(tool('reg-dup', '首次', '组1', 1));
    registerTool(tool('reg-dup', '再次', '组1', 1));
    expect(getToolById('reg-dup')?.meta.name).toBe('首次');
  });
});
