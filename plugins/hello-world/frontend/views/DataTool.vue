<!--
  数据访问进阶：core/db 通用 CRUD + 事务 + 二进制文件 IO 演示。
  - 通用 CRUD：db.insert / findAll / updateById / deleteById / deleteWhere / count
  - 事务：runInTransaction 成功提交与失败整体回滚（对比 count 证明原子性）
  - 二进制文件：file_read_bytes（base64）读取预览，file_write_bytes 另存副本
-->
<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { Plus, Trash2 } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { db } from '@/core/db';
import { ipc } from '@/core/ipc';
import { normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';
import type { CapItem } from '../schema';
import ToolShell from '@/components/tool/ToolShell.vue';

const TABLE = 'cap_items';

// ── 通用 CRUD ─────────────────────────────────────────────────
const items = ref<CapItem[]>([]);
const total = ref(0);
const name = ref('');
const value = ref(0);
const busy = ref(false);

async function load(): Promise<void> {
  busy.value = true;
  try {
    items.value = await db.findAll<CapItem>(TABLE, { orderBy: 'id DESC', limit: 50 });
    total.value = await db.count(TABLE);
  } catch (err) {
    reportFailure('加载', err);
  } finally {
    busy.value = false;
  }
}

async function addItem(): Promise<void> {
  const trimmed = name.value.trim();
  if (!trimmed) {
    toast.warning('名称不能为空');
    return;
  }
  try {
    const id = await db.insert(TABLE, { name: trimmed, value: value.value });
    toast.success(`已新增（ID: ${id}）`);
    name.value = '';
    value.value = 0;
    await load();
  } catch (err) {
    reportFailure('新增', err);
  }
}

async function bumpValue(item: CapItem): Promise<void> {
  try {
    await db.updateById(TABLE, item.id, { value: item.value + 1 });
    await load();
  } catch (err) {
    reportFailure('更新', err);
  }
}

async function removeItem(item: CapItem): Promise<void> {
  try {
    await db.deleteById(TABLE, item.id);
    toast.success(`已删除（ID: ${item.id}）`);
    await load();
  } catch (err) {
    reportFailure('删除', err);
  }
}

async function removeZeroValue(): Promise<void> {
  try {
    const affected = await db.deleteWhere(TABLE, { value: 0 });
    toast.success(`已删除 value=0 的条目 ${affected} 条`);
    await load();
  } catch (err) {
    reportFailure('条件删除', err);
  }
}

// ── 事务：成功提交 / 失败整体回滚 ─────────────────────────────
const txBusy = ref(false);

async function txCommit(): Promise<void> {
  txBusy.value = true;
  try {
    await db.runInTransaction([
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['事务条目 A', 10] },
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['事务条目 B', 20] },
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['事务条目 C', 30] },
    ]);
    toast.success('事务已提交（3 条）');
    await load();
  } catch (err) {
    reportFailure('事务提交', err);
  } finally {
    txBusy.value = false;
  }
}

async function txRollback(): Promise<void> {
  txBusy.value = true;
  const before = await db.count(TABLE);
  try {
    await db.runInTransaction([
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['回滚条目 A', 1] },
      { sql: `INSERT INTO ${TABLE} (name, value) VALUES ($1, $2)`, params: ['回滚条目 B', 2] },
      // 故意引用不存在的列 → 触发错误，前两条应一并回滚
      { sql: `INSERT INTO ${TABLE} (nonexistent_column) VALUES ($1)`, params: ['boom'] },
    ]);
    toast.error('预期失败却提交成功（请检查迁移）');
  } catch (err) {
    const after = await db.count(TABLE);
    const absorbed = before === after;
    toast.info('事务已回滚', {
      description: absorbed
        ? `count 保持 ${before}，证明原子性`
        : `count ${before} → ${after}（不符合预期）`,
    });
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

function base64ByteLength(base64: string): number {
  const padding = base64.endsWith('==') ? 2 : base64.endsWith('=') ? 1 : 0;
  return Math.floor((base64.length * 3) / 4) - padding;
}

async function pickBinary(): Promise<void> {
  const path = await openFileDialog({ multiple: false, directory: false });
  if (typeof path !== 'string') return;
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
    reportFailure('读取文件', err);
  }
}

async function saveBinaryCopy(): Promise<void> {
  if (!pickedBase64.value) return;
  const suggested = pickedPath.value.replace(/(\.[^.]+)?$/, '-copy$1');
  const target = await saveFileDialog({ defaultPath: suggested });
  if (!target) return;
  try {
    await ipc('file_write_bytes', { path: target, contents: pickedBase64.value });
    toast.success('副本已保存', { description: target });
  } catch (err) {
    reportFailure('保存副本', err);
  }
}

function reportFailure(prefix: string, err: unknown): void {
  const error = normalizeError(err);
  logger.error(`${prefix}失败: [${error.code}] ${error.message}`);
  toast.error(`${prefix}失败：${error.message}`, { description: error.code });
}

onMounted(load);
</script>

<template>
  <ToolShell
    title="数据访问进阶"
    description="通用 CRUD、事务原子性与二进制文件读写（cap_items 表）"
  >
    <div class="mx-auto grid w-full grid-cols-12">
      <div class="col-span-12 space-y-6 lg:col-start-3 lg:col-span-8">
        <!-- 通用 CRUD -->
        <section class="space-y-2.5">
          <div class="flex items-center justify-between">
            <div>
              <h3 class="text-sm font-medium">通用 CRUD</h3>
              <p class="text-xs text-muted-foreground">
                db.insert / findAll / updateById / deleteById / deleteWhere / count（共
                {{ total }} 条）
              </p>
            </div>
            <div class="flex gap-2">
              <Button variant="outline" size="sm" :disabled="busy" @click="load">刷新</Button>
              <Button variant="ghost" size="sm" :disabled="busy" @click="removeZeroValue">
                删除 value=0
              </Button>
            </div>
          </div>

          <form class="flex items-end gap-2" @submit.prevent="addItem">
            <div class="flex-1 space-y-1">
              <Label for="cap-name" class="text-xs text-muted-foreground">名称</Label>
              <Input id="cap-name" v-model="name" placeholder="条目名称" />
            </div>
            <div class="w-28 space-y-1">
              <Label for="cap-value" class="text-xs text-muted-foreground">数值</Label>
              <Input id="cap-value" v-model.number="value" type="number" />
            </div>
            <Button type="submit">
              <Plus class="size-4" />
              新增
            </Button>
          </form>

          <p
            v-if="items.length === 0"
            class="rounded-lg border border-dashed py-8 text-center text-xs text-muted-foreground"
          >
            暂无数据
          </p>
          <div
            v-else
            class="divide-y divide-border/60 overflow-hidden rounded-lg border border-border bg-card"
          >
            <div v-for="item in items" :key="item.id" class="flex items-center gap-3 px-3 py-2">
              <span class="w-10 shrink-0 font-mono text-xs text-muted-foreground">{{
                item.id
              }}</span>
              <span class="min-w-0 flex-1 truncate text-sm">{{ item.name }}</span>
              <span class="w-12 shrink-0 text-right font-mono text-xs text-muted-foreground">
                {{ item.value }}
              </span>
              <Button variant="outline" size="sm" @click="bumpValue(item)">+1</Button>
              <Button
                variant="ghost"
                size="icon-sm"
                class="text-destructive"
                aria-label="删除"
                @click="removeItem(item)"
              >
                <Trash2 class="size-4" />
              </Button>
            </div>
          </div>
        </section>

        <!-- 事务 -->
        <section class="space-y-2.5">
          <div>
            <h3 class="text-sm font-medium">事务（runInTransaction）</h3>
            <p class="text-xs text-muted-foreground">
              Rust 侧单事务执行，任一语句失败则整体回滚；失败路径同时演示错误处理
            </p>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button variant="secondary" size="sm" :disabled="txBusy" @click="txCommit">
              提交 3 条事务
            </Button>
            <Button variant="outline" size="sm" :disabled="txBusy" @click="txRollback">
              演示失败回滚
            </Button>
          </div>
        </section>

        <!-- 二进制文件 -->
        <section class="space-y-2.5">
          <div>
            <h3 class="text-sm font-medium">二进制文件（base64 通道）</h3>
            <p class="text-xs text-muted-foreground">
              file_read_bytes 读取任意文件为 base64；file_write_bytes 写回副本（图片可预览）
            </p>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <Button variant="secondary" size="sm" @click="pickBinary">选择文件…</Button>
            <Button variant="outline" size="sm" :disabled="!pickedBase64" @click="saveBinaryCopy">
              另存副本
            </Button>
            <span v-if="pickedPath" class="font-mono text-xs text-muted-foreground">
              {{ pickedSize }} 字节
            </span>
          </div>
          <div v-if="pickedPath" class="space-y-2 rounded-lg border border-border bg-card p-3">
            <p class="break-all font-mono text-xs text-muted-foreground">{{ pickedPath }}</p>
            <img
              v-if="pickedPreview"
              :src="pickedPreview"
              alt="文件预览"
              class="max-h-48 rounded-md border border-border object-contain"
            />
            <p v-else class="text-xs text-muted-foreground">非图片文件，仅可另存副本</p>
          </div>
        </section>
      </div>
    </div>
  </ToolShell>
</template>
