/**
 * tender-optimizer 生命周期钩子：应用启动时执行一次。
 *
 * 评分参数行由迁移（INSERT OR IGNORE id=1）保证存在，
 * 此处仅记录加载日志。
 */

import type { PluginContext } from '@/core/plugins';

export default (ctx: PluginContext): void => {
  ctx.logger.debug('插件已加载: tender-optimizer（报价测算 / 测算历史）');
};
