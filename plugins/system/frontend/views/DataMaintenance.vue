<!--
  数据维护：连接应用数据库的轻量管理工具。
  左列表（表清单）+ 右详情（数据 / 结构 / DDL 三个 Tab）。
  数据支持分页浏览、单元格内联编辑、删行、新增行；无 rowid 表自动降级只读。
-->
<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { Plus, RefreshCw } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
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
import { normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';
import {
  PAGE_SIZE,
  parseCellValue,
  sqlColumns,
  sqlDdl,
  sqlDeleteRow,
  sqlInsertRow,
  sqlPage,
  sqlPageNoRowid,
  sqlRowCount,
  sqlRowidProbe,
  sqlTableList,
  sqlUpdateCell,
  sqlUpdateRow,
  toColumnInfo,
  type ColumnInfo,
  type DataRow,
  type InsertEntry,
  type PragmaColumnRow,
  type TableInfo,
  type TableSchemaInfo,
} from '../shared';
import TableList from '../components/TableList.vue';
import DataGrid from '../components/DataGrid.vue';
import TableSchema from '../components/TableSchema.vue';
import RowEditorDialog from '../components/RowEditorDialog.vue';
import RowDetailDialog from '../components/RowDetailDialog.vue';
import ToolShell from '@/components/tool/ToolShell.vue';

function fail(operation: string, err: unknown): void {
  const error = normalizeError(err);
  toast.error(`${operation}失败：${error.message}`, { description: error.code, duration: 6000 });
  logger.error(`数据维护 ${operation}失败: [${error.code}] ${error.message}`);
}

// ── 表清单 ──────────────────────────────────────────────────
const tables = ref<TableInfo[]>([]);
const tablesLoading = ref(false);
const selected = ref<string | null>(null);

async function loadTables(): Promise<void> {
  tablesLoading.value = true;
  try {
    const list = await db.select<{ name: string }>(sqlTableList().sql, []);
    const infos = await Promise.all(
      list.map(async (item) => {
        const stmt = sqlRowCount(item.name);
        const countRows = await db
          .select<{ n: number }>(stmt.sql, stmt.params)
          .catch(() => [] as { n: number }[]);
        return { name: item.name, rowCount: countRows[0]?.n ?? 0 };
      })
    );
    tables.value = infos;
    if (selected.value && !infos.some((table) => table.name === selected.value)) {
      selected.value = null;
    }
  } catch (err) {
    fail('加载表清单', err);
  } finally {
    tablesLoading.value = false;
  }
}

async function selectTable(name: string): Promise<void> {
  if (selected.value === name) return;
  selected.value = name;
  page.value = 1;
  tab.value = 'data';
  // 表可能被重建（换 rowid 特性），重探一次代价仅一条 LIMIT 1
  rowidOk.value.delete(name);
  await Promise.all([loadSchema(name), loadRows()]);
}

// ── 表结构 ──────────────────────────────────────────────────
const schema = ref<TableSchemaInfo | null>(null);
const schemaLoading = ref(false);

// 竞态守卫：各加载函数捕获发起时的表名，响应后仅当仍是当前选中表才写状态，
// 快速连续切换表名时慢响应不会覆盖新数据
async function loadSchema(name: string): Promise<void> {
  schemaLoading.value = true;
  try {
    const columnsStmt = sqlColumns(name);
    const ddlStmt = sqlDdl(name);
    const countStmt = sqlRowCount(name);
    const [columns, ddlRows, countRows] = await Promise.all([
      db.select<PragmaColumnRow>(columnsStmt.sql, columnsStmt.params),
      db.select<{ sql: string | null }>(ddlStmt.sql, ddlStmt.params),
      db.select<{ n: number }>(countStmt.sql, countStmt.params),
    ]);
    if (selected.value !== name) return;
    schema.value = {
      name,
      columns: columns.map(toColumnInfo),
      ddl: ddlRows[0]?.sql ?? null,
      rowCount: countRows[0]?.n ?? 0,
    };
  } catch (err) {
    if (selected.value !== name) return;
    // 失败即清空：避免旧表结构配新表数据造成列-行错配
    schema.value = null;
    fail('加载表结构', err);
  } finally {
    schemaLoading.value = false;
  }
}

// ── 数据分页（rowid 探测结果按表缓存，无 rowid 表降级只读） ──
const rows = ref<DataRow[]>([]);
const rowsLoading = ref(false);
const total = ref(0);
const page = ref(1);
const tab = ref<'data' | 'schema' | 'ddl'>('data');
const rowidOk = ref(new Map<string, boolean>());
const canEdit = computed(() => rowidOk.value.get(selected.value ?? '') ?? false);
const pageCount = computed(() => Math.max(1, Math.ceil(total.value / PAGE_SIZE)));

async function loadRows(): Promise<void> {
  const name = selected.value;
  if (!name) {
    rows.value = [];
    total.value = 0;
    return;
  }
  rowsLoading.value = true;
  try {
    if (!rowidOk.value.has(name)) {
      const probe = sqlRowidProbe(name);
      let ok = true;
      try {
        await db.select(probe.sql, probe.params);
      } catch {
        ok = false;
      }
      rowidOk.value.set(name, ok);
    }
    // rowid 可用性取本表缓存值，避免响应式 canEdit 读到切换后的新表
    const canRowid = rowidOk.value.get(name) ?? false;
    const countStmt = sqlRowCount(name);
    const countRows = await db.select<{ n: number }>(countStmt.sql, countStmt.params);
    if (selected.value !== name) return;
    total.value = countRows[0]?.n ?? 0;
    page.value = Math.min(page.value, pageCount.value);
    const offset = (page.value - 1) * PAGE_SIZE;
    const stmt = canRowid
      ? sqlPage(name, PAGE_SIZE, offset)
      : sqlPageNoRowid(name, PAGE_SIZE, offset);
    const pageRows = await db.select<DataRow>(stmt.sql, stmt.params);
    if (selected.value !== name) return;
    rows.value = pageRows;
  } catch (err) {
    if (selected.value !== name) return;
    fail('加载表数据', err);
  } finally {
    rowsLoading.value = false;
  }
}

function gotoPage(target: number): void {
  page.value = Math.min(Math.max(target, 1), pageCount.value);
  void loadRows();
}

// ── 编辑单元格 ──────────────────────────────────────────────
async function handleEditCell(payload: {
  row: DataRow;
  column: ColumnInfo;
  raw: string;
  isNull: boolean;
}): Promise<void> {
  const name = selected.value;
  if (!name) return;
  try {
    const value = parseCellValue(payload.raw, payload.isNull, payload.column);
    const stmt = sqlUpdateCell(name, payload.column.name, value, payload.row.__rid);
    await db.execute(stmt.sql, stmt.params);
    toast.success('单元格已保存', { description: `${payload.column.name} = ${String(value)}` });
    await loadRows();
  } catch (err) {
    fail('更新单元格', err);
  }
}

// ── 删行（AlertDialog 二次确认；关闭与读取的时序约束同 hello-world 样板） ──
const deleteOpen = ref(false);
const deletingRow = ref<DataRow | null>(null);

function askDeleteRow(row: DataRow): void {
  deletingRow.value = row;
  deleteOpen.value = true;
}

async function confirmDeleteRow(): Promise<void> {
  const row = deletingRow.value;
  const name = selected.value;
  if (!row || !name) return;
  deleteOpen.value = false;
  try {
    const stmt = sqlDeleteRow(name, row.__rid);
    const result = await db.execute(stmt.sql, stmt.params);
    toast.success(`已删除 ${result.rowsAffected} 行`);
  } catch (err) {
    fail('删除行', err);
  } finally {
    deletingRow.value = null;
    await Promise.all([loadTables(), loadRows()]);
  }
}

// ── 新增行 ──────────────────────────────────────────────────
const addOpen = ref(false);

async function handleInsert(entries: InsertEntry[]): Promise<void> {
  const name = selected.value;
  if (!name) return;
  try {
    const stmt = sqlInsertRow(name, entries);
    await db.execute(stmt.sql, stmt.params);
    addOpen.value = false;
    toast.success('已新增 1 行');
    page.value = Math.ceil((total.value + 1) / PAGE_SIZE);
    await Promise.all([loadTables(), loadRows()]);
  } catch (err) {
    fail('新增行', err);
  }
}

// ── 行查看/编辑弹窗（长内容场景，操作列入口） ────────────────
const rowDialogOpen = ref(false);
const rowDialogMode = ref<'view' | 'edit'>('view');
const activeRow = ref<DataRow | null>(null);

function openRowDialog(row: DataRow, mode: 'view' | 'edit'): void {
  activeRow.value = row;
  rowDialogMode.value = mode;
  rowDialogOpen.value = true;
}

async function handleRowSave(entries: InsertEntry[]): Promise<void> {
  const name = selected.value;
  const row = activeRow.value;
  if (!name || !row) return;
  try {
    const stmt = sqlUpdateRow(name, entries, row.__rid);
    const result = await db.execute(stmt.sql, stmt.params);
    rowDialogOpen.value = false;
    if (result.rowsAffected === 0) {
      toast.warning('未更新任何行', { description: '该行可能已被删除' });
    } else {
      toast.success(`已保存 ${result.rowsAffected} 行`);
    }
  } catch (err) {
    fail('更新行', err);
  } finally {
    await Promise.all([loadTables(), loadRows()]);
  }
}

// ── 全局刷新（KeepAlive 切回时同样执行） ────────────────────
const refreshing = computed(() => tablesLoading.value || rowsLoading.value);

async function refreshAll(): Promise<void> {
  await loadTables();
  if (selected.value) await Promise.all([loadSchema(selected.value), loadRows()]);
}

onMounted(refreshAll);
onActivated(refreshAll);
</script>

<template>
  <ToolShell
    title="数据维护"
    description="浏览应用数据库的表结构与数据，支持单元格编辑、新增与删除"
  >
    <template #actions>
      <Button variant="outline" size="sm" :disabled="refreshing" @click="refreshAll">
        <RefreshCw class="size-4" :class="{ 'animate-spin': refreshing }" />
        刷新
      </Button>
    </template>

    <div class="grid grid-cols-12 gap-4">
      <!-- 左列：表清单（md+ 悬浮固定；吸附点 = sticky 页头 65px + 内容区 p-5 20px = 85px，
           与卡片初始位置对齐，滚动全程零位移；右侧滚动时保持可见） -->
      <aside class="col-span-12 md:col-span-4 lg:col-span-3">
        <TableList
          class="md:sticky md:top-[85px]"
          :tables="tables"
          :selected="selected"
          :loading="tablesLoading"
          @select="selectTable"
        />
      </aside>

      <!-- 右列：表详情（md+ 定高不滚动，滚动条下沉到各内容区：
           预留 = TitleBar 40px + ToolShell 页头 65px + 内容区 p-5 上下 40px = 145px，
           缺一项都会把 main 撑出整页滚动条） -->
      <section
        class="col-span-12 flex min-w-0 flex-col md:col-span-8 md:max-h-[calc(100dvh-145px)] md:overflow-hidden lg:col-span-9"
      >
        <div v-if="!selected" class="rounded-lg border border-dashed py-24 text-center">
          <p class="text-sm text-muted-foreground">从左侧选择一个表</p>
          <p class="mt-1 text-xs text-muted-foreground">可查看结构、DDL，并直接编辑数据</p>
        </div>

        <Tabs v-else v-model="tab" class="min-h-0 flex-1">
          <div class="mb-3 flex shrink-0 items-center justify-between gap-3">
            <div class="flex min-w-0 items-center gap-2">
              <h3 class="truncate font-mono text-sm font-semibold">{{ selected }}</h3>
              <Badge variant="secondary">{{ total }} 行</Badge>
              <Badge v-if="!canEdit" variant="outline" class="text-muted-foreground">
                只读（无 rowid）
              </Badge>
            </div>
            <div class="flex shrink-0 items-center gap-2">
              <TabsList>
                <TabsTrigger value="data">数据</TabsTrigger>
                <TabsTrigger value="schema">结构</TabsTrigger>
                <TabsTrigger value="ddl">DDL</TabsTrigger>
              </TabsList>
              <Button size="sm" :disabled="!canEdit" @click="addOpen = true">
                <Plus class="size-4" />
                新增行
              </Button>
            </div>
          </div>

          <TabsContent value="data" class="mt-0 flex min-h-0 flex-col">
            <DataGrid
              :columns="schema?.columns ?? []"
              :rows="rows"
              :loading="rowsLoading"
              :readonly="!canEdit"
              :page="page"
              :page-count="pageCount"
              :total="total"
              @edit-cell="handleEditCell"
              @view-row="(row) => openRowDialog(row, 'view')"
              @edit-row="(row) => openRowDialog(row, 'edit')"
              @delete-row="askDeleteRow"
              @page-change="gotoPage"
            />
          </TabsContent>

          <TabsContent value="schema" class="mt-0 min-h-0 overflow-y-auto">
            <TableSchema :schema="schema" :loading="schemaLoading" />
          </TabsContent>

          <TabsContent value="ddl" class="mt-0 min-h-0 overflow-y-auto">
            <pre
              class="rounded-lg border border-border bg-console p-3 font-mono text-xs leading-5 text-console-foreground"
              >{{ schema?.ddl ?? '（未获取到 DDL）' }}</pre>
          </TabsContent>
        </Tabs>
      </section>
    </div>

    <!-- 新增行弹窗 -->
    <RowEditorDialog
      v-model:open="addOpen"
      :table="selected ?? ''"
      :columns="schema?.columns ?? []"
      @submit="handleInsert"
    />

    <!-- 行查看/编辑弹窗 -->
    <RowDetailDialog
      v-model:open="rowDialogOpen"
      :mode="rowDialogMode"
      :table="selected ?? ''"
      :columns="schema?.columns ?? []"
      :row="activeRow"
      @save="handleRowSave"
    />

    <!-- 删除行确认 -->
    <AlertDialog :open="deleteOpen" @update:open="(value) => (deleteOpen = value)">
      <AlertDialogContent class="sm:max-w-sm">
        <AlertDialogHeader>
          <AlertDialogTitle>删除该行？</AlertDialogTitle>
          <AlertDialogDescription>
            表「{{ selected }}」中 rowid 为
            {{ deletingRow?.__rid }} 的行将被永久删除，该操作不可撤销。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>取消</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmDeleteRow"
          >
            删除
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </ToolShell>
</template>
