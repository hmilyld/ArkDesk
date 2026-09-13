/**
 * 插件设置单例：发送页与设置页共用同一响应式配置，避免多处 useToolSettings 状态分叉。
 */

import { useToolSettings } from '@/core/plugins';
import { DEFAULT_SETTINGS, type HttpSettings } from './shared';

export const httpSettings = useToolSettings<HttpSettings>('network-tools', DEFAULT_SETTINGS);
