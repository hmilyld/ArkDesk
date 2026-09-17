<!--
  接口测试：单工作区。左栏集合/历史，右栏请求构建 + 响应查看。
-->
<script setup lang="ts">
import { computed, onActivated, onBeforeUnmount, onDeactivated, onMounted, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { Loader2, Save, Send, Square, Terminal } from '@lucide/vue';
import { ipc } from '@/core/ipc';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import ToolShell from '@/components/tool/ToolShell.vue';
import Panel from '@/components/tool/Panel.vue';
import ErrorState from '@/components/native/ErrorState.vue';
import CollectionsSidebar from '../components/CollectionsSidebar.vue';
import CurlDialog from '../components/CurlDialog.vue';
import RequestTabs from '../components/RequestTabs.vue';
import ResponsePanel from '../components/ResponsePanel.vue';
import { useCollections } from '../composables/useCollections';
import { useDraft } from '../composables/useDraft';
import { useEnvironments } from '../composables/useEnvironments';
import { useHistory, type HistoryRow } from '../composables/useHistory';
import { useSend } from '../composables/useSend';
import { buildCurl } from '../lib/curl';
import { errorMessage } from '../lib/error';
import {
  buildSendOptions,
  createRequestSpec,
  HTTP_METHODS,
  normalizeSpec,
  parseCookieHeader,
  uid,
  type HttpRequestSpec,
  type RequestMeta,
} from '../shared';
import { httpSettings as settings } from '../lib/settings-store';
import {
  createBundle,
  parseBundle,
  stringifyBundle,
  type Bundle,
  type BundleCollection,
  type BundleEnvironment,
  type BundleRequest,
} from '../lib/transfer';
import { collectVars, resolveSpec } from '../lib/variables';

const HISTORY_BODY_LIMIT = 256 * 1024;
const VAR_PLACEHOLDER = '{{name}}';

const spec = ref<HttpRequestSpec>(createRequestSpec());
const meta = ref<RequestMeta>({ id: null, name: '未命名请求', collectionId: null });
const warnings = ref<string[]>([]);
const curlOpen = ref(false);
/** 工具页内可定位的校验/操作错误（就地错误条，不用 toast） */
const formError = ref('');
/** 保存成功后的短暂状态提示（§4：成功用状态变化表达，不弹 toast） */
const saveFlash = ref(false);
let flashTimer: ReturnType<typeof setTimeout> | null = null;

const {
  collections,
  requests,
  refresh: refreshCollections,
  createCollection,
  renameCollection,
  moveCollection,
  deleteCollection,
  saveRequest,
  loadRequest,
  renameRequest,
  moveRequest,
  deleteRequest,
} = useCollections();

const {
  environments,
  refresh: refreshEnvironments,
  varsOf,
  createEnvironment,
  addVar,
} = useEnvironments();
const {
  items: historyItems,
  refresh: refreshHistory,
  record: recordHistory,
  prune: pruneHistory,
  remove: removeHistory,
  clear: clearHistory,
} = useHistory();
const { save: saveDraft, load: loadDraft } = useDraft();
const {
  loading: sending,
  response,
  error: sendError,
  run: runSend,
  cancel: cancelSend,
} = useSend();

const activeEnvValue = computed({
  get: () =>
    settings.value.activeEnvironmentId ? String(settings.value.activeEnvironmentId) : 'none',
  set: (value: string) =>
    (settings.value.activeEnvironmentId = value === 'none' ? null : Number(value)),
});
const exportCurl = computed(() => buildCurl(spec.value));

let draftTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  [spec, meta],
  () => {
    if (draftTimer) clearTimeout(draftTimer);
    draftTimer = setTimeout(() => void saveDraft(spec.value, meta.value), 600);
  },
  { deep: true }
);

async function refreshAll(): Promise<void> {
  await Promise.all([refreshCollections(), refreshEnvironments(), refreshHistory()]);
}

function flushDraft(): void {
  if (draftTimer) clearTimeout(draftTimer);
  void saveDraft(spec.value, meta.value);
}

let activatedOnce = false;
onMounted(async () => {
  await refreshAll();
  const restored = await loadDraft();
  if (restored) {
    spec.value = normalizeSpec(restored.spec);
    meta.value = restored.meta;
  }
});
// KeepAlive 重新激活时刷新数据（设置页可能已改环境/集合）；首次激活由 onMounted 处理
onActivated(async () => {
  if (!activatedOnce) {
    activatedOnce = true;
    return;
  }
  await refreshAll();
});
onDeactivated(flushDraft);
onBeforeUnmount(flushDraft);
onBeforeUnmount(() => {
  if (flashTimer) clearTimeout(flashTimer);
});

async function handleSend(): Promise<void> {
  if (sending.value) return;
  formError.value = '';
  const activeVars = collectVars(
    varsOf(settings.value.activeEnvironmentId).map((item) => ({
      key: item.key,
      value: item.value,
      enabled: item.enabled,
    }))
  );
  const { spec: resolved, missing } = resolveSpec(spec.value, activeVars);
  if (missing.length) {
    formError.value = `存在未解析变量：${missing.join('、')}`;
    return;
  }
  if (!resolved.url.trim()) {
    formError.value = '请填写请求 URL';
    return;
  }

  const taskId = uid();
  const built = buildSendOptions(resolved, settings.value, taskId);
  warnings.value = built.warnings;
  const result = await runSend(built.options, taskId);

  if (settings.value.historyEnabled) {
    const bodyForHistory =
      result && !result.isBinary && (result.bodyText?.length ?? 0) <= HISTORY_BODY_LIMIT
        ? (result.bodyText ?? null)
        : null;
    await recordHistory({
      method: resolved.method,
      url: resolved.url,
      request: JSON.stringify(resolved),
      status: result?.status ?? null,
      ok: result?.ok ?? null,
      elapsedMs: result?.elapsedMs ?? null,
      sizeBytes: result?.sizeBytes ?? null,
      responseHeaders: JSON.stringify(result?.headers ?? []),
      responseBody: bodyForHistory,
      bodyTruncated: result?.truncated ?? false,
      error: sendError.value || null,
      environmentId: settings.value.activeEnvironmentId,
    });
    await pruneHistory(settings.value.historyLimit);
    await refreshHistory(settings.value.historyLimit);
  }
}

async function handleNewRequest(collectionId: number | null): Promise<void> {
  const id = await saveRequest(createRequestSpec(), {
    id: null,
    name: '新建请求',
    collectionId,
  });
  const loaded = await loadRequest(id);
  if (loaded) {
    spec.value = loaded;
    meta.value = { id, name: '新建请求', collectionId };
  }
}

async function handleNewFolder(parentId: number | null): Promise<void> {
  await createCollection('新建文件夹', parentId, 'folder');
}

async function handleSelectRequest(id: number): Promise<void> {
  const loaded = await loadRequest(id);
  const brief = requests.value.find((item) => item.id === id);
  if (loaded) {
    spec.value = loaded;
    meta.value = {
      id,
      name: brief?.name ?? '请求',
      collectionId: brief?.collection_id ?? null,
    };
  }
}

async function handleSave(): Promise<void> {
  if (!meta.value.name.trim()) {
    formError.value = '请填写请求名称';
    return;
  }
  formError.value = '';
  const id = await saveRequest(spec.value, meta.value);
  meta.value = { ...meta.value, id };
  saveFlash.value = true;
  if (flashTimer) clearTimeout(flashTimer);
  flashTimer = setTimeout(() => (saveFlash.value = false), 2000);
}

async function handleRenameCollection(id: number, name: string): Promise<void> {
  await renameCollection(id, name);
}
async function handleRenameRequest(id: number, name: string): Promise<void> {
  await renameRequest(id, name);
  if (meta.value.id === id) meta.value = { ...meta.value, name };
}
async function handleDeleteCollection(id: number): Promise<void> {
  await deleteCollection(id);
}
async function handleDeleteRequest(id: number): Promise<void> {
  await deleteRequest(id);
  if (meta.value.id === id) meta.value = { ...meta.value, id: null };
}
function isDescendant(ancestor: number, candidate: number | null): boolean {
  let current = candidate;
  while (current !== null) {
    if (current === ancestor) return true;
    current = collections.value.find((item) => item.id === current)?.parent_id ?? null;
  }
  return false;
}
async function handleMoveCollection(id: number, parentId: number | null): Promise<void> {
  if (parentId !== null && (parentId === id || isDescendant(id, parentId))) {
    toast.error('不能移动到自身或其子级');
    return;
  }
  await moveCollection(id, parentId);
}
async function handleMoveRequest(id: number, collectionId: number | null): Promise<void> {
  await moveRequest(id, collectionId);
  if (meta.value.id === id) meta.value = { ...meta.value, collectionId };
}

async function handleOpenHistory(row: HistoryRow): Promise<void> {
  try {
    spec.value = normalizeSpec(JSON.parse(row.request));
    meta.value = { id: null, name: `${row.method} ${row.url}（历史）`, collectionId: null };
    formError.value = '';
  } catch {
    formError.value = '历史请求解析失败，无法载入';
  }
}
async function handleDeleteHistory(id: number): Promise<void> {
  await removeHistory(id);
  await refreshHistory(settings.value.historyLimit);
}
async function handleClearHistory(): Promise<void> {
  await clearHistory();
  await refreshHistory(settings.value.historyLimit);
}

function handleImportCurl(imported: HttpRequestSpec): void {
  spec.value = imported;
  meta.value = { ...meta.value, id: null };
}
function handleApplyCookie(cookieHeader: string): void {
  spec.value = {
    ...spec.value,
    cookies: [...spec.value.cookies, ...parseCookieHeader(cookieHeader)],
  };
}

// ── 自有 JSON 导入导出 ──

async function exportBundleFile(bundle: Bundle, defaultName: string): Promise<void> {
  const target = await saveFileDialog({ defaultPath: defaultName });
  if (!target) return;
  try {
    await ipc('file_write_text', { path: target, contents: stringifyBundle(bundle) });
  } catch (err) {
    toast.error(`导出失败：${errorMessage(err)}`);
  }
}

async function buildCollectionBundle(collectionId: number): Promise<BundleCollection> {
  const node = collections.value.find((item) => item.id === collectionId);
  if (!node) return { name: '未命名集合', requests: [], children: [] };
  const items: BundleRequest[] = [];
  for (const brief of requests.value.filter((item) => item.collection_id === node.id)) {
    const loaded = await loadRequest(brief.id);
    if (loaded) items.push({ name: brief.name, spec: loaded });
  }
  const children: BundleCollection[] = [];
  for (const child of collections.value.filter((item) => item.parent_id === node.id)) {
    children.push(await buildCollectionBundle(child.id));
  }
  return { name: node.name, requests: items, children };
}

async function handleExportRequest(id: number): Promise<void> {
  const brief = requests.value.find((item) => item.id === id);
  const loaded = await loadRequest(id);
  if (!brief || !loaded) return;
  await exportBundleFile(
    createBundle({ requests: [{ name: brief.name, spec: loaded }] }),
    `${brief.name}.json`
  );
}

async function handleExportCollection(id: number): Promise<void> {
  const bundle = await buildCollectionBundle(id);
  await exportBundleFile(createBundle({ collections: [bundle] }), `${bundle.name}.json`);
}

async function handleExportAll(): Promise<void> {
  const rootRequests: BundleRequest[] = [];
  for (const brief of requests.value.filter((item) => item.collection_id === null)) {
    const loaded = await loadRequest(brief.id);
    if (loaded) rootRequests.push({ name: brief.name, spec: loaded });
  }
  const rootCollections: BundleCollection[] = [];
  for (const collection of collections.value.filter((item) => item.parent_id === null)) {
    rootCollections.push(await buildCollectionBundle(collection.id));
  }
  const envs: BundleEnvironment[] = environments.value.map((env) => ({
    name: env.name,
    vars: varsOf(env.id).map((row) => ({
      key: row.key,
      value: row.value,
      enabled: row.enabled,
      isSecret: row.is_secret,
    })),
  }));
  await exportBundleFile(
    createBundle({ requests: rootRequests, collections: rootCollections, environments: envs }),
    'network-tools-export.json'
  );
}

async function handleImportBundle(): Promise<void> {
  const selected = await openFileDialog({
    multiple: false,
    filters: [{ name: 'JSON', extensions: ['json'] }],
  });
  if (typeof selected !== 'string') return;
  let bundle: Bundle;
  try {
    bundle = parseBundle(await ipc<string>('file_read_text', { path: selected }));
  } catch (err) {
    formError.value = `导入失败：${errorMessage(err)}`;
    return;
  }
  try {
    for (const request of bundle.requests ?? []) {
      await saveRequest(normalizeSpec(request.spec), {
        id: null,
        name: request.name,
        collectionId: null,
      });
    }
    const insertCollection = async (
      node: BundleCollection,
      parentId: number | null
    ): Promise<void> => {
      const createdId = await createCollection(node.name, parentId, 'folder');
      for (const request of node.requests) {
        await saveRequest(normalizeSpec(request.spec), {
          id: null,
          name: request.name,
          collectionId: createdId,
        });
      }
      for (const child of node.children) await insertCollection(child, createdId);
    };
    for (const collection of bundle.collections ?? []) await insertCollection(collection, null);
    for (const env of bundle.environments ?? []) {
      const envId = await createEnvironment(env.name);
      for (const row of env.vars) {
        await addVar(envId, {
          key: row.key,
          value: row.value,
          enabled: row.enabled,
          is_secret: row.isSecret,
        });
      }
    }
    await Promise.all([refreshCollections(), refreshEnvironments()]);
  } catch (err) {
    toast.error(`导入失败：${errorMessage(err)}`);
  }
}
</script>

<template>
  <ToolShell
    title="接口测试"
    description="模拟 GET/POST 等请求，自定义 Headers / Cookie / Body 并查看响应"
  >
    <template #actions>
      <Button variant="outline" size="sm" @click="curlOpen = true">
        <Terminal class="mr-1 size-3.5" />
        cURL
      </Button>
    </template>

    <div class="grid grid-cols-12 items-start gap-4">
      <aside class="col-span-12 lg:col-span-3">
        <Panel
          title="集合与历史"
          class="flex h-[70vh] flex-col lg:sticky lg:top-28 lg:h-[calc(100vh-9.5rem)]"
          body-class="min-h-0 flex-1 space-y-0 p-3"
        >
          <CollectionsSidebar
            :collections="collections"
            :requests="requests"
            :history="historyItems"
            :active-request-id="meta.id"
            @select-request="handleSelectRequest"
            @open-history="handleOpenHistory"
            @new-request="handleNewRequest"
            @new-folder="handleNewFolder"
            @rename-collection="handleRenameCollection"
            @rename-request="handleRenameRequest"
            @delete-collection="handleDeleteCollection"
            @delete-request="handleDeleteRequest"
            @move-collection="handleMoveCollection"
            @move-request="handleMoveRequest"
            @import-bundle="handleImportBundle"
            @export-all="handleExportAll"
            @export-request="handleExportRequest"
            @export-collection="handleExportCollection"
            @clear-history="handleClearHistory"
            @delete-history="handleDeleteHistory"
          />
        </Panel>
      </aside>

      <section class="col-span-12 flex flex-col gap-4 lg:col-span-9 lg:h-[calc(100vh-9.5rem)]">
        <Panel
          title="请求"
          :hint="saveFlash ? '已保存' : undefined"
          class="flex max-h-[60%] shrink-0 flex-col"
          body-class="flex min-h-0 flex-1 flex-col gap-3 space-y-0 overflow-auto p-4"
        >
          <div class="flex flex-wrap items-center gap-2">
            <Input v-model="meta.name" placeholder="请求名称" class="h-8 w-40" spellcheck="false" />
            <Select v-model="spec.method">
              <SelectTrigger size="sm" class="w-28">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="method in HTTP_METHODS" :key="method" :value="method">
                  {{ method }}
                </SelectItem>
              </SelectContent>
            </Select>
            <Input
              v-model="spec.url"
              placeholder="https://api.example.com/{{id}}"
              class="h-8 min-w-48 flex-1 font-mono text-xs"
              spellcheck="false"
              @keydown.enter="handleSend"
            />
            <Button v-if="!sending" size="sm" :disabled="sending" @click="handleSend">
              <Send class="mr-1 size-3.5" />
              发送
            </Button>
            <Button v-else variant="destructive" size="sm" @click="cancelSend">
              <Square class="mr-1 size-3.5" />
              停止
            </Button>
            <Button variant="outline" size="sm" @click="handleSave">
              <Save class="mr-1 size-3.5" />
              保存
            </Button>
            <Loader2 v-if="sending" class="size-4 animate-spin text-muted-foreground" />
          </div>

          <ErrorState v-if="formError" :message="formError" />

          <div v-if="environments.length > 0" class="flex items-center gap-2">
            <span class="text-xs text-muted-foreground">环境</span>
            <Select v-model="activeEnvValue">
              <SelectTrigger size="sm" class="w-48">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="none">无环境</SelectItem>
                <SelectItem v-for="env in environments" :key="env.id" :value="String(env.id)">
                  {{ env.name }}
                </SelectItem>
              </SelectContent>
            </Select>
            <span class="text-xs text-muted-foreground">变量占位符：{{ VAR_PLACEHOLDER }}</span>
          </div>
          <p v-else class="text-xs text-muted-foreground">
            未配置环境变量；可在「设置」中添加，请求中用 {{ VAR_PLACEHOLDER }} 引用
          </p>

          <RequestTabs v-model="spec" />
        </Panel>

        <Panel
          title="响应"
          class="flex min-h-0 flex-1 flex-col"
          body-class="flex min-h-0 flex-1 flex-col space-y-0 p-4"
        >
          <ResponsePanel
            :response="response"
            :loading="sending"
            :error="sendError"
            :warnings="warnings"
            fill
            @apply-cookie="handleApplyCookie"
          />
        </Panel>
      </section>
    </div>

    <CurlDialog v-model:open="curlOpen" :export-text="exportCurl" @import="handleImportCurl" />
  </ToolShell>
</template>
