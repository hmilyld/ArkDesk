<!--
  测算历史：归档记录的分页列表 + 详情对比 + 结果工作簿导出 + 删除。
  结果快照以 JSON 存于 tender_history，详情/导出时解析传回 Rust 命令。
-->
<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { Eye, FileDown, FileClock, RefreshCw, Trash2 } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  COMPANY_TYPE_LABELS,
  PAGE_SIZE,
  computeRecommended,
  formatRate,
  formatReduction,
  formatStd,
  formatWan,
  type CompanyInput,
  type ExportPayload,
  type ScenarioResult,
} from '../shared';
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
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
import { tenderHistory, type TenderHistory } from '../schema';
import ToolShell from '@/components/tool/ToolShell.vue';
import Panel from '@/components/tool/Panel.vue';
import EmptyState from '@/components/native/EmptyState.vue';
import LoadingState from '@/components/native/LoadingState.vue';

// ── 列表与分页（前端分页：全量拉取，本地切页） ────────────────
const history = ref<TenderHistory[]>([]);
const loading = ref(false);
const page = ref(1);

const pageCount = computed(() => Math.max(1, Math.ceil(history.value.length / PAGE_SIZE)));
const pageRows = computed(() =>
  history.value.slice((page.value - 1) * PAGE_SIZE, page.value * PAGE_SIZE)
);
const pageNumbers = computed(() =>
  Array.from({ length: pageCount.value }, (_, i) => i + 1).filter(
    (n) => pageCount.value <= 5 || n === 1 || n === pageCount.value || Math.abs(n - page.value) <= 2
  )
);

async function loadHistory(): Promise<void> {
  loading.value = true;
  try {
    history.value = await kdb.select().from(tenderHistory).orderBy(desc(tenderHistory.id));
    page.value = Math.min(page.value, pageCount.value);
  } finally {
    loading.value = false;
  }
}

function gotoPage(target: number): void {
  page.value = Math.min(Math.max(target, 1), pageCount.value);
}

// ── 详情弹窗 ───────────────────────────────────────────────────
interface HistoryDetail {
  row: TenderHistory;
  companies: CompanyInput[];
  results: ScenarioResult[];
}

const detail = ref<HistoryDetail | null>(null);

function openDetail(row: TenderHistory): void {
  try {
    detail.value = {
      row,
      companies: JSON.parse(row.companiesJson) as CompanyInput[],
      results: JSON.parse(row.resultsJson) as ScenarioResult[],
    };
  } catch (err) {
    const error = normalizeError(err);
    toast.error(`结果数据解析失败：${error.message}`);
  }
}

const detailBest = computed(() =>
  detail.value ? detail.value.results[detail.value.row.bestIndex] : null
);

const detailRecommended = computed(() => {
  if (!detail.value || !detailBest.value) return [];
  return computeRecommended(detail.value.companies, detailBest.value);
});

// ── 删除（AlertDialog 二次确认） ───────────────────────────────
// AlertDialogAction 点击时先触发弹窗关闭（update:open），再执行按钮的
// click 回调——确认行不能在 update:open 里清空，否则回调读到 null 短路。
// 因此关闭只翻转 deleteOpen，deletingRow 在删除完成后才清除。
const deleteOpen = ref(false);
const deletingRow = ref<TenderHistory | null>(null);

function askDelete(row: TenderHistory): void {
  deletingRow.value = row;
  deleteOpen.value = true;
}

async function confirmDelete(): Promise<void> {
  const row = deletingRow.value;
  if (!row) return;
  deleteOpen.value = false;
  try {
    await kdb.delete(tenderHistory).where(eq(tenderHistory.id, row.id));
  } finally {
    deletingRow.value = null;
    await loadHistory();
  }
}

// ── 导出结果工作簿 ─────────────────────────────────────────────
const exportingId = ref<number | null>(null);

/** 文件名安全化：替换路径非法字符 */
function safeFileName(name: string): string {
  return name.replace(/[\\/:*?"<>|]/g, '-');
}

async function exportRow(row: TenderHistory): Promise<void> {
  let payload: ExportPayload;
  try {
    payload = {
      companies: JSON.parse(row.companiesJson) as CompanyInput[],
      results: JSON.parse(row.resultsJson) as ScenarioResult[],
      best_index: row.bestIndex,
    };
  } catch (err) {
    const error = normalizeError(err);
    toast.error(`结果数据解析失败：${error.message}`);
    return;
  }

  const path = await saveFileDialog({
    defaultPath: `${row.planName ? safeFileName(row.planName) + '_' : ''}测算结果_${row.createdAt.replace(/[: ]/g, '-')}.xlsx`,
    filters: [{ name: 'Excel 工作簿', extensions: ['xlsx'] }],
  });
  if (!path) return;

  exportingId.value = row.id;
  try {
    await ipc<string>('tender_optimizer_export_result', { payload, path });
    toast.success('导出成功', { description: path });
  } catch (err) {
    const error = normalizeError(err);
    toast.error(`导出失败：${error.message}`, { description: error.code, duration: 6000 });
    logger.error(`结果导出失败: [${error.code}] ${error.message}`);
  } finally {
    exportingId.value = null;
  }
}

onMounted(loadHistory);
// KeepAlive 场景：从测算页切回时刷新（测算会新增历史记录）
onActivated(loadHistory);
</script>

<template>
  <ToolShell title="测算历史" description="每次运行测算自动归档，支持结果对比、工作簿导出与删除">
    <template #actions>
      <Button variant="outline" size="sm" :disabled="loading" @click="loadHistory">
        <RefreshCw class="size-3.5" :class="{ 'animate-spin': loading }" />
        刷新
      </Button>
    </template>

    <Panel
      title="测算记录"
      :hint="history.length ? `共 ${history.length} 条 · 每页 ${PAGE_SIZE} 条` : undefined"
      body-class="space-y-0 p-0"
    >
      <LoadingState v-if="loading && history.length === 0" class="p-4" :rows="6" />

      <EmptyState
        v-else-if="history.length === 0"
        :icon="FileClock"
        title="暂无测算记录"
        description="到「报价测算」页运行一次测算后会自动归档到这里"
      >
        <Button variant="outline" size="sm" @click="loadHistory">
          <RefreshCw class="size-3.5" />
          刷新
        </Button>
      </EmptyState>

      <template v-else>
        <Table>
          <TableHeader>
            <TableRow class="hover:bg-transparent">
              <TableHead class="w-14 pl-3">ID</TableHead>
              <TableHead class="max-w-36">方案名称</TableHead>
              <TableHead class="w-40">测算时间</TableHead>
              <TableHead class="w-24 text-right">公司数</TableHead>
              <TableHead class="w-24 text-right">场景数</TableHead>
              <TableHead>最优场景</TableHead>
              <TableHead class="w-32 text-right">目标报价 (万)</TableHead>
              <TableHead class="w-20 text-right">得分</TableHead>
              <TableHead class="w-32 pr-3 text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow v-for="row in pageRows" :key="row.id" class="hover:bg-accent/40">
              <TableCell class="pl-3 font-mono text-xs text-muted-foreground tabular-nums">
                {{ row.id }}
              </TableCell>
              <TableCell class="max-w-0 truncate">
                <span v-if="row.planName" class="font-medium">{{ row.planName }}</span>
                <span v-else class="text-xs text-muted-foreground">-</span>
              </TableCell>
              <TableCell class="font-mono text-xs text-muted-foreground tabular-nums">
                {{ row.createdAt }}
              </TableCell>
              <TableCell class="text-right font-mono text-xs tabular-nums">
                {{ row.numCompanies }}
              </TableCell>
              <TableCell class="text-right font-mono text-xs tabular-nums">
                {{ row.numScenarios }}
              </TableCell>
              <TableCell class="max-w-0 truncate">
                <span class="font-medium">{{ row.bestScenario ?? '-' }}</span>
              </TableCell>
              <TableCell class="text-right font-mono text-xs font-semibold tabular-nums">
                {{ formatWan(row.targetPrice, 4) }}
              </TableCell>
              <TableCell class="text-right font-mono text-xs text-success tabular-nums">
                {{ formatWan(row.bestScore) }}
              </TableCell>
              <TableCell class="pr-3 text-right">
                <div class="inline-flex items-center gap-0.5">
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="查看详情"
                    title="查看详情"
                    @click="openDetail(row)"
                  >
                    <Eye class="size-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="导出结果"
                    title="导出结果"
                    :disabled="exportingId === row.id"
                    @click="exportRow(row)"
                  >
                    <FileDown class="size-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="删除记录"
                    title="删除记录"
                    @click="askDelete(row)"
                  >
                    <Trash2 class="size-4 text-muted-foreground" />
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>

        <!-- 分页 -->
        <div class="flex items-center justify-between gap-3 border-t px-4 py-2.5">
          <p class="text-xs text-muted-foreground tabular-nums">
            共 {{ history.length }} 条 · 每页 {{ PAGE_SIZE }} 条
          </p>
          <div class="flex items-center gap-1.5">
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
                class="min-w-8 px-2 font-mono tabular-nums"
                :class="
                  n === page
                    ? 'border-primary/50 bg-primary/10 text-primary'
                    : 'text-muted-foreground'
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
        </div>
      </template>
    </Panel>

    <!-- 详情弹窗：最优方案 + 场景对比 + 公司推荐 -->
    <Dialog :open="detail !== null" @update:open="(v) => (detail = v ? detail : null)">
      <DialogContent class="sm:max-w-3xl">
        <template v-if="detail && detailBest">
          <DialogHeader>
            <DialogTitle>
              测算详情 #{{ detail.row.id }}
              <Badge v-if="detail.row.planName" variant="outline" class="ml-1 align-middle">
                {{ detail.row.planName }}
              </Badge>
            </DialogTitle>
            <DialogDescription>
              {{ detail.row.createdAt }} · {{ detail.row.numCompanies }} 家公司 ·
              {{ detail.row.numScenarios }} 个场景 · {{ detail.row.numSimulations }} 次模拟
            </DialogDescription>
          </DialogHeader>

          <div class="max-h-[65vh] space-y-4 overflow-y-auto py-1">
            <!-- 最优方案：inset 底，不叠盒子 -->
            <div class="rounded-md bg-muted p-3">
              <div class="mb-2 flex items-center gap-2">
                <Badge>最优方案</Badge>
                <span class="text-sm font-medium">{{ detailBest.scenario_name }}</span>
              </div>
              <div class="grid grid-cols-2 gap-3 md:grid-cols-4">
                <div>
                  <p class="text-xs text-muted-foreground">目标报价</p>
                  <p class="font-mono text-lg font-semibold tabular-nums">
                    {{ detailBest.target_price.toFixed(4) }} 万
                  </p>
                </div>
                <div>
                  <p class="text-xs text-muted-foreground">预期得分</p>
                  <p class="font-mono text-lg font-semibold tabular-nums text-success">
                    {{ detailBest.target_score.toFixed(2) }}
                  </p>
                </div>
                <div>
                  <p class="text-xs text-muted-foreground">基准价</p>
                  <p class="font-mono text-lg font-semibold tabular-nums">
                    {{ detailBest.base_price.toFixed(2) }} 万
                  </p>
                </div>
                <div>
                  <p class="text-xs text-muted-foreground">预测均价 / 搜索范围</p>
                  <p class="font-mono text-lg font-semibold tabular-nums">
                    {{ detailBest.search_center.toFixed(2) }}
                    <span class="text-sm text-muted-foreground">
                      ±{{ detailBest.search_range.toFixed(2) }}
                    </span>
                  </p>
                </div>
              </div>
            </div>

            <!-- 场景对比：行分隔走 divide-y，不额外画盒子 -->
            <section>
              <h4 class="mb-1.5 text-xs font-medium text-muted-foreground">场景对比</h4>
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b text-left text-xs text-muted-foreground">
                    <th class="px-3 py-2 font-medium">场景</th>
                    <th class="px-3 py-2 font-medium">降幅</th>
                    <th class="px-3 py-2 font-medium">标准差</th>
                    <th class="px-3 py-2 text-right font-medium">参与率</th>
                    <th class="px-3 py-2 text-right font-medium">目标报价 (万)</th>
                    <th class="px-3 py-2 text-right font-medium">得分</th>
                    <th class="px-3 py-2 text-right font-medium">结果</th>
                  </tr>
                </thead>
                <tbody class="divide-y">
                  <tr
                    v-for="(r, index) in detail.results"
                    :key="r.scenario_name"
                    :class="index === detail.row.bestIndex ? 'bg-muted' : ''"
                  >
                    <td class="px-3 py-2 font-medium">{{ r.scenario_name }}</td>
                    <td class="px-3 py-2 font-mono text-xs tabular-nums">
                      {{ formatReduction(r.reduction_type, r.reduction_value) }}
                    </td>
                    <td class="px-3 py-2 text-xs text-muted-foreground">
                      {{ formatStd(r.reduction_type, r.std_dev) }}
                    </td>
                    <td class="px-3 py-2 text-right text-xs text-muted-foreground tabular-nums">
                      {{ formatRate(r.participation_rate) }}
                    </td>
                    <td class="px-3 py-2 text-right font-mono text-xs font-semibold tabular-nums">
                      {{ r.target_price.toFixed(4) }}
                    </td>
                    <td class="px-3 py-2 text-right font-mono text-xs tabular-nums">
                      {{ r.target_score.toFixed(2) }}
                    </td>
                    <td class="px-3 py-2 text-right">
                      <Badge v-if="index === detail.row.bestIndex" variant="default">最优</Badge>
                    </td>
                  </tr>
                </tbody>
              </table>
            </section>

            <!-- 公司推荐报价 -->
            <section>
              <h4 class="mb-1.5 text-xs font-medium text-muted-foreground">公司推荐报价</h4>
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b text-left text-xs text-muted-foreground">
                    <th class="px-3 py-2 font-medium">公司</th>
                    <th class="px-3 py-2 font-medium">类型</th>
                    <th class="px-3 py-2 text-right font-medium">第一轮报价</th>
                    <th class="px-3 py-2 text-right font-medium">推荐报价 (万)</th>
                    <th class="px-3 py-2 text-right font-medium">变化 (万)</th>
                  </tr>
                </thead>
                <tbody class="divide-y">
                  <tr v-for="rec in detailRecommended" :key="rec.index">
                    <td class="px-3 py-2">
                      {{ detail.companies[rec.index]?.name ?? '-' }}
                    </td>
                    <td class="px-3 py-2 text-xs text-muted-foreground">
                      {{ COMPANY_TYPE_LABELS[detail.companies[rec.index]?.company_type ?? 'U'] }}
                    </td>
                    <td class="px-3 py-2 text-right font-mono text-xs tabular-nums">
                      {{ formatWan(detail.companies[rec.index]?.round1_price) }}
                    </td>
                    <td class="px-3 py-2 text-right font-mono text-xs tabular-nums">
                      {{ rec.recommended_price === null ? '-' : rec.recommended_price.toFixed(4) }}
                    </td>
                    <td
                      class="px-3 py-2 text-right font-mono text-xs tabular-nums"
                      :class="(rec.change ?? 0) < 0 ? 'text-success' : 'text-muted-foreground'"
                    >
                      {{ rec.change === null ? '-' : rec.change.toFixed(4) }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </section>
          </div>
        </template>
      </DialogContent>
    </Dialog>

    <!-- 删除确认 -->
    <AlertDialog :open="deleteOpen" @update:open="(value) => (deleteOpen = value)">
      <AlertDialogContent class="sm:max-w-sm">
        <AlertDialogHeader>
          <AlertDialogTitle>删除测算记录？</AlertDialogTitle>
          <AlertDialogDescription>
            记录 #{{ deletingRow?.id }}（{{
              deletingRow?.bestScenario
            }}）将被永久删除，该操作不可撤销。
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
