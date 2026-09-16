<!--
  流量详情：可编辑请求（复用 RequestTabs）+ 捕获响应 + 重放响应。
-->
<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { Send } from '@lucide/vue';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import RequestTabs from '../RequestTabs.vue';
import ResponsePanel from '../ResponsePanel.vue';
import { useSend } from '../../composables/useSend';
import { errorMessage } from '../../error';
import {
  flowToResponseView,
  flowToSpec,
  isBinaryRequest,
  type FlowRecord,
} from '../../intercept-shared';
import {
  buildSendOptions,
  createRequestSpec,
  statusBadgeVariant,
  uid,
  type HttpRequestSpec,
} from '../../shared';
import { httpSettings as settings } from '../../settings-store';

const props = defineProps<{
  flow: FlowRecord | null;
  flowBodyTemp: (id: number, side: 'req' | 'res') => Promise<string>;
}>();

const spec = ref<HttpRequestSpec>(createRequestSpec());
const activeTab = ref('request');
const replayWarnings = ref<string[]>([]);
const { loading, response, error, run, cancel } = useSend();

const capturedResponse = computed(() => (props.flow ? flowToResponseView(props.flow) : null));

let loadToken = 0;
watch(
  () => props.flow,
  async (flow) => {
    const token = ++loadToken;
    activeTab.value = 'request';
    replayWarnings.value = [];
    if (!flow) {
      spec.value = createRequestSpec();
      return;
    }
    const next = flowToSpec(flow);
    if (isBinaryRequest(flow)) {
      try {
        const path = await props.flowBodyTemp(flow.summary.id, 'req');
        next.body.type = 'binary';
        next.body.binaryPath = path;
        next.body.binaryContentType = flow.reqBody?.contentType ?? '';
      } catch (err) {
        if (token === loadToken) toast.error(`二进制请求体准备失败：${errorMessage(err)}`);
      }
    }
    if (token === loadToken) spec.value = next;
  },
  { immediate: true }
);

async function replay(): Promise<void> {
  if (!spec.value.url.trim()) {
    toast.error('URL 不能为空');
    return;
  }
  const taskId = uid();
  const built = buildSendOptions(spec.value, settings.value, taskId);
  replayWarnings.value = built.warnings;
  activeTab.value = 'replay';
  await run(built.options, taskId);
}
</script>

<template>
  <div v-if="!flow" class="flex h-full items-center justify-center text-xs text-muted-foreground">
    选择左侧一条流量查看详情
  </div>

  <div v-else class="flex h-full min-h-0 flex-col gap-3">
    <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
      <span class="text-xs font-semibold">{{ flow.summary.method }}</span>
      <Badge :variant="statusBadgeVariant(flow.summary.status)">
        {{ flow.summary.error ? 'ERR' : (flow.summary.status ?? '—') }}
      </Badge>
      <span class="truncate font-mono text-xs text-muted-foreground">{{ flow.summary.url }}</span>
      <span
        v-if="flow.summary.contentType"
        class="shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground"
      >
        {{ flow.summary.contentType }}
      </span>
      <span class="font-mono text-xs text-muted-foreground">
        {{ flow.summary.durationMs === null ? '' : `${flow.summary.durationMs}ms` }}
      </span>
      <span class="font-mono text-xs text-muted-foreground">{{ flow.clientAddr }}</span>
      <span v-if="flow.summary.error" class="text-xs text-destructive">{{
        flow.summary.error
      }}</span>
    </div>

    <Tabs v-model="activeTab" class="flex min-h-0 flex-1 flex-col gap-2">
      <div class="flex items-center justify-between gap-2">
        <TabsList>
          <TabsTrigger value="request">请求</TabsTrigger>
          <TabsTrigger value="captured">捕获响应</TabsTrigger>
          <TabsTrigger value="replay">重放响应</TabsTrigger>
        </TabsList>
        <Button v-if="!loading" size="sm" @click="replay">
          <Send class="size-4" />
          重放
        </Button>
        <Button v-else variant="destructive" size="sm" @click="cancel">停止</Button>
      </div>

      <TabsContent value="request" class="min-h-0 flex-1 overflow-auto">
        <RequestTabs v-model="spec" />
      </TabsContent>

      <TabsContent value="captured" class="min-h-0 flex-1">
        <ResponsePanel
          :response="capturedResponse"
          :loading="false"
          :error="''"
          :warnings="[]"
          fill
        />
      </TabsContent>

      <TabsContent value="replay" class="min-h-0 flex-1">
        <ResponsePanel
          :response="response"
          :loading="loading"
          :error="error"
          :warnings="replayWarnings"
          fill
        />
      </TabsContent>
    </Tabs>
  </div>
</template>
