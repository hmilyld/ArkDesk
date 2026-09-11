<!--
  数据表格：完整表格页样板（span=12 满幅档位示范）。
  覆盖：列表渲染、前端分页、新增/编辑（Dialog）、删除确认（AlertDialog）、
  行内动作（DropdownMenu）、批量演示数据、CSV 导出/导入（Rust 命令 + 文件对话框）。
-->
<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { MoreHorizontal, Pencil, Plus, Trash2 } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { kdb } from '@/core/db';
import { ipc } from '@/core/ipc';
import { normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';
import { desc, eq } from 'drizzle-orm';
import { helloTasks, type HelloTask } from '../schema';
import {
  buildSeedTasks,
  PAGE_SIZE,
  PRIORITY_LABELS,
  STATUS_BADGE_VARIANTS,
  STATUS_OPTIONS,
} from '../shared';
import ToolShell from '@/components/tool/ToolShell.vue';

// ── 数据与分页（前端分页：全量拉取，本地切页） ────────────────
const tasks = ref<HelloTask[]>([]);
const loading = ref(false);
const page = ref(1);

const pageCount = computed(() => Math.max(1, Math.ceil(tasks.value.length / PAGE_SIZE)));
const pageRows = computed(() =>
  tasks.value.slice((page.value - 1) * PAGE_SIZE, page.value * PAGE_SIZE)
);
const pageNumbers = computed(() =>
  Array.from({ length: pageCount.value }, (_, i) => i + 1).filter(
    // 页数多时只显示当前页前后各 2 页 + 首尾（简化示意）
    (n) => pageCount.value <= 5 || n === 1 || n === pageCount.value || Math.abs(n - page.value) <= 2
  )
);

async function loadTasks(): Promise<void> {
  loading.value = true;
  try {
    tasks.value = await kdb.select().from(helloTasks).orderBy(desc(helloTasks.id));
    page.value = Math.min(page.value, pageCount.value);
  } finally {
    loading.value = false;
  }
}

function gotoPage(target: number): void {
  page.value = Math.min(Math.max(target, 1), pageCount.value);
}

// ── 批量演示数据：一次多行插入，便于观察分页 ──────────────────
const seeding = ref(false);

async function seedTasks(): Promise<void> {
  seeding.value = true;
  try {
    await kdb.insert(helloTasks).values(buildSeedTasks(100));
    toast.success('已填充 100 条演示数据');
    await loadTasks();
  } catch (err) {
    const error = normalizeError(err);
    logger.error(`填充演示数据失败: [${error.code}] ${error.message}`);
  } finally {
    seeding.value = false;
  }
}

// ── 新增 / 编辑（同一个 Dialog，create / edit 两态复用） ──────
const dialogOpen = ref(false);
const editingId = ref<number | null>(null);
const formTitle = ref('');
const formStatus = ref<'pending' | 'in_progress' | 'done'>('pending');
const formPriority = ref<'high' | 'medium' | 'low'>('medium');
const saving = ref(false);

const titleError = computed(() => {
  if (!formTitle.value.trim()) return '任务标题不能为空';
  if (formTitle.value.trim().length > 50) return '标题不能超过 50 个字符';
  return '';
});

function openCreate(): void {
  editingId.value = null;
  formTitle.value = '';
  formStatus.value = 'pending';
  formPriority.value = 'medium';
  dialogOpen.value = true;
}

function openEdit(task: HelloTask): void {
  editingId.value = task.id;
  formTitle.value = task.title;
  formStatus.value = task.status as typeof formStatus.value;
  formPriority.value = task.priority as typeof formPriority.value;
  dialogOpen.value = true;
}

async function saveTask(): Promise<void> {
  if (titleError.value) return;
  saving.value = true;
  try {
    const values = {
      title: formTitle.value.trim(),
      status: formStatus.value,
      priority: formPriority.value,
    };
    if (editingId.value === null) {
      // .returning() 拿自增 id（proxy 驱动的 run 路径不回传 lastInsertId）
      const [created] = await kdb
        .insert(helloTasks)
        .values(values)
        .returning({ id: helloTasks.id });
      toast.success(`任务已添加（ID: ${created?.id}）`, { description: values.title });
    } else {
      await kdb.update(helloTasks).set(values).where(eq(helloTasks.id, editingId.value));
      toast.success('任务已更新', { description: values.title });
    }
    dialogOpen.value = false;
    await loadTasks();
  } finally {
    saving.value = false;
  }
}

// ── 删除（AlertDialog 二次确认） ─────────────────────────────
// AlertDialogAction 点击时先触发弹窗关闭（update:open），再执行按钮的
// click 回调——确认行不能在 update:open 里清空，否则回调读到 null 短路。
// 因此关闭只翻转 deleteOpen，deleting 在删除完成后才清除。
const deleteOpen = ref(false);
const deleting = ref<HelloTask | null>(null);

function askDelete(task: HelloTask): void {
  deleting.value = task;
  deleteOpen.value = true;
}

async function confirmDelete(): Promise<void> {
  const task = deleting.value;
  if (!task) return;
  deleteOpen.value = false;
  try {
    await kdb.delete(helloTasks).where(eq(helloTasks.id, task.id));
    toast.success(`已删除（ID: ${task.id}）`, { description: task.title });
  } finally {
    deleting.value = null;
    await loadTasks();
  }
}

// ── 导出 / 导入 CSV（Rust 命令 + 系统文件对话框） ─────────────
const exporting = ref(false);
const importing = ref(false);

async function exportCsv(): Promise<void> {
  exporting.value = true;
  try {
    const path = await ipc<string>('hello_world_export_csv');
    toast.success('导出成功', { description: path });
  } catch (err) {
    const error = normalizeError(err);
    toast.error(`导出失败：${error.message}`, { description: error.code });
    logger.error(`CSV 导出失败: [${error.code}] ${error.message}`);
  } finally {
    exporting.value = false;
  }
}

async function importCsv(): Promise<void> {
  const path = await openFileDialog({
    multiple: false,
    directory: false,
    filters: [{ name: 'CSV', extensions: ['csv'] }],
  });
  if (!path) return;
  importing.value = true;
  try {
    const count = await ipc<number>('hello_world_import_csv', { path });
    toast.success(`导入成功：${count} 条任务`);
    await loadTasks();
  } catch (err) {
    const error = normalizeError(err);
    toast.error(`导入失败：${error.message}`, { description: error.code, duration: 6000 });
    logger.error(`CSV 导入失败: [${error.code}] ${error.message}`);
  } finally {
    importing.value = false;
  }
}

function statusLabel(status: string): string {
  return STATUS_OPTIONS.find((option) => option.value === status)?.label ?? status;
}

onMounted(loadTasks);
// KeepAlive 场景：从表单工具切回时刷新数据
onActivated(loadTasks);
</script>

<template>
  <ToolShell
    title="数据表格"
    description="hello_tasks 表完整 CRUD：分页、弹窗编辑、删除确认、CSV 导入导出"
  >
    <template #actions>
      <Button variant="outline" size="sm" :disabled="loading" @click="loadTasks">刷新</Button>
      <Button variant="secondary" size="sm" :disabled="seeding" @click="seedTasks">
        {{ seeding ? '填充中…' : '填充演示数据' }}
      </Button>
      <Button size="sm" @click="openCreate">
        <Plus class="size-4" />
        新增任务
      </Button>
    </template>

    <!-- 工具条：导入 / 导出（更多操作收进下拉，演示 DropdownMenu 用法） -->
    <div class="mb-3 flex items-center justify-between gap-3">
      <p class="text-xs text-muted-foreground">
        共 {{ tasks.length }} 条 · 每页 {{ PAGE_SIZE }} 条
      </p>
      <DropdownMenu>
        <DropdownMenuTrigger as-child>
          <Button variant="outline" size="sm" :disabled="exporting || importing">
            {{ exporting || importing ? '处理中…' : '导入 / 导出' }}
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" class="w-40">
          <DropdownMenuItem @click="exportCsv">导出 CSV</DropdownMenuItem>
          <DropdownMenuItem @click="importCsv">导入 CSV…</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>

    <!-- 表格 -->
    <p v-if="loading" class="py-10 text-center text-xs text-muted-foreground">加载中…</p>
    <div v-else-if="tasks.length === 0" class="rounded-lg border border-dashed py-12 text-center">
      <p class="text-sm text-muted-foreground">暂无任务</p>
      <p class="mt-1 text-xs text-muted-foreground">
        点右上角「填充演示数据」快速造一批，或手动新增
      </p>
    </div>
    <div v-else class="overflow-hidden rounded-lg border border-border bg-card">
      <Table>
        <TableHeader>
          <TableRow class="hover:bg-transparent">
            <TableHead class="w-14 pl-3">ID</TableHead>
            <TableHead>标题</TableHead>
            <TableHead class="w-24">状态</TableHead>
            <TableHead class="w-20">优先级</TableHead>
            <TableHead class="w-40">创建时间</TableHead>
            <TableHead class="w-16 pr-3 text-right">操作</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow v-for="task in pageRows" :key="task.id" class="hover:bg-accent/40">
            <TableCell class="pl-3 font-mono text-xs text-muted-foreground">
              {{ task.id }}
            </TableCell>
            <TableCell class="max-w-0 truncate font-medium">{{ task.title }}</TableCell>
            <TableCell>
              <Badge
                :variant="
                  STATUS_BADGE_VARIANTS[task.status as keyof typeof STATUS_BADGE_VARIANTS] ??
                  'secondary'
                "
              >
                {{ statusLabel(task.status) }}
              </Badge>
            </TableCell>
            <TableCell class="text-muted-foreground">
              {{ PRIORITY_LABELS[task.priority] ?? task.priority }}
            </TableCell>
            <TableCell class="font-mono text-xs text-muted-foreground">
              {{ task.createdAt }}
            </TableCell>
            <TableCell class="pr-3 text-right">
              <DropdownMenu>
                <DropdownMenuTrigger as-child>
                  <Button variant="ghost" size="icon-sm" aria-label="行操作">
                    <MoreHorizontal class="size-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" class="w-28">
                  <DropdownMenuItem @click="openEdit(task)">
                    <Pencil class="size-4" />
                    编辑
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    class="text-destructive focus:text-destructive"
                    @click="askDelete(task)"
                  >
                    <Trash2 class="size-4" />
                    删除
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </div>

    <!-- 分页（前端分页：切换页码切片渲染） -->
    <div v-if="tasks.length > 0" class="mt-3 flex items-center justify-center gap-1.5">
      <Button variant="outline" size="sm" :disabled="page === 1" @click="gotoPage(page - 1)">
        上一页
      </Button>
      <template v-for="(n, index) in pageNumbers" :key="n">
        <span
          v-if="index > 0 && n - pageNumbers[index - 1]! > 1"
          class="px-1 text-muted-foreground"
        >
          …
        </span>
        <Button
          variant="outline"
          size="sm"
          class="min-w-8 px-2 font-mono"
          :class="
            n === page ? 'border-primary/50 bg-primary/10 text-primary' : 'text-muted-foreground'
          "
          @click="gotoPage(n)"
        >
          {{ n }}
        </Button>
      </template>
      <Button
        variant="outline"
        size="sm"
        :disabled="page === pageCount"
        @click="gotoPage(page + 1)"
      >
        下一页
      </Button>
    </div>

    <!-- 新增 / 编辑弹窗（Dialog） -->
    <Dialog v-model:open="dialogOpen">
      <DialogContent class="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{{
            editingId === null ? '新增任务' : `编辑任务 #${editingId}`
          }}</DialogTitle>
          <DialogDescription>写入 hello_tasks 表，数据表格与设置页共享该数据</DialogDescription>
        </DialogHeader>

        <div class="space-y-3 py-2">
          <div class="space-y-1">
            <Label for="task-title">任务标题</Label>
            <Input
              id="task-title"
              v-model="formTitle"
              placeholder="例如：整理周报"
              :aria-invalid="!!titleError"
              @keydown.enter="saveTask"
            />
            <p v-if="titleError" class="text-xs text-destructive">{{ titleError }}</p>
          </div>
          <div class="flex gap-3">
            <div class="flex-1 space-y-1">
              <Label for="task-status">状态</Label>
              <Select v-model="formStatus">
                <SelectTrigger id="task-status" class="w-full">
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
            </div>
            <div class="flex-1 space-y-1">
              <Label for="task-priority">优先级</Label>
              <Select v-model="formPriority">
                <SelectTrigger id="task-priority" class="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="high">高</SelectItem>
                  <SelectItem value="medium">中</SelectItem>
                  <SelectItem value="low">低</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" size="sm" @click="dialogOpen = false">取消</Button>
          <Button size="sm" :disabled="!!titleError || saving" @click="saveTask">
            {{ editingId === null ? '添加' : '保存' }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- 删除确认（AlertDialog 二次确认，防误触） -->
    <AlertDialog :open="deleteOpen" @update:open="(value) => (deleteOpen = value)">
      <AlertDialogContent class="sm:max-w-sm">
        <AlertDialogHeader>
          <AlertDialogTitle>删除任务？</AlertDialogTitle>
          <AlertDialogDescription>
            「{{ deleting?.title }}」将被永久删除，该操作不可撤销。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>取消</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmDelete"
          >
            删除
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </ToolShell>
</template>
