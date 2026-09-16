/**
 * 请求拦截：代理状态、流量列表、WebSocket 记录与事件订阅。
 */

import { ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ipc } from '@/core/ipc';
import type {
  CaInfo,
  FlowRecord,
  FlowSummary,
  ProxyConfig,
  ProxyStatus,
  SystemProxyStatus,
  WsRecord,
  WsSummary,
} from '../intercept-shared';

export function useInterceptor() {
  const status = ref<ProxyStatus | null>(null);
  const systemProxy = ref<SystemProxyStatus | null>(null);
  const flows = ref<FlowSummary[]>([]);
  const wsRecords = ref<WsSummary[]>([]);
  const loading = ref(false);

  let unlistenFlow: UnlistenFn | null = null;
  let unlistenWs: UnlistenFn | null = null;

  async function refreshStatus(): Promise<void> {
    status.value = (await ipc<ProxyStatus | null>('network_tools_proxy_status')) ?? null;
  }

  async function refreshFlows(): Promise<void> {
    flows.value = (await ipc<FlowSummary[] | null>('network_tools_flows_list')) ?? [];
  }

  async function refreshWs(): Promise<void> {
    wsRecords.value = (await ipc<WsSummary[] | null>('network_tools_ws_list')) ?? [];
  }

  async function refreshSystemProxy(): Promise<void> {
    systemProxy.value =
      (await ipc<SystemProxyStatus | null>('network_tools_system_proxy_status')) ?? null;
  }

  async function systemProxyEnable(port: number): Promise<void> {
    systemProxy.value = await ipc<SystemProxyStatus>('network_tools_system_proxy_enable', { port });
  }

  async function systemProxyDisable(): Promise<void> {
    systemProxy.value = await ipc<SystemProxyStatus>('network_tools_system_proxy_disable');
  }

  async function refreshAll(): Promise<void> {
    await Promise.all([refreshStatus(), refreshFlows(), refreshWs(), refreshSystemProxy()]);
  }

  async function start(port: number, config: ProxyConfig): Promise<void> {
    loading.value = true;
    try {
      status.value = await ipc<ProxyStatus>('network_tools_proxy_start', { port, config });
    } finally {
      loading.value = false;
    }
  }

  async function stop(): Promise<void> {
    loading.value = true;
    try {
      status.value = await ipc<ProxyStatus>('network_tools_proxy_stop');
    } finally {
      loading.value = false;
    }
  }

  async function configure(config: ProxyConfig): Promise<void> {
    await ipc('network_tools_proxy_configure', { config });
  }

  async function clear(): Promise<void> {
    await ipc('network_tools_flows_clear');
    await ipc('network_tools_ws_clear');
    flows.value = [];
    wsRecords.value = [];
    await refreshStatus();
  }

  function getFlow(id: number): Promise<FlowRecord> {
    return ipc<FlowRecord>('network_tools_flow_get', { id });
  }

  function getWs(id: number): Promise<WsRecord> {
    return ipc<WsRecord>('network_tools_ws_get', { id });
  }

  function flowBodyTemp(id: number, side: 'req' | 'res'): Promise<string> {
    return ipc<string>('network_tools_flow_body_temp', { id, side });
  }

  function caInfo(): Promise<CaInfo> {
    return ipc<CaInfo>('network_tools_ca_info');
  }

  async function exportCa(path: string): Promise<void> {
    await ipc('network_tools_ca_export', { path });
  }

  async function regenerateCa(): Promise<CaInfo> {
    const info = await ipc<CaInfo>('network_tools_ca_regenerate');
    await refreshStatus();
    return info;
  }

  const FRONTEND_MAX = 2000;

  function upsert<T extends { id: number }>(list: T[], items: T[]): void {
    for (const item of items) {
      const index = list.findIndex((existing) => existing.id === item.id);
      if (index >= 0) list[index] = item;
      else list.push(item);
    }
    if (list.length > FRONTEND_MAX) list.splice(0, list.length - FRONTEND_MAX);
  }

  async function subscribe(): Promise<void> {
    unlistenFlow = await listen<FlowSummary[]>('network-tools://flow', (event) => {
      upsert(flows.value, event.payload);
    });
    unlistenWs = await listen<WsSummary[]>('network-tools://ws', (event) => {
      upsert(wsRecords.value, event.payload);
    });
  }

  function unsubscribe(): void {
    unlistenFlow?.();
    unlistenWs?.();
    unlistenFlow = null;
    unlistenWs = null;
  }

  return {
    status,
    systemProxy,
    flows,
    wsRecords,
    loading,
    refreshSystemProxy,
    systemProxyEnable,
    systemProxyDisable,
    refreshAll,
    start,
    stop,
    configure,
    clear,
    getFlow,
    getWs,
    flowBodyTemp,
    caInfo,
    exportCa,
    regenerateCa,
    subscribe,
    unsubscribe,
  };
}
