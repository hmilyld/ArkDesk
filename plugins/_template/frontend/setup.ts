/**
 * 可选：插件生命周期钩子。
 *
 * 默认导出的函数会在应用启动时执行一次（注册完成后）。
 * ctx 提供框架能力：ctx.logger（日志）、ctx.db（数据库）。
 * 无初始化需求的插件可删除本文件。
 */

import type { PluginContext } from '@/core/plugins';

export default (_ctx: PluginContext): void => {
  // 示例：ctx.logger.debug(`插件已加载: <plugin-id>`);
};
