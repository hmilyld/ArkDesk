<!--
  草稿箱：保存手动/AI 文章，多选后批量生成视频。
-->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
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
import { notifyIfBackground } from '@/core/notify';
import {
  FileText,
  Plus,
  RefreshCw,
  Trash2,
  Pencil,
  Play,
  Square,
  Film,
  FolderOpen,
} from '@lucide/vue';
import ToolShell from '@/components/tool/ToolShell.vue';
import Panel from '@/components/tool/Panel.vue';
import EmptyState from '@/components/native/EmptyState.vue';
import ListRow from '@/components/native/ListRow.vue';
import ArticleForm from '../components/ArticleForm.vue';
import { useGeneration } from '../composables/useGeneration';
import {
  errorMessage,
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
/** 是否显示已生成的草稿（默认隐藏，但数据保留在草稿箱） */
const showGenerated = ref(false);

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

/** 列表中可见的草稿（默认过滤掉已生成的） */
const visibleDrafts = computed(() =>
  showGenerated.value ? drafts.value : drafts.value.filter((d) => !d.generatedRefId)
);
const generatedCount = computed(() => drafts.value.filter((d) => d.generatedRefId).length);
const selectedCount = computed(() => selected.value.length);
const allSelected = computed(
  () => visibleDrafts.value.length > 0 && selectedCount.value === visibleDrafts.value.length
);

async function load(): Promise<void> {
  loading.value = true;
  try {
    drafts.value = await ipc<Draft[]>('text2video_draft_list');
    const visibleIds = new Set(visibleDrafts.value.map((d) => d.id));
    selected.value = selected.value.filter((id) => visibleIds.has(id));
  } catch (err) {
    toast.error(`加载草稿失败: ${errorMessage(err)}`);
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
  selected.value = value === true ? visibleDrafts.value.map((d) => d.id) : [];
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
    toast.error(`保存草稿失败: ${errorMessage(err)}`);
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
    toast.error(`删除草稿失败: ${errorMessage(err)}`);
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
    toast.error(`删除草稿失败: ${errorMessage(err)}`);
  }
}

async function batchGenerate(): Promise<void> {
  if (running.value) return;
  const items = visibleDrafts.value.filter((d) => selected.value.includes(d.id));
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
        draftId: d.id,
      })),
      channel,
    });
    summary.value = result;

    selected.value = [];
    await load();
    if (result.rendered > 0) {
      toast.info(`已生成 ${result.rendered} 条，可在「显示已生成」中查看对应草稿`);
    }

    if (result.cancelled) {
      toast.info(`已取消，本次生成 ${result.rendered} 条`);
    } else {
      toast.success(`批量生成完成：成功 ${result.rendered} 条`);
      void notifyIfBackground('批量生成完成', `成功生成 ${result.rendered} 条视频`);
    }
  } catch (err) {
    toast.error(`批量生成失败: ${errorMessage(err)}`);
  } finally {
    finish();
  }
}

// 隐藏已生成草稿时，同步取消对不可见项的选择
watch(showGenerated, () => {
  const visibleIds = new Set(visibleDrafts.value.map((d) => d.id));
  selected.value = selected.value.filter((id) => visibleIds.has(id));
});

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
        <!-- 草稿列表：选择操作在面板头部；条目用分隔线（一条一卡会叠盒子） -->
        <Panel title="草稿" :hint="`已选 ${selectedCount} / ${visibleDrafts.length}`">
          <template #actions>
            <label class="flex cursor-pointer items-center gap-1.5 text-sm">
              <Checkbox :model-value="allSelected" @update:model-value="toggleAll" />
              全选
            </label>
            <label
              v-if="generatedCount"
              class="flex cursor-pointer items-center gap-1.5 text-sm text-muted-foreground"
            >
              <Checkbox v-model="showGenerated" />
              显示已生成（{{ generatedCount }}）
            </label>
            <Button size="sm" :disabled="running || !selectedCount" @click="batchGenerate">
              <Play class="mr-1 size-3.5" />
              批量生成
            </Button>
            <Button v-if="running" variant="destructive" size="sm" @click="cancel">
              <Square class="mr-1 size-3.5" />
              取消
            </Button>
            <Button variant="outline" size="sm" :disabled="!selectedCount" @click="removeSelected">
              <Trash2 class="mr-1 size-3.5" />
              删除所选
            </Button>
          </template>

          <div v-if="visibleDrafts.length" class="divide-y">
            <ListRow
              v-for="draft in visibleDrafts"
              :key="draft.id"
              :title="draft.title"
              :description="`${draft.author || '佚名'} · ${draft.updatedAt}`"
              class="px-0 py-2.5"
            >
              <template #leading>
                <Checkbox
                  :model-value="isSelected(draft.id)"
                  @update:model-value="(value) => toggleSelect(draft.id, value)"
                />
              </template>
              <template #trailing>
                <Badge v-if="draft.generatedRefId" variant="outline">已生成</Badge>
                <Badge :variant="draft.source === 'ai' ? 'default' : 'secondary'">
                  {{ draft.source === 'ai' ? 'AI' : '手动' }}
                </Badge>
                <Button variant="ghost" size="sm" @click="openEdit(draft)">
                  <Pencil class="mr-1 size-3.5" />
                  编辑
                </Button>
                <Button variant="ghost" size="sm" @click="remove(draft.id)">
                  <Trash2 class="mr-1 size-3.5" />
                  删除
                </Button>
              </template>
            </ListRow>
          </div>

          <EmptyState
            v-else
            :icon="FileText"
            :title="drafts.length ? '剩余草稿均已生成' : '暂无草稿'"
            :description="
              drafts.length
                ? '勾选「显示已生成」可查看已生成的草稿'
                : '点击右上角「新建草稿」，或在生成页把内容「存为草稿」'
            "
          />
        </Panel>

        <!-- 进度 -->
        <Panel v-if="running || logs.length" title="生成进度">
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
            class="max-h-40 overflow-auto rounded-md bg-console p-3 font-mono text-xs leading-relaxed break-words whitespace-pre-wrap text-console-foreground/80"
            >{{ logs.join('\n') }}</pre>
        </Panel>

        <!-- 本次结果 -->
        <Panel
          v-if="summary"
          title="本次结果"
          :hint="`${summary.cancelled ? '已取消' : '完成'} · ${summary.outputDir}`"
        >
          <ListRow
            v-for="item in summary.results"
            :key="item.refId"
            :icon="Film"
            :title="item.title"
            :description="item.detail"
            class="px-0 py-2"
          >
            <template #trailing>
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
            </template>
          </ListRow>
        </Panel>
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
