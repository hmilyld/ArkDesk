<!--
  草稿箱：保存手动/AI 文章，多选后批量生成视频。
-->
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ipc } from '@/core/ipc';
import { notify } from '@/core/notify';
import { Plus, RefreshCw, Trash2, Pencil, Play, Square, Film, FolderOpen } from '@lucide/vue';
import ToolShell from '@/components/tool/ToolShell.vue';
import ArticleForm from '../components/ArticleForm.vue';
import { useGeneration } from '../composables/useGeneration';
import {
  normalizeSettings,
  settings,
  statusLabel,
  STAGE_LABELS,
  type ArticleValue,
  type Draft,
  type RunSummary,
} from '../shared';

const gen = useGeneration();
const { running, progress, logs, summary, progressPercent, begin, finish, cancel, openPath } = gen;

const drafts = ref<Draft[]>([]);
const selected = ref<number[]>([]);
const loading = ref(false);
const deleteAfterGenerate = ref(false);

const editorOpen = ref(false);
const editor = ref<{ id?: number; title: string; author: string; content: string; source: string }>(
  {
    title: '',
    author: '',
    content: '',
    source: 'manual',
  }
);
const saving = ref(false);

/** 编辑对话框表单：与生成页共用 ArticleForm */
const editorForm = computed<ArticleValue>({
  get: () => ({
    title: editor.value.title,
    author: editor.value.author,
    content: editor.value.content,
  }),
  set: (value) => {
    editor.value = { ...editor.value, ...value };
  },
});

const selectedCount = computed(() => selected.value.length);
const allSelected = computed(
  () => drafts.value.length > 0 && selectedCount.value === drafts.value.length
);

async function load(): Promise<void> {
  loading.value = true;
  try {
    drafts.value = await ipc<Draft[]>('text2video_draft_list');
    selected.value = selected.value.filter((id) => drafts.value.some((d) => d.id === id));
  } catch (err) {
    toast.error(`加载草稿失败: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    loading.value = false;
  }
}

function isSelected(id: number): boolean {
  return selected.value.includes(id);
}

function toggleSelect(id: number, value: boolean | 'indeterminate'): void {
  if (value === true && !selected.value.includes(id)) {
    selected.value = [...selected.value, id];
  } else if (value !== true) {
    selected.value = selected.value.filter((x) => x !== id);
  }
}

function toggleAll(value: boolean | 'indeterminate'): void {
  selected.value = value === true ? drafts.value.map((d) => d.id) : [];
}

function openNew(): void {
  editor.value = { title: '', author: '', content: '', source: 'manual' };
  editorOpen.value = true;
}

function openEdit(draft: Draft): void {
  editor.value = {
    id: draft.id,
    title: draft.title,
    author: draft.author,
    content: draft.content,
    source: draft.source,
  };
  editorOpen.value = true;
}

async function saveEditor(): Promise<void> {
  if (!editor.value.title.trim() || !editor.value.content.trim()) {
    toast.error('请填写标题与正文');
    return;
  }
  saving.value = true;
  try {
    await ipc('text2video_draft_save', {
      draft: {
        id: editor.value.id,
        title: editor.value.title.trim(),
        author: editor.value.author.trim(),
        content: editor.value.content,
        source: editor.value.source,
      },
    });
    editorOpen.value = false;
    await load();
    toast.success('草稿已保存');
  } catch (err) {
    toast.error(`保存草稿失败: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    saving.value = false;
  }
}

async function remove(id: number): Promise<void> {
  try {
    await ipc('text2video_draft_delete', { id });
    selected.value = selected.value.filter((x) => x !== id);
    await load();
    toast.success('草稿已删除');
  } catch (err) {
    toast.error(`删除草稿失败: ${err instanceof Error ? err.message : String(err)}`);
  }
}

async function removeSelected(): Promise<void> {
  if (!selectedCount.value) return;
  try {
    for (const id of selected.value) {
      await ipc('text2video_draft_delete', { id });
    }
    selected.value = [];
    await load();
    toast.success('所选草稿已删除');
  } catch (err) {
    toast.error(`删除草稿失败: ${err instanceof Error ? err.message : String(err)}`);
  }
}

async function batchGenerate(): Promise<void> {
  if (running.value) return;
  const items = drafts.value.filter((d) => selected.value.includes(d.id));
  if (!items.length) {
    toast.error('请先选择要生成的草稿');
    return;
  }

  const channel = begin(items.length);
  try {
    const result = await ipc<RunSummary>('text2video_generate_batch', {
      options: { template: 'scroll', settings: normalizeSettings(settings.value) },
      inputs: items.map((d) => ({
        title: d.title,
        author: d.author,
        content: d.content,
        source: d.source,
      })),
      channel,
    });
    summary.value = result;

    // 生成后按需删除已成功出片的草稿（结果顺序与 inputs 一致）
    if (deleteAfterGenerate.value && !result.cancelled) {
      const succeededIds = result.results
        .map((r, index) => (r.status === 'done' ? items[index]?.id : null))
        .filter((id): id is number => id !== null);
      if (succeededIds.length) {
        for (const id of succeededIds) {
          await ipc('text2video_draft_delete', { id });
        }
        selected.value = selected.value.filter((id) => !succeededIds.includes(id));
        await load();
      }
    }

    if (result.cancelled) {
      toast.info(`已取消，本次生成 ${result.rendered} 条`);
    } else {
      toast.success(`批量生成完成：成功 ${result.rendered} 条`);
      void notify('批量生成完成', `成功生成 ${result.rendered} 条视频`);
    }
  } catch (err) {
    toast.error(`批量生成失败: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    finish();
  }
}

onMounted(load);
</script>

<template>
  <ToolShell title="草稿箱" description="保存文章，多选后批量生成视频">
    <template #actions>
      <Button variant="outline" size="sm" @click="openNew">
        <Plus class="mr-1 size-3.5" />
        新建草稿
      </Button>
      <Button variant="outline" size="sm" :disabled="loading" @click="load">
        <RefreshCw class="mr-1 size-3.5" :class="{ 'animate-spin': loading }" />
        刷新
      </Button>
    </template>

    <div class="mx-auto grid w-full grid-cols-12 gap-4">
      <div class="col-span-12 space-y-4">
        <!-- 工具栏 -->
        <div class="flex flex-wrap items-center gap-3 rounded-lg border bg-card px-4 py-2.5">
          <label class="flex cursor-pointer items-center gap-2 text-sm">
            <Checkbox :model-value="allSelected" @update:model-value="toggleAll" />
            全选
          </label>
          <span class="text-xs text-muted-foreground">
            已选 {{ selectedCount }} / {{ drafts.length }}
          </span>
          <label class="flex cursor-pointer items-center gap-2 text-sm">
            <Checkbox v-model="deleteAfterGenerate" />
            生成后删除草稿
          </label>
          <div class="ml-auto flex items-center gap-2">
            <Button size="sm" :disabled="running || !selectedCount" @click="batchGenerate">
              <Play class="mr-1 size-4" />
              批量生成
            </Button>
            <Button v-if="running" variant="destructive" size="sm" @click="cancel">
              <Square class="mr-1 size-4" />
              取消
            </Button>
            <Button variant="outline" size="sm" :disabled="!selectedCount" @click="removeSelected">
              <Trash2 class="mr-1 size-3.5" />
              删除所选
            </Button>
          </div>
        </div>

        <!-- 进度 -->
        <div v-if="running || logs.length" class="space-y-2 rounded-lg border bg-card p-4">
          <div class="flex items-center justify-between text-sm">
            <span class="font-medium">
              {{ STAGE_LABELS[progress.stage] ?? (progress.stage || '就绪') }}
            </span>
            <span v-if="progress.total" class="text-xs text-muted-foreground">
              {{ progress.current }} / {{ progress.total }}
            </span>
          </div>
          <div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
            <div
              class="h-full rounded-full bg-primary transition-all"
              :style="{ width: `${progressPercent}%` }"
            />
          </div>
          <pre
            class="max-h-40 overflow-auto whitespace-pre-wrap break-words rounded bg-console p-3 font-mono text-xs leading-relaxed text-muted-foreground"
            >{{ logs.join('\n') }}</pre>
        </div>

        <!-- 结果 -->
        <div v-if="summary" class="space-y-2">
          <h3 class="text-sm font-semibold">
            本次结果
            <span class="ml-2 text-xs font-normal text-muted-foreground">
              {{ summary.cancelled ? '已取消' : '完成' }} · 输出到
              {{ summary.outputDir }}
            </span>
          </h3>
          <div class="space-y-2">
            <div
              v-for="item in summary.results"
              :key="item.refId"
              class="flex items-center justify-between gap-3 rounded-lg border bg-card px-4 py-2.5"
            >
              <div class="flex min-w-0 items-center gap-3">
                <Film class="size-4 shrink-0 text-muted-foreground" />
                <div class="min-w-0">
                  <p class="truncate text-sm">{{ item.title }}</p>
                  <p v-if="item.detail" class="truncate text-xs text-muted-foreground">
                    {{ item.detail }}
                  </p>
                </div>
              </div>
              <div class="flex shrink-0 items-center gap-2">
                <Badge :variant="item.status === 'done' ? 'default' : 'secondary'">
                  {{ statusLabel(item.status) }}
                </Badge>
                <template v-if="item.status === 'done'">
                  <Button variant="ghost" size="sm" @click="openPath(item.video, false)">
                    打开视频
                  </Button>
                  <Button variant="ghost" size="sm" @click="openPath(item.video, true)">
                    <FolderOpen class="mr-1 size-3.5" />
                    位置
                  </Button>
                </template>
              </div>
            </div>
          </div>
        </div>

        <!-- 草稿列表 -->
        <div v-if="drafts.length" class="space-y-1.5">
          <div
            v-for="draft in drafts"
            :key="draft.id"
            class="flex items-center gap-3 rounded-lg border bg-card px-4 py-2.5"
          >
            <Checkbox
              :model-value="isSelected(draft.id)"
              @update:model-value="(value) => toggleSelect(draft.id, value)"
            />
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm">{{ draft.title }}</p>
              <p class="truncate text-xs text-muted-foreground">
                {{ draft.author || '佚名' }} · {{ draft.updatedAt }}
              </p>
            </div>
            <Badge :variant="draft.source === 'ai' ? 'default' : 'secondary'">
              {{ draft.source === 'ai' ? 'AI' : '手动' }}
            </Badge>
            <div class="flex shrink-0 items-center gap-1">
              <Button variant="ghost" size="sm" @click="openEdit(draft)">
                <Pencil class="mr-1 size-3.5" />
                编辑
              </Button>
              <Button variant="ghost" size="sm" @click="remove(draft.id)">
                <Trash2 class="mr-1 size-3.5" />
                删除
              </Button>
            </div>
          </div>
        </div>
        <p
          v-else
          class="rounded-lg border border-dashed px-4 py-10 text-center text-sm text-muted-foreground"
        >
          暂无草稿，点击右上角「新建草稿」，或在生成页把内容「存为草稿」
        </p>
      </div>
    </div>

    <!-- 编辑对话框：宽版，最大高度受限于视口，正文框内部滚动 -->
    <Dialog v-model:open="editorOpen">
      <DialogContent class="flex max-h-[85vh] flex-col sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{{ editor.id ? '编辑草稿' : '新建草稿' }}</DialogTitle>
        </DialogHeader>
        <div class="min-h-0 flex-1 overflow-y-auto pr-1">
          <ArticleForm v-model="editorForm" compact />
        </div>
        <DialogFooter>
          <Button variant="outline" @click="editorOpen = false">取消</Button>
          <Button :disabled="saving" @click="saveEditor">保存</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </ToolShell>
</template>
