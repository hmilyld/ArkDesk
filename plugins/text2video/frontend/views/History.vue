<!--
  处理记录页：查看历史生成结果，打开产物、显示位置、删除记录。
-->
<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ipc } from '@/core/ipc';
import { RefreshCw, FolderOpen, Trash2, Film, FileText } from '@lucide/vue';
import ToolShell from '@/components/tool/ToolShell.vue';
import { errorMessage, openArtifact as openPath, statusLabel, type HistoryRow } from '../shared';

const rows = ref<HistoryRow[]>([]);
const loading = ref(false);
const viewing = ref<HistoryRow | null>(null);

async function load(): Promise<void> {
  loading.value = true;
  try {
    rows.value = await ipc<HistoryRow[]>('text2video_history', { limit: 200 });
  } catch (err) {
    toast.error(`加载记录失败: ${errorMessage(err)}`);
  } finally {
    loading.value = false;
  }
}

async function remove(refId: string): Promise<void> {
  try {
    await ipc('text2video_delete_history', { refId });
    rows.value = rows.value.filter((row) => row.refId !== refId);
    toast.success('记录已删除');
  } catch (err) {
    toast.error(`删除失败: ${errorMessage(err)}`);
  }
}

onMounted(load);
</script>

<template>
  <ToolShell title="处理记录" description="历史生成结果与状态">
    <template #actions>
      <Button variant="outline" size="sm" :disabled="loading" @click="load">
        <RefreshCw class="mr-1 size-3.5" :class="{ 'animate-spin': loading }" />
        刷新
      </Button>
    </template>

    <div v-if="rows.length" class="overflow-x-auto rounded-lg border bg-card">
      <table class="w-full text-sm">
        <thead>
          <tr class="border-b text-left text-xs text-muted-foreground">
            <th class="px-4 py-2.5 font-medium">标题</th>
            <th class="px-4 py-2.5 font-medium">状态</th>
            <th class="px-4 py-2.5 font-medium">时间</th>
            <th class="px-4 py-2.5 text-right font-medium">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="row in rows" :key="row.refId" class="border-b last:border-0">
            <td class="max-w-0 px-4 py-2.5">
              <p class="truncate">{{ row.title || row.refId }}</p>
              <p v-if="row.detail" class="truncate text-xs text-muted-foreground">
                {{ row.detail }}
              </p>
            </td>
            <td class="px-4 py-2.5">
              <Badge :variant="row.status === 'done' ? 'default' : 'secondary'">
                {{ statusLabel(row.status) }}
              </Badge>
            </td>
            <td class="whitespace-nowrap px-4 py-2.5 text-xs text-muted-foreground">
              {{ row.createdAt }}
            </td>
            <td class="whitespace-nowrap px-4 py-2.5 text-right">
              <Button variant="ghost" size="sm" @click="viewing = row">
                <FileText class="mr-1 size-3.5" />
                查看
              </Button>
              <template v-if="row.status === 'done' && row.video">
                <Button variant="ghost" size="sm" @click="openPath(row.video, false)">
                  <Film class="mr-1 size-3.5" />
                  打开
                </Button>
                <Button variant="ghost" size="sm" @click="openPath(row.video, true)">
                  <FolderOpen class="mr-1 size-3.5" />
                  位置
                </Button>
              </template>
              <Button variant="ghost" size="sm" @click="remove(row.refId)">
                <Trash2 class="mr-1 size-3.5" />
                删除
              </Button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <p
      v-else
      class="rounded-lg border border-dashed px-4 py-10 text-center text-sm text-muted-foreground"
    >
      暂无处理记录
    </p>

    <!-- 文章素材弹窗：保留生成时使用的文字内容 -->
    <Dialog :open="!!viewing" @update:open="(open) => !open && (viewing = null)">
      <DialogContent class="flex max-h-[85vh] flex-col sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle class="truncate">{{ viewing?.title || viewing?.refId }}</DialogTitle>
        </DialogHeader>
        <div class="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1">
          <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
            <span>作者：{{ viewing?.author || '佚名' }}</span>
            <span>来源：{{ viewing?.source === 'ai' ? 'AI 创作' : '手动' }}</span>
            <span>时间：{{ viewing?.createdAt }}</span>
          </div>
          <div
            v-if="viewing?.status === 'done' && viewing?.video"
            class="flex items-center gap-2 text-xs text-muted-foreground"
          >
            <span class="truncate">{{ viewing.video }}</span>
            <Button variant="ghost" size="sm" @click="openPath(viewing.video, false)"
              >打开视频</Button
            >
            <Button variant="ghost" size="sm" @click="openPath(viewing.video, true)">位置</Button>
          </div>
          <pre
            class="whitespace-pre-wrap break-words rounded-lg border bg-card p-4 font-sans text-sm leading-relaxed"
            >{{ viewing?.content || '（无正文内容）' }}</pre>
        </div>
        <DialogFooter>
          <Button variant="outline" @click="viewing = null">关闭</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </ToolShell>
</template>
