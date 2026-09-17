<!--
  数据访问进阶：core/db 通用 CRUD + 事务 + 二进制文件 IO 演示。
  - 通用 CRUD：db.insert / findAll / updateById / deleteById / deleteWhere / count
  - 事务：runInTransaction 成功提交与失败整体回滚（对比 count 证明原子性）
  - 二进制文件：file_read_bytes（base64）读取预览，file_write_bytes 另存副本
  每个模块用 Panel 包裹；错误就地呈现，操作结果就地反馈。
-->
<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { Plus, RefreshCw, Trash2 } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
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
import { db } from '@/core/db';
import { ipc } from '@/core/ipc';
import { normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';
import type { CapItem } from '../schema';
import EmptyState from '@/components/native/EmptyState.vue';
import ErrorState from '@/components/native/ErrorState.vue';
import FormRow from '@/components/native/FormRow.vue';
import ListRow from '@/components/native/ListRow.vue';
import LoadingState from '@/components/native/LoadingState.vue';
import Panel from '@/components/tool/Panel.vue';
import ToolShell from '@/components/tool/ToolShell.vue';

const TABLE = 'cap_items';

function describe(operation: string, err: unknown): string {
  const error = normalizeError(err);
  logger.error(`${operation}失败: [${error.code}] ${error.message}`);
  return `${operation}失败：${error.message}`;
}

// ── 通用 CRUD ─────────────────────────────────────────────────
const items = ref<CapItem[]>([]);
const total = ref(0);
const name = ref('');
const value = ref(0);
const busy = ref(false);
const nameError = ref('');
const crudError = ref('');

async function load(): Promise<void> {
  busy.value = true;
  crudError.value = '';
  try {
    items.value = await db.findAll<CapItem>(TABLE, { orderBy: 'id DESC', limit: 50 });
    total.value = await db.count(TABLE);
  } catch (err) {
    crudError.value = describe('加载', err);
  } finally {
    busy.value = false;
  }
}

async function addItem(): Promise<void> {
  const trimmed = name.value.trim();
  if (!trimmed) {
    nameError.value = '名称不能为空';
    return;
  }
  nameError.value = '';
  crudError.value = '';
  try {
    await db.insert(TABLE, { name: trimmed, value: value.value });
    name.value = '';
    value.value = 0;
    await load();
  } catch (err) {
    crudError.value = describe('新增', err);
  }
}

async function bumpValue(item: CapItem): Promise<void> {
  crudError.value = '';
  try {
    await db.updateById(TABLE, item.id, { value: item.value + 1 });
    await load();
  } catch (err) {
    crudError.value = describe('更新', err);
  }
}

async function removeItem(item: CapItem): Promise<void> {
  crudError.value = '';
  try {
    await db.deleteById(TABLE, item.id);
    await load();
  } catch (err) {
    crudError.value = describe('删除', err);
  }
}

// ── 条件删除（破坏性操作 → destructive + Alert 确认） ─────────
const bulkDeleteOpen = ref(false);
const bulkDeleteNote = ref('');

async function confirmRemoveZeroValue(): Promise<void> {
  bulkDeleteOpen.value = false;
  crudError.value = '';
  try {
    const affected = await db.deleteWhere(TABLE, { value: 0 });
    bulkDeleteNote.value = `已删除 value=0 的条目 ${affected} 条`;
    await load();
  } catch (err) {
    crudError.value = describe('条件删除', err);
  }
}

// ── 事务：成功提交 / 失败整体回滚 ─────────────────────────────
const txBusy = ref(false);
const txResult = ref('');
const txResultOk = ref(true);

async function txCommit(): Promise<void> {
  txBusy.value = true;
  txResult.value = '';
  try {
    await db.runInTransaction([
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['事务条目 A', 10] },
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['事务条目 B', 20] },
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['事务条目 C', 30] },
    ]);
    txResult.value = '事务已提交：新增 3 条';
    txResultOk.value = true;
    await load();
  } catch (err) {
    txResult.value = describe('事务提交', err);
    txResultOk.value = false;
  } finally {
    txBusy.value = false;
  }
}

async function txRollback(): Promise<void> {
  txBusy.value = true;
  txResult.value = '';
  const before = await db.count(TABLE);
  try {
    await db.runInTransaction([
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['回滚条目 A', 1] },
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['回滚条目 B', 2] },
      // 故意引用不存在的列 → 触发错误，前两条应一并回滚
      { sql: `INSERT INTO ${TABLE} (nonexistent_column) VALUES ($1)`, params: ['boom'] },
    ]);
    txResult.value = '预期失败却提交成功，请检查迁移。';
    txResultOk.value = false;
  } catch (err) {
    const after = await db.count(TABLE);
    const absorbed = before === after;
    txResult.value = absorbed
      ? `事务已回滚：count 保持 ${before}，已证明原子性`
      : `事务已回滚，但 count ${before} → ${after}，不符合预期`;
    txResultOk.value = absorbed;
    logger.warn(`事务回滚演示: [${normalizeError(err).code}] ${normalizeError(err).message}`);
  } finally {
    txBusy.value = false;
    await load();
  }
}

// ── 二进制文件 IO（base64 通道） ──────────────────────────────
const MIME: Record<string, string> = {
  png: 'image/png',
  jpg: 'image/jpeg',
  jpeg: 'image/jpeg',
  gif: 'image/gif',
  webp: 'image/webp',
  bmp: 'image/bmp',
  svg: 'image/svg+xml',
};

const pickedPath = ref('');
const pickedSize = ref(0);
const pickedPreview = ref('');
const pickedBase64 = ref('');
const binaryNote = ref('');
const binaryError = ref('');

function base64ByteLength(base64: string): number {
  const padding = base64.endsWith('==') ? 2 : base64.endsWith('=') ? 1 : 0;
  return Math.floor((base64.length * 3) / 4) - padding;
}

async function pickBinary(): Promise<void> {
  const path = await openFileDialog({ multiple: false, directory: false });
  if (typeof path !== 'string') return;
  binaryError.value = '';
  binaryNote.value = '';
  try {
    const base64 = await ipc<string>('file_read_bytes', { path });
    const ext = path.split('.').pop()?.toLowerCase() ?? '';
    const mime = MIME[ext];
    pickedPath.value = path;
    pickedBase64.value = base64;
    pickedSize.value = base64ByteLength(base64);
    pickedPreview.value = mime ? `data:${mime};base64,${base64}` : '';
    logger.info(`读取二进制文件: ${path}（${pickedSize.value} 字节）`);
  } catch (err) {
    binaryError.value = describe('读取文件', err);
  }
}

async function saveBinaryCopy(): Promise<void> {
  if (!pickedBase64.value) return;
  const suggested = pickedPath.value.replace(/(\.[^.]+)?$/, '-copy$1');
  const target = await saveFileDialog({ defaultPath: suggested });
  if (!target) return;
  binaryError.value = '';
  binaryNote.value = '';
  try {
    await ipc('file_write_bytes', { path: target, contents: pickedBase64.value });
    binaryNote.value = `副本已保存：${target}`;
  } catch (err) {
    binaryError.value = describe('保存副本', err);
  }
}

onMounted(load);
</script>

<template>
  <ToolShell
    title="数据访问进阶"
    description="通用 CRUD、事务原子性与二进制文件读写（cap_items 表）"
  >
    <div class="grid grid-cols-12">
      <div class="col-span-12 space-y-4 lg:col-span-8 lg:col-start-3">
        <!-- 通用 CRUD -->
        <Panel title="通用 CRUD" :hint="`cap_items · 共 ${total} 条`">
          <template #actions>
            <Button variant="ghost" size="sm" :disabled="busy" @click="load">
              <RefreshCw class="size-3.5" />
              刷新
            </Button>
            <Button
              variant="ghost"
              size="sm"
              class="text-destructive hover:text-destructive"
              :disabled="busy"
              @click="bulkDeleteOpen = true"
            >
              删除 value=0
            </Button>
          </template>
          <p class="text-xs text-muted-foreground">
            db.insert / findAll / updateById / deleteById / deleteWhere / count
          </p>

          <form class="space-y-3" @submit.prevent="addItem">
            <FormRow label="名称" required :error="nameError">
              <template #default="{ id, invalid, describedBy }">
                <Input
                  :id="id"
                  v-model="name"
                  placeholder="条目名称"
                  :aria-invalid="invalid"
                  :aria-describedby="describedBy"
                />
              </template>
            </FormRow>
            <FormRow label="数值" description="点击列表中的 +1 可逐条累加">
              <Input v-model.number="value" type="number" class="w-28 text-right tabular-nums" />
            </FormRow>
            <div class="flex justify-end">
              <Button type="submit" :disabled="busy">
                <Plus class="size-3.5" />
                新增
              </Button>
            </div>
          </form>

          <ErrorState v-if="crudError" :message="crudError" :on-retry="load" />
          <p v-if="bulkDeleteNote" role="status" class="text-xs text-muted-foreground">
            {{ bulkDeleteNote }}
          </p>

          <LoadingState v-if="busy" :rows="4" />
          <EmptyState
            v-else-if="items.length === 0"
            title="还没有条目"
            description="在上方填写名称与数值，新增第一条记录"
          />
          <div v-else class="divide-y divide-border">
            <ListRow
              v-for="item in items"
              :key="item.id"
              class="rounded-none px-0"
              :title="item.name"
              :description="`ID: ${item.id}`"
            >
              <template #trailing>
                <span class="w-12 text-right font-mono text-xs tabular-nums text-muted-foreground">
                  {{ item.value }}
                </span>
                <Button variant="outline" size="sm" @click="bumpValue(item)">+1</Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  class="text-destructive hover:text-destructive"
                  title="删除条目"
                  aria-label="删除条目"
                  @click="removeItem(item)"
                >
                  <Trash2 class="size-4" />
                </Button>
              </template>
            </ListRow>
          </div>
        </Panel>

        <!-- 事务 -->
        <Panel title="事务" hint="runInTransaction">
          <p class="text-xs text-muted-foreground">
            Rust 侧单事务执行，任一语句失败则整体回滚；失败路径同时演示错误处理
          </p>
          <div class="flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" :disabled="txBusy" @click="txCommit">
              提交 3 条事务
            </Button>
            <Button variant="outline" size="sm" :disabled="txBusy" @click="txRollback">
              演示失败回滚
            </Button>
          </div>
          <p
            v-if="txResult"
            role="status"
            class="text-xs"
            :class="txResultOk ? 'text-success' : 'text-destructive'"
          >
            {{ txResult }}
          </p>
        </Panel>

        <!-- 二进制文件 -->
        <Panel title="二进制文件" hint="base64 通道">
          <p class="text-xs text-muted-foreground">
            file_read_bytes 读取任意文件为 base64；file_write_bytes 写回副本（图片可预览）
          </p>
          <div class="flex flex-wrap items-center gap-2">
            <Button variant="secondary" size="sm" @click="pickBinary">选择文件…</Button>
            <Button variant="outline" size="sm" :disabled="!pickedBase64" @click="saveBinaryCopy">
              另存副本
            </Button>
            <span v-if="pickedPath" class="font-mono text-xs tabular-nums text-muted-foreground">
              {{ pickedSize }} 字节
            </span>
          </div>
          <ErrorState v-if="binaryError" :message="binaryError" />
          <p v-if="binaryNote" role="status" class="text-xs text-muted-foreground break-all">
            {{ binaryNote }}
          </p>
          <div v-if="pickedPath" class="space-y-2 rounded-md bg-muted p-3">
            <p class="break-all font-mono text-xs text-muted-foreground">{{ pickedPath }}</p>
            <img
              v-if="pickedPreview"
              :src="pickedPreview"
              alt="文件预览"
              class="max-h-48 rounded-md border object-contain"
            />
            <p v-else class="text-xs text-muted-foreground">非图片文件，仅可另存副本</p>
          </div>
        </Panel>
      </div>
    </div>

    <!-- 条件删除确认（破坏性操作） -->
    <AlertDialog :open="bulkDeleteOpen" @update:open="(open) => (bulkDeleteOpen = open)">
      <AlertDialogContent class="sm:max-w-sm">
        <AlertDialogHeader>
          <AlertDialogTitle>删除所有 value=0 的条目？</AlertDialogTitle>
          <AlertDialogDescription>
            该操作会永久删除 value 为 0 的全部条目，且不可撤销。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>取消</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmRemoveZeroValue"
          >
            删除
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </ToolShell>
</template>
