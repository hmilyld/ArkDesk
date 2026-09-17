/**
 * hello-world 插件内共享代码。
 *
 * 多功能插件的核心优势之一：目录内的多个工具项复用同一份类型与常量，
 * 修改只需改这一处。各工具（Tool/TableTool/FormTool/Settings）从这里引用。
 */

/** 插件配置（设置页读写，经 useToolSettings 持久化） */
export interface HelloWorldConfig {
  /** 问候语模板，{name} 为占位符 */
  greetingTemplate: string;
}

/** 配置默认值（Rust 侧同名回退见 hello_world/mod.rs） */
export const HELLO_CONFIG_DEFAULTS: HelloWorldConfig = {
  greetingTemplate: '你好，{name}！PocketArk 插件链路已打通。',
};

/** 任务状态选项（hello_tasks.status） */
export const STATUS_OPTIONS = [
  { value: 'pending', label: '待处理' },
  { value: 'in_progress', label: '进行中' },
  { value: 'done', label: '已完成' },
] as const;

export type TaskStatus = (typeof STATUS_OPTIONS)[number]['value'];

/** 表格中状态对应的 Badge 样式 */
export const STATUS_BADGE_VARIANTS: Record<TaskStatus, 'default' | 'secondary' | 'outline'> = {
  pending: 'secondary',
  in_progress: 'default',
  done: 'outline',
};

/** 任务优先级中文标签（hello_tasks.priority） */
export const PRIORITY_LABELS: Record<string, string> = {
  high: '高',
  medium: '中',
  low: '低',
};

/** 分页每页条数（数据表格工具） */
export const PAGE_SIZE = 10;

/** 生成表格演示数据（title 随机拼接，status/priority 覆盖全部取值） */
export function buildSeedTasks(
  count: number
): { title: string; status: string; priority: string }[] {
  const topics = ['调研', '整理', '验证', '优化', '修复', '编写', '评审', '归档'];
  const objects = [
    '插件清单',
    '日志管道',
    '迁移脚本',
    '表格分页',
    '导入导出',
    '主题切换',
    '窗口状态',
    'HTTP 通道',
  ];
  const statuses: TaskStatus[] = ['pending', 'in_progress', 'done'];
  const priorities = ['high', 'medium', 'low'];

  return Array.from({ length: count }, (_, i) => ({
    title: `${topics[i % topics.length]}${objects[(i * 3) % objects.length]} #${i + 1}`,
    status: statuses[i % statuses.length],
    priority: priorities[(i >> 1) % priorities.length],
  }));
}
