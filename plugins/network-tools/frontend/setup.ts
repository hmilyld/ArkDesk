/**
 * 网络工具启动自检。
 *
 * 代理监听随进程退出而消失，但「一键系统代理」写入的是操作系统设置，进程被强杀时
 * 可能残留并指向已失效的本地端口。启动时检测：系统代理处于托管态但代理未运行 →
 * 自动还原，避免用户网络被指向死端口。
 */

import { ipc } from '@/core/ipc';
import { logger } from '@/core/logger';

interface ProxyStatus {
  running: boolean;
}

interface SystemProxyStatus {
  supported: boolean;
  enabled: boolean;
}

export default async function setup(): Promise<void> {
  try {
    const systemProxy = await ipc<SystemProxyStatus>('network_tools_system_proxy_status');
    if (!systemProxy.enabled) return;

    const status = await ipc<ProxyStatus>('network_tools_proxy_status');
    if (status.running) return;

    await ipc('network_tools_system_proxy_disable');
    logger.warn('检测到上次会话遗留的系统代理指向本地拦截端口，已自动还原');
  } catch (err) {
    logger.debug(`网络工具启动自检跳过: ${String(err)}`);
  }
}
