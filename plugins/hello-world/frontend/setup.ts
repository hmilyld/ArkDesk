/**
 * hello-world 生命周期钩子：应用启动时执行一次。
 *
 * ctx 提供框架能力（ctx.logger / ctx.db），此处仅作演示；
 * 无初始化需求的插件可省略本文件。
 */

import type { PluginContext } from '@/core/plugins';

export default (ctx: PluginContext): void => {
  ctx.logger.debug('插件已加载: hello-world（8 个工具项）');
};
