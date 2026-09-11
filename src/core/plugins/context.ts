/**
 * 插件上下文：框架向工具插件暴露的受控能力集合。
 * 工具代码只通过 ctx 使用框架设施，不直接依赖底层实现。
 */
import { logger } from '@/core/logger';
import { db } from '@/core/db';

export interface PluginContext {
  logger: typeof logger;
  db: typeof db;
}

export function createPluginContext(): PluginContext {
  return { logger, db };
}
