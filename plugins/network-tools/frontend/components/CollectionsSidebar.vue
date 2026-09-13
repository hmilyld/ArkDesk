<!--
  左栏：集合树（任意层级 + 拖拽移动）与历史记录。
-->
<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import {
  ChevronDown,
  ChevronRight,
  Clock,
  Download,
  Eraser,
  Folder,
  FolderInput,
  FolderPlus,
  Pencil,
  Plus,
  Trash2,
  Upload,
} from '@lucide/vue';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { CollectionRow, RequestBriefRow } from '../composables/useCollections';
import type { HistoryRow } from '../composables/useHistory';
import { formatDuration, httpMethodClass } from '../shared';

const props = defineProps<{
  collections: CollectionRow[];
  requests: RequestBriefRow[];
  history: HistoryRow[];
  activeRequestId: number | null;
}>();

const emit = defineEmits<{
  'select-request': [id: number];
  'open-history': [row: HistoryRow];
  'new-request': [collectionId: number | null];
  'new-folder': [parentId: number | null];
  'rename-collection': [id: number, name: string];
  'rename-request': [id: number, name: string];
  'delete-collection': [id: number];
  'delete-request': [id: number];
  'move-collection': [id: number, parentId: number | null];
  'move-request': [id: number, collectionId: number | null];
  'import-bundle': [];
  'export-all': [];
  'export-request': [id: number];
  'export-collection': [id: number];
  'clear-history': [];
  'delete-history': [id: number];
}>();

interface TreeNode {
  kind: 'collection' | 'request';
  depth: number;
  collection?: CollectionRow;
  request?: RequestBriefRow;
}

const expanded = ref<Set<number>>(new Set());
const dragNode = ref<TreeNode | null>(null);
const editingId = ref<number | null>(null);
const editingKind = ref<'collection' | 'request' | null>(null);
const editingName = ref('');
const search = ref('');

function requestMatches(request: RequestBriefRow, query: string): boolean {
  return request.name.toLowerCase().includes(query) || request.url.toLowerCase().includes(query);
}

/** 搜索时：命中请求的祖先集合 + 名称命中的集合本身。空查询返回全部集合 */
const includedCollections = computed<Set<number> | null>(() => {
  const query = search.value.trim().toLowerCase();
  if (!query) return null;
  const included = new Set<number>();
  const includeAncestors = (id: number): void => {
    let current: number | null = id;
    while (current !== null) {
      included.add(current);
      current = props.collections.find((item) => item.id === current)?.parent_id ?? null;
    }
  };
  for (const request of props.requests) {
    if (request.collection_id !== null && requestMatches(request, query)) {
      includeAncestors(request.collection_id);
    }
  }
  for (const collection of props.collections) {
    if (collection.name.toLowerCase().includes(query)) includeAncestors(collection.id);
  }
  return included;
});

const tree = computed<TreeNode[]>(() => {
  const query = search.value.trim().toLowerCase();
  const included = includedCollections.value;
  const nodes: TreeNode[] = [];
  const childrenOf = (parentId: number | null) =>
    props.collections
      .filter((item) => item.parent_id === parentId)
      .sort((a, b) => a.sort_order - b.sort_order);
  const requestsOf = (collectionId: number | null) =>
    props.requests
      .filter((item) => item.collection_id === collectionId)
      .sort((a, b) => a.sort_order - b.sort_order);

  const walk = (parentId: number | null, depth: number): void => {
    for (const collection of childrenOf(parentId)) {
      if (included && !included.has(collection.id)) continue;
      nodes.push({ kind: 'collection', depth, collection });
      if (expanded.value.has(collection.id) || query) {
        walk(collection.id, depth + 1);
        for (const request of requestsOf(collection.id)) {
          if (query && !requestMatches(request, query)) continue;
          nodes.push({ kind: 'request', depth: depth + 1, request });
        }
      }
    }
  };
  walk(null, 0);
  for (const request of requestsOf(null)) {
    if (query && !requestMatches(request, query)) continue;
    nodes.push({ kind: 'request', depth: 0, request });
  }
  return nodes;
});

const filteredHistory = computed<HistoryRow[]>(() => {
  const query = search.value.trim().toLowerCase();
  if (!query) return props.history;
  return props.history.filter(
    (row) => row.url.toLowerCase().includes(query) || row.method.toLowerCase().includes(query)
  );
});

/** 扁平化的集合列表（含层级深度），用于「移动到」菜单 */
const flatCollections = computed(() => {
  const out: { id: number; name: string; depth: number }[] = [];
  const walk = (parentId: number | null, depth: number): void => {
    const children = props.collections
      .filter((item) => item.parent_id === parentId)
      .sort((a, b) => a.sort_order - b.sort_order);
    for (const child of children) {
      out.push({ id: child.id, name: child.name, depth });
      walk(child.id, depth + 1);
    }
  };
  walk(null, 0);
  return out;
});

function isExpanded(id: number): boolean {
  return expanded.value.has(id);
}
function toggle(id: number): void {
  const next = new Set(expanded.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  expanded.value = next;
}

// ── 指针拖拽（WebView 下比原生 HTML5 DnD 可靠） ──
const dragging = ref(false);
const dropKey = ref<string | null>(null);
let startX = 0;
let startY = 0;

function keyOf(node: TreeNode): string {
  if (node.collection) return `c-${node.collection.id}`;
  return `r-${node.request?.id}`;
}
/** 拖放到该行时，目标父级（集合 id 或 'root'） */
function dropParentOf(node: TreeNode): string {
  if (node.collection) return String(node.collection.id);
  const parent = node.request?.collection_id;
  return parent === null || parent === undefined ? 'root' : String(parent);
}
function currentParentOf(node: TreeNode): number | null {
  if (node.collection) return node.collection.parent_id;
  return node.request?.collection_id ?? null;
}

function onPointerDown(node: TreeNode, event: PointerEvent): void {
  if (editingId.value !== null || event.button !== 0) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest('button, input, textarea, [role="menu"]')) return;
  dragNode.value = node;
  dragging.value = false;
  dropKey.value = null;
  startX = event.clientX;
  startY = event.clientY;
  window.removeEventListener('pointermove', onPointerMove);
  window.removeEventListener('pointerup', onPointerUp);
  window.addEventListener('pointermove', onPointerMove);
  window.addEventListener('pointerup', onPointerUp);
}

function onPointerMove(event: PointerEvent): void {
  if (!dragNode.value) return;
  if (!dragging.value) {
    if (Math.abs(event.clientX - startX) + Math.abs(event.clientY - startY) < 6) return;
    dragging.value = true;
  }
  const element = document.elementFromPoint(event.clientX, event.clientY) as HTMLElement | null;
  dropKey.value = element?.closest<HTMLElement>('[data-drop-key]')?.dataset.dropKey ?? null;
  event.preventDefault();
}

function onPointerUp(event: PointerEvent): void {
  window.removeEventListener('pointermove', onPointerMove);
  window.removeEventListener('pointerup', onPointerUp);
  const node = dragNode.value;
  const wasDragging = dragging.value;
  dragNode.value = null;
  dragging.value = false;
  dropKey.value = null;
  if (!wasDragging || !node) return;

  const element = document.elementFromPoint(event.clientX, event.clientY) as HTMLElement | null;
  const row = element?.closest<HTMLElement>('[data-drop-parent]');
  if (!row || row.dataset.dropKey === keyOf(node)) return;
  const raw = row.dataset.dropParent ?? 'root';
  const parentId = raw === 'root' ? null : Number(raw);
  if (currentParentOf(node) === parentId) return; // 位置未变，忽略
  if (node.collection) emit('move-collection', node.collection.id, parentId);
  else if (node.request) emit('move-request', node.request.id, parentId);
}

onBeforeUnmount(() => {
  window.removeEventListener('pointermove', onPointerMove);
  window.removeEventListener('pointerup', onPointerUp);
});

function startRename(node: TreeNode): void {
  if (node.collection) {
    editingId.value = node.collection.id;
    editingName.value = node.collection.name;
    editingKind.value = 'collection';
  } else if (node.request) {
    editingId.value = node.request.id;
    editingName.value = node.request.name;
    editingKind.value = 'request';
  }
}
function commitRename(): void {
  if (editingId.value !== null && editingName.value.trim()) {
    if (editingKind.value === 'collection')
      emit('rename-collection', editingId.value, editingName.value.trim());
    else emit('rename-request', editingId.value, editingName.value.trim());
  }
  editingId.value = null;
  editingKind.value = null;
}
</script>

<template>
  <Tabs default-value="collections" class="flex h-full min-h-0 flex-col gap-2">
    <TabsList class="w-full">
      <TabsTrigger value="collections" class="flex-1">集合</TabsTrigger>
      <TabsTrigger value="history" class="flex-1">历史</TabsTrigger>
    </TabsList>

    <Input
      v-model="search"
      placeholder="搜索请求 / 集合 / 历史"
      class="h-7 text-xs"
      spellcheck="false"
    />

    <TabsContent value="collections" class="flex min-h-0 flex-1 flex-col gap-2">
      <div class="flex items-center gap-1">
        <Button variant="outline" size="xs" class="flex-1" @click="emit('new-folder', null)">
          <FolderPlus class="size-3.5" />
          文件夹
        </Button>
        <Button variant="outline" size="xs" class="flex-1" @click="emit('new-request', null)">
          <Plus class="size-3.5" />
          请求
        </Button>
      </div>
      <div class="flex items-center gap-1">
        <Button variant="ghost" size="xs" class="flex-1" @click="emit('import-bundle')">
          <Upload class="size-3.5" />
          导入
        </Button>
        <Button variant="ghost" size="xs" class="flex-1" @click="emit('export-all')">
          <Download class="size-3.5" />
          导出全部
        </Button>
      </div>
      <div
        class="min-h-0 flex-1 overflow-auto rounded-md border border-border/60"
        data-drop-key="root-area"
        data-drop-parent="root"
      >
        <div
          v-for="node in tree"
          :key="node.collection ? `c-${node.collection.id}` : `r-${node.request?.id}`"
          class="group flex items-center gap-1 rounded-sm px-1 py-0.5 select-none hover:bg-muted/60"
          :class="[
            node.request && node.request.id === activeRequestId ? 'bg-muted' : '',
            dragging && dropKey === keyOf(node) ? 'ring-1 ring-primary ring-inset' : '',
            dragging ? 'cursor-grabbing' : 'cursor-grab',
          ]"
          :data-drop-key="keyOf(node)"
          :data-drop-parent="dropParentOf(node)"
          :style="{ paddingLeft: `${node.depth * 14 + 4}px` }"
          @pointerdown="onPointerDown(node, $event)"
        >
          <template v-if="node.collection">
            <button
              type="button"
              class="flex size-4 shrink-0 items-center justify-center text-muted-foreground"
              @click="toggle(node.collection.id)"
            >
              <component
                :is="isExpanded(node.collection.id) ? ChevronDown : ChevronRight"
                class="size-3.5"
              />
            </button>
            <Folder class="size-3.5 shrink-0 text-muted-foreground" />
            <Input
              v-if="editingId === node.collection.id"
              v-model="editingName"
              class="h-6 flex-1 text-xs"
              @keydown.enter="commitRename"
              @keydown.esc="editingId = null"
              @blur="commitRename"
            />
            <span v-else class="min-w-0 flex-1 truncate text-xs">{{ node.collection.name }}</span>
          </template>
          <template v-else-if="node.request">
            <span
              class="w-9 shrink-0 text-right text-[10px] font-semibold"
              :class="httpMethodClass(node.request.method)"
            >
              {{ node.request.method }}
            </span>
            <Input
              v-if="editingId === node.request.id"
              v-model="editingName"
              class="h-6 flex-1 text-xs"
              @keydown.enter="commitRename"
              @keydown.esc="editingId = null"
              @blur="commitRename"
            />
            <span
              v-else
              class="min-w-0 flex-1 cursor-pointer truncate text-left text-xs"
              @click="emit('select-request', node.request.id)"
            >
              {{ node.request.name }}
            </span>
          </template>

          <span class="flex shrink-0 items-center opacity-0 group-hover:opacity-100">
            <template v-if="node.collection">
              <button
                type="button"
                title="新建请求"
                class="p-0.5 text-muted-foreground hover:text-foreground"
                @click="emit('new-request', node.collection.id)"
              >
                <Plus class="size-3.5" />
              </button>
            </template>
            <DropdownMenu>
              <DropdownMenuTrigger as-child>
                <button
                  type="button"
                  title="移动到文件夹"
                  class="p-0.5 text-muted-foreground hover:text-foreground"
                  @click.stop
                >
                  <FolderInput class="size-3.5" />
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" class="max-h-72 overflow-auto">
                <DropdownMenuLabel>移动到…</DropdownMenuLabel>
                <DropdownMenuItem
                  @select="
                    node.collection
                      ? emit('move-collection', node.collection.id, null)
                      : node.request && emit('move-request', node.request.id, null)
                  "
                >
                  根目录
                </DropdownMenuItem>
                <DropdownMenuItem
                  v-for="target in flatCollections"
                  :key="target.id"
                  :disabled="target.id === (node.collection?.id ?? node.request?.collection_id)"
                  @select="
                    node.collection
                      ? emit('move-collection', node.collection.id, target.id)
                      : node.request && emit('move-request', node.request.id, target.id)
                  "
                >
                  <span :style="{ paddingLeft: `${target.depth * 10}px` }">{{ target.name }}</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <button
              type="button"
              title="导出"
              class="p-0.5 text-muted-foreground hover:text-foreground"
              @click="
                node.collection
                  ? emit('export-collection', node.collection.id)
                  : node.request && emit('export-request', node.request.id)
              "
            >
              <Download class="size-3.5" />
            </button>
            <button
              type="button"
              title="重命名"
              class="p-0.5 text-muted-foreground hover:text-foreground"
              @click="startRename(node)"
            >
              <Pencil class="size-3.5" />
            </button>
            <button
              type="button"
              title="删除"
              class="p-0.5 text-muted-foreground hover:text-destructive"
              @click="
                node.collection
                  ? emit('delete-collection', node.collection.id)
                  : node.request && emit('delete-request', node.request.id)
              "
            >
              <Trash2 class="size-3.5" />
            </button>
          </span>
        </div>
        <p v-if="tree.length === 0" class="px-3 py-6 text-center text-xs text-muted-foreground">
          {{ search.trim() ? '无匹配结果' : '暂无集合，点击上方新建' }}
        </p>
      </div>
    </TabsContent>

    <TabsContent value="history" class="flex min-h-0 flex-1 flex-col gap-2">
      <div class="flex items-center justify-between">
        <span class="text-xs text-muted-foreground">{{ filteredHistory.length }} 条</span>
        <Button
          variant="ghost"
          size="xs"
          :disabled="history.length === 0"
          @click="emit('clear-history')"
        >
          <Eraser class="size-3.5" />
          清空
        </Button>
      </div>
      <div class="min-h-0 flex-1 overflow-auto rounded-md border border-border/60">
        <div
          v-for="row in filteredHistory"
          :key="row.id"
          class="group flex items-center gap-2 px-2 py-1 hover:bg-muted/60"
        >
          <Clock class="size-3.5 shrink-0 text-muted-foreground" />
          <span
            class="w-9 shrink-0 text-right text-[10px] font-semibold"
            :class="httpMethodClass(row.method)"
          >
            {{ row.method }}
          </span>
          <button
            type="button"
            class="min-w-0 flex-1 truncate text-left text-xs"
            @click="emit('open-history', row)"
          >
            {{ row.url }}
          </button>
          <span class="shrink-0 font-mono text-[10px] text-muted-foreground">
            {{ row.error ? 'ERR' : row.status }}
          </span>
          <span class="shrink-0 font-mono text-[10px] text-muted-foreground">
            {{ formatDuration(row.elapsed_ms) }}
          </span>
          <button
            type="button"
            class="shrink-0 p-0.5 text-muted-foreground opacity-0 hover:text-destructive group-hover:opacity-100"
            @click="emit('delete-history', row.id)"
          >
            <Trash2 class="size-3.5" />
          </button>
        </div>
        <p
          v-if="filteredHistory.length === 0"
          class="px-3 py-6 text-center text-xs text-muted-foreground"
        >
          暂无历史记录
        </p>
      </div>
    </TabsContent>
  </Tabs>
</template>
