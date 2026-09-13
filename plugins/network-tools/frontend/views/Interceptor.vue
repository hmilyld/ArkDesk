<!--
  请求拦截：本地 MITM 代理，捕获 / 查看 / 编辑 / 重放 HTTP(S) 流量。
-->
<script setup lang="ts">
import { computed, onActivated, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { Activity, Play, ShieldCheck, Square, Trash2 } from '@lucide/vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import ToolShell from '@/components/tool/ToolShell.vue';
import CaDialog from '../components/intercept/CaDialog.vue';
import FlowDetail from '../components/intercept/FlowDetail.vue';
import FlowList from '../components/intercept/FlowList.vue';
import PortDialog from '../components/intercept/PortDialog.vue';
import WsDetail from '../components/intercept/WsDetail.vue';
import { useInterceptor } from '../composables/useInterceptor';
import { errorMessage } from '../error';
import {
  EMPTY_FILTER,
  STATUS_OPTIONS,
  TYPE_OPTIONS,
  filterFlows,
  type FlowFilter,
  type FlowRecord,
  type ProxyConfig,
  type WsRecord,
} from '../intercept-shared';
import { HTTP_METHODS } from '../shared';
import { httpSettings as settings } from '../settings-store';

const {
  status,
  systemProxy,
  flows,
  wsRecords,
  loading,
  refreshAll,
  refreshSystemProxy,
  systemProxyEnable,
  systemProxyDisable,
  start,
  stop,
  configure,
  clear,
  getFlow,
  getWs,
  flowBodyTemp,
  exportCa,
  regenerateCa,
  subscribe,
  unsubscribe,
} = useInterceptor();

const sourceTab = ref<'http' | 'ws'>('http');
const filter = ref<FlowFilter>({ ...EMPTY_FILTER });
const selectedFlowId = ref<number | null>(null);
const selectedWsId = ref<number | null>(null);
const selectedFlow = ref<FlowRecord | null>(null);
const selectedWs = ref<WsRecord | null>(null);
const portDialogOpen = ref(false);
const caDialogOpen = ref(false);

// 最新发生的流量置顶
const filteredFlows = computed(() => filterFlows(flows.value, filter.value).slice().reverse());
const sortedWs = computed(() => [...wsRecords.value].reverse());
const running = computed(() => status.value?.running ?? false);

function currentConfig(): ProxyConfig {
  return {
    recordBodies: settings.value.interceptorRecordBodies,
    maxBodyKb: settings.value.interceptorMaxBodyKb,
    maxFlows: settings.value.interceptorMaxFlows,
    maxWsFrames: settings.value.interceptorMaxWsFrames,
  };
}

async function startWithPort(port: number): Promise<void> {
  try {
    await start(port, currentConfig());
    toast.success(`代理已启动`, { description: `127.0.0.1:${port}` });
  } catch (err) {
    toast.error(`启动失败：${errorMessage(err)}`);
    portDialogOpen.value = true;
    return;
  }
  if (settings.value.interceptorAutoSystemProxy && systemProxy.value?.supported !== false) {
    try {
      await systemProxyEnable(port);
      toast.info('已设置系统代理指向本地');
    } catch (err) {
      toast.error(`设置系统代理失败：${errorMessage(err)}`);
    }
  }
}

async function toggleSystemProxy(value: boolean | 'indeterminate'): Promise<void> {
  try {
    if (value === true && status.value?.port) {
      await systemProxyEnable(status.value.port);
      toast.success('系统代理已指向本地');
    } else {
      await systemProxyDisable();
      toast.info('系统代理已还原');
    }
  } catch (err) {
    toast.error(`系统代理操作失败：${errorMessage(err)}`);
  }
}

async function handleStart(): Promise<void> {
  const port = settings.value.interceptorPort;
  if (!port) {
    portDialogOpen.value = true;
    return;
  }
  await startWithPort(port);
}

async function handlePortConfirm(port: number): Promise<void> {
  settings.value.interceptorPort = port;
  await startWithPort(port);
}

async function handleStop(): Promise<void> {
  await stop();
  await refreshSystemProxy();
  toast.info('代理已停止');
}

async function handleClear(): Promise<void> {
  await clear();
  selectedFlow.value = null;
  selectedWs.value = null;
  selectedFlowId.value = null;
  selectedWsId.value = null;
}

async function selectFlow(id: number): Promise<void> {
  selectedWsId.value = null;
  selectedWs.value = null;
  selectedFlowId.value = id;
  try {
    selectedFlow.value = await getFlow(id);
  } catch (err) {
    selectedFlow.value = null;
    toast.error(errorMessage(err));
  }
}

async function selectWs(id: number): Promise<void> {
  selectedFlowId.value = null;
  selectedFlow.value = null;
  selectedWsId.value = id;
  try {
    selectedWs.value = await getWs(id);
  } catch (err) {
    selectedWs.value = null;
    toast.error(errorMessage(err));
  }
}

async function handleExportCa(): Promise<void> {
  const target = await saveFileDialog({ defaultPath: 'arkdesk-network-tools-ca.crt' });
  if (!target) return;
  try {
    await exportCa(target);
    toast.success('已导出证书', { description: target });
  } catch (err) {
    toast.error(`导出失败：${errorMessage(err)}`);
  }
}

async function handleRegenerateCa(): Promise<void> {
  try {
    await regenerateCa();
    toast.success('已重新生成根证书，请重新安装信任');
  } catch (err) {
    toast.error(`重新生成失败：${errorMessage(err)}`);
  }
}

function bodyTemp(id: number, side: 'req' | 'res'): Promise<string> {
  return flowBodyTemp(id, side);
}

watch(
  () => [
    settings.value.interceptorRecordBodies,
    settings.value.interceptorMaxBodyKb,
    settings.value.interceptorMaxFlows,
    settings.value.interceptorMaxWsFrames,
  ],
  () => {
    if (running.value) void configure(currentConfig());
  }
);

onMounted(async () => {
  await refreshAll();
  await subscribe();
});
onActivated(() => {
  void refreshAll();
});
onBeforeUnmount(() => {
  unsubscribe();
});
</script>

<template>
  <ToolShell title="请求拦截" description="本地 MITM 代理：捕获、查看、编辑并重放 HTTP(S) 请求">
    <template #actions>
      <Badge :variant="running ? 'default' : 'secondary'" class="gap-1">
        <Activity class="size-3" />
        {{ running ? `运行中 · ${status?.port}` : '未启动' }}
      </Badge>
      <div class="flex items-center gap-1.5" :title="systemProxy?.detail">
        <span class="text-xs text-muted-foreground">系统代理</span>
        <Switch
          :model-value="systemProxy?.enabled ?? false"
          :disabled="!systemProxy?.supported || loading || (!running && !systemProxy?.enabled)"
          @update:model-value="toggleSystemProxy"
        />
      </div>
      <Button v-if="!running" size="sm" :disabled="loading" @click="handleStart">
        <Play class="size-4" />
        启动
      </Button>
      <Button v-else variant="destructive" size="sm" :disabled="loading" @click="handleStop">
        <Square class="size-4" />
        停止
      </Button>
      <Button variant="outline" size="sm" @click="caDialogOpen = true">
        <ShieldCheck class="size-4" />
        根证书
      </Button>
      <Button
        variant="outline"
        size="sm"
        :disabled="flows.length === 0 && wsRecords.length === 0"
        @click="handleClear"
      >
        <Trash2 class="size-4" />
        清空
      </Button>
    </template>

    <div class="grid grid-cols-12 items-start gap-4">
      <section class="col-span-12 lg:col-span-5">
        <div
          class="flex h-[calc(100vh-11rem)] flex-col gap-2 rounded-lg border border-border bg-card p-3"
        >
          <Tabs v-model="sourceTab">
            <TabsList class="w-full">
              <TabsTrigger value="http" class="flex-1">HTTP ({{ flows.length }})</TabsTrigger>
              <TabsTrigger value="ws" class="flex-1"
                >WebSocket ({{ wsRecords.length }})</TabsTrigger
              >
            </TabsList>
          </Tabs>

          <div v-if="sourceTab === 'http'" class="grid grid-cols-2 gap-1.5">
            <Input
              v-model="filter.host"
              placeholder="Host 过滤"
              class="h-8 text-xs"
              spellcheck="false"
            />
            <Select v-model="filter.method">
              <SelectTrigger size="sm" class="text-xs">
                <SelectValue placeholder="方法" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__all__">全部方法</SelectItem>
                <SelectItem v-for="method in HTTP_METHODS" :key="method" :value="method">
                  {{ method }}
                </SelectItem>
              </SelectContent>
            </Select>
            <Input
              v-model="filter.url"
              placeholder="URL 关键字"
              class="h-8 text-xs"
              spellcheck="false"
            />
            <Select v-model="filter.status">
              <SelectTrigger size="sm" class="text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in STATUS_OPTIONS"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>
            <Select v-model="filter.type" class="col-span-2">
              <SelectTrigger size="sm" class="w-full text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem
                  v-for="option in TYPE_OPTIONS"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="min-h-0 flex-1 overflow-hidden rounded-md border border-border/60">
            <FlowList
              v-if="sourceTab === 'http'"
              :flows="filteredFlows"
              :selected-id="selectedFlowId"
              @select="selectFlow"
            />
            <div v-else class="h-full overflow-auto">
              <button
                v-for="item in sortedWs"
                :key="item.id"
                type="button"
                class="flex w-full items-center gap-2 border-b border-border/50 px-2 py-1.5 text-left hover:bg-muted/60"
                :class="item.id === selectedWsId ? 'bg-muted' : ''"
                @click="selectWs(item.id)"
              >
                <span class="min-w-0 flex-1 truncate text-xs font-mono">{{ item.url }}</span>
                <span class="shrink-0 text-[10px] text-muted-foreground"
                  >{{ item.frameCount }} 帧</span
                >
              </button>
              <p
                v-if="wsRecords.length === 0"
                class="px-3 py-8 text-center text-xs text-muted-foreground"
              >
                暂无 WebSocket 连接
              </p>
            </div>
          </div>
        </div>
      </section>

      <section class="col-span-12 lg:col-span-7">
        <div
          class="h-[calc(100vh-11rem)] overflow-hidden rounded-lg border border-border bg-card p-3"
        >
          <FlowDetail v-if="sourceTab === 'http'" :flow="selectedFlow" :flow-body-temp="bodyTemp" />
          <WsDetail v-else :record="selectedWs" />
        </div>
      </section>
    </div>

    <PortDialog
      v-model:open="portDialogOpen"
      :port="settings.interceptorPort || 8888"
      @confirm="handlePortConfirm"
    />
    <CaDialog
      v-model:open="caDialogOpen"
      :ca="status?.ca ?? null"
      @export="handleExportCa"
      @regenerate="handleRegenerateCa"
    />
  </ToolShell>
</template>
