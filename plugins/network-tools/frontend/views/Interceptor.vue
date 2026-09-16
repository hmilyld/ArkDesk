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
import Panel from '@/components/tool/Panel.vue';
import EmptyState from '@/components/native/EmptyState.vue';
import ErrorState from '@/components/native/ErrorState.vue';
import SearchField from '@/components/native/SearchField.vue';
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
/** 启动代理失败的原地错误（列表面板内错误条） */
const startError = ref('');
/** 载入流量/WS 详情失败的原地错误（详情面板内错误条） */
const detailError = ref('');

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
  startError.value = '';
  try {
    await start(port, currentConfig());
  } catch (err) {
    startError.value = `启动失败：${errorMessage(err)}`;
    portDialogOpen.value = true;
    return;
  }
  if (settings.value.interceptorAutoSystemProxy && systemProxy.value?.supported !== false) {
    try {
      await systemProxyEnable(port);
    } catch (err) {
      toast.error(`设置系统代理失败：${errorMessage(err)}`);
    }
  }
}

async function toggleSystemProxy(value: boolean | 'indeterminate'): Promise<void> {
  try {
    if (value === true && status.value?.port) {
      await systemProxyEnable(status.value.port);
    } else {
      await systemProxyDisable();
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
  detailError.value = '';
  try {
    selectedFlow.value = await getFlow(id);
  } catch (err) {
    selectedFlow.value = null;
    detailError.value = errorMessage(err);
  }
}

async function selectWs(id: number): Promise<void> {
  selectedFlowId.value = null;
  selectedFlow.value = null;
  selectedWsId.value = id;
  detailError.value = '';
  try {
    selectedWs.value = await getWs(id);
  } catch (err) {
    selectedWs.value = null;
    detailError.value = errorMessage(err);
  }
}

async function handleExportCa(): Promise<void> {
  const target = await saveFileDialog({ defaultPath: 'arkdesk-network-tools-ca.crt' });
  if (!target) return;
  try {
    await exportCa(target);
  } catch (err) {
    toast.error(`导出失败：${errorMessage(err)}`);
  }
}

async function handleRegenerateCa(): Promise<void> {
  try {
    await regenerateCa();
    toast.info('已重新生成根证书，请重新安装信任');
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
        <Activity class="size-3.5" />
        {{ running ? `运行中 · ${status?.port}` : '未启动' }}
      </Badge>
      <div class="flex items-center gap-1.5" :title="systemProxy?.detail">
        <span class="text-xs text-muted-foreground">系统代理</span>
        <Switch
          size="sm"
          :model-value="systemProxy?.enabled ?? false"
          :disabled="!systemProxy?.supported || loading || (!running && !systemProxy?.enabled)"
          @update:model-value="toggleSystemProxy"
        />
      </div>
      <Button v-if="!running" size="sm" :disabled="loading" @click="handleStart">
        <Play class="mr-1 size-3.5" />
        启动
      </Button>
      <Button v-else variant="destructive" size="sm" :disabled="loading" @click="handleStop">
        <Square class="mr-1 size-3.5" />
        停止
      </Button>
      <Button variant="outline" size="sm" @click="caDialogOpen = true">
        <ShieldCheck class="mr-1 size-3.5" />
        根证书
      </Button>
      <Button
        variant="outline"
        size="sm"
        :disabled="flows.length === 0 && wsRecords.length === 0"
        @click="handleClear"
      >
        <Trash2 class="mr-1 size-3.5" />
        清空
      </Button>
    </template>

    <div class="grid grid-cols-12 items-start gap-4">
      <section class="col-span-12 lg:col-span-5">
        <Panel
          title="流量"
          class="flex h-[calc(100vh-11rem)] flex-col"
          body-class="flex min-h-0 flex-1 flex-col space-y-0 p-0"
        >
          <div class="space-y-2 p-3">
            <Tabs v-model="sourceTab">
              <TabsList class="w-full">
                <TabsTrigger value="http" class="flex-1">HTTP ({{ flows.length }})</TabsTrigger>
                <TabsTrigger value="ws" class="flex-1"
                  >WebSocket ({{ wsRecords.length }})</TabsTrigger
                >
              </TabsList>
            </Tabs>

            <ErrorState v-if="startError" :message="startError" />

            <div v-if="sourceTab === 'http'" class="grid grid-cols-2 gap-1.5">
              <SearchField v-model="filter.host" placeholder="Host 过滤" spellcheck="false" />
              <Select v-model="filter.method">
                <SelectTrigger size="sm" class="w-full text-xs">
                  <SelectValue placeholder="方法" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__all__">全部方法</SelectItem>
                  <SelectItem v-for="method in HTTP_METHODS" :key="method" :value="method">
                    {{ method }}
                  </SelectItem>
                </SelectContent>
              </Select>
              <SearchField v-model="filter.url" placeholder="URL 关键字" spellcheck="false" />
              <Select v-model="filter.status">
                <SelectTrigger size="sm" class="w-full text-xs">
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
          </div>

          <div class="min-h-0 flex-1 overflow-hidden border-t">
            <FlowList
              v-if="sourceTab === 'http'"
              :flows="filteredFlows"
              :selected-id="selectedFlowId"
              @select="selectFlow"
            />
            <div v-else class="flex h-full flex-col divide-y overflow-auto">
              <button
                v-for="item in sortedWs"
                :key="item.id"
                type="button"
                class="flex w-full items-center gap-2 px-3 py-1.5 text-left transition-colors hover:bg-accent focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/60 focus-visible:ring-inset outline-none"
                :class="item.id === selectedWsId ? 'bg-primary/10' : ''"
                :aria-current="item.id === selectedWsId ? 'true' : undefined"
                @click="selectWs(item.id)"
              >
                <span class="min-w-0 flex-1 truncate font-mono text-xs">{{ item.url }}</span>
                <span class="shrink-0 text-xs text-muted-foreground">{{ item.frameCount }} 帧</span>
              </button>
              <EmptyState
                v-if="wsRecords.length === 0"
                class="my-auto"
                :icon="Activity"
                title="暂无 WebSocket 连接"
                description="启动代理并让应用走系统代理后，连接会显示在这里"
              />
            </div>
          </div>
        </Panel>
      </section>

      <section class="col-span-12 lg:col-span-7">
        <Panel
          title="详情"
          class="flex h-[calc(100vh-11rem)] flex-col"
          body-class="flex min-h-0 flex-1 flex-col gap-2 space-y-0 p-3"
        >
          <ErrorState v-if="detailError" :message="detailError" />
          <div class="min-h-0 flex-1">
            <FlowDetail
              v-if="sourceTab === 'http'"
              :flow="selectedFlow"
              :flow-body-temp="bodyTemp"
            />
            <WsDetail v-else :record="selectedWs" />
          </div>
        </Panel>
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
