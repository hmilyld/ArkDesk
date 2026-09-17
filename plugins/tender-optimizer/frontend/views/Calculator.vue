<!--
  报价测算：导入统一模板 → 编辑公司 / 场景 / 参数 → 蒙特卡洛测算 → 结果展示。
  覆盖：Excel 导入导出（Rust 命令 + 文件对话框）、行内编辑即时落库、
  结果对比表（最优行高亮）、测算历史自动归档。
-->
<script setup lang="ts">
import { computed, onActivated, onMounted, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { save as saveFileDialog, open as openFileDialog } from '@tauri-apps/plugin-dialog';
import {
  Building2,
  Download,
  FlaskConical,
  Pencil,
  Play,
  Plus,
  RotateCcw,
  Trash2,
  Upload,
  X,
} from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Checkbox } from '@/components/ui/checkbox';
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
  DialogFooter,
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
import { asc, eq } from 'drizzle-orm';
import {
  tenderCompanies,
  tenderParams,
  tenderHistory,
  tenderScenarios,
  type TenderCompany,
  type TenderParams,
  type TenderScenario,
} from '../schema';
import {
  BEHAVIOR_OPTIONS,
  COMPANY_TYPE_LABELS,
  COMPANY_TYPE_OPTIONS,
  DEFAULT_NUM_SIMULATIONS,
  DEFAULT_PARAMS,
  REDUCTION_TYPE_OPTIONS,
  formatRate,
  formatReduction,
  formatStd,
  formatWan,
  paramsToWire,
  toCompanyInput,
  toScenarioInput,
  type CalculateResponse,
  type ImportResult,
  type ReductionType,
} from '../shared';
import ToolShell from '@/components/tool/ToolShell.vue';
import Panel from '@/components/tool/Panel.vue';
import FormRow from '@/components/native/FormRow.vue';
import Segmented from '@/components/native/Segmented.vue';
import EmptyState from '@/components/native/EmptyState.vue';
import LoadingState from '@/components/native/LoadingState.vue';
import ErrorState from '@/components/native/ErrorState.vue';

// ── 工作集加载 ─────────────────────────────────────────────────
const loading = ref(false);
const companies = ref<TenderCompany[]>([]);
const scenarios = ref<TenderScenario[]>([]);
const paramsRow = ref<TenderParams | null>(null);

const activeScenarioCount = computed(() => scenarios.value.filter((s) => s.isActive).length);
const targetCount = computed(() => companies.value.filter((c) => c.companyType === 'T').length);

// ── 就地错误 / 提示（docs/design.md §4：校验错误就地呈现，不用 toast） ──
const formError = ref('');
const calcError = ref('');
const importNotice = ref<{ kind: 'info' | 'warning' | 'error'; text: string } | null>(null);
const noticeClass = computed(() => {
  const kind = importNotice.value?.kind;
  if (kind === 'error') return 'border-destructive/30 bg-destructive/10 text-destructive';
  if (kind === 'warning') return 'border-warning/30 bg-warning/10 text-warning';
  return 'border-info/30 bg-info/10 text-info';
});

// 数据变化后校验通过即清除顶部错误条
watch([() => companies.value.length, targetCount, activeScenarioCount], () => {
  if (formError.value && !validateLocal()) formError.value = '';
});

async function loadAll(): Promise<void> {
  loading.value = true;
  try {
    companies.value = await kdb
      .select()
      .from(tenderCompanies)
      .orderBy(asc(tenderCompanies.sortOrder), asc(tenderCompanies.id));
    scenarios.value = await kdb
      .select()
      .from(tenderScenarios)
      .orderBy(asc(tenderScenarios.sortOrder), asc(tenderScenarios.id));
    const rows = await kdb.select().from(tenderParams);
    paramsRow.value = rows[0] ?? null;
  } finally {
    loading.value = false;
  }
}

// ── 枚举控件的分段选项（docs/design.md §3：2–5 项互斥用 Segmented） ──
const COMPANY_TYPE_SEGMENTS = COMPANY_TYPE_OPTIONS.map((option) => ({
  value: option.value,
  label: COMPANY_TYPE_LABELS[option.value],
}));
const BEHAVIOR_SEGMENTS = [{ value: '', label: '未设置' }, ...BEHAVIOR_OPTIONS];
const REDUCTION_TYPE_SEGMENTS = REDUCTION_TYPE_OPTIONS.map((option) => ({
  value: option.value,
  label: option.label,
}));

// ── 公司工作集：行内编辑 + 即时落库 ────────────────────────────
async function persistCompany(row: TenderCompany): Promise<void> {
  if (row.id === null || row.id === undefined) return;
  await kdb
    .update(tenderCompanies)
    .set({
      name: row.name,
      round1Price: Number(row.round1Price) || 0,
      priceLimit: Number(row.priceLimit) || 0,
      companyType: row.companyType,
      behavior: row.behavior,
    })
    .where(eq(tenderCompanies.id, row.id));
}

function setCompanyType(row: TenderCompany, value: string): void {
  row.companyType = value;
  void persistCompany(row);
}

function setBehavior(row: TenderCompany, value: string): void {
  row.behavior = value === '' ? null : value;
  void persistCompany(row);
}

async function addCompany(): Promise<void> {
  // 新行插到表格第一行：sort_order 取现有最小值减 1（升序展示）
  const topOrder =
    companies.value.length > 0 ? Math.min(...companies.value.map((c) => c.sortOrder)) - 1 : 0;
  const [created] = await kdb
    .insert(tenderCompanies)
    .values({
      name: `公司${companies.value.length + 1}`,
      round1Price: 1,
      priceLimit: 1,
      companyType: 'U',
      behavior: null,
      sortOrder: topOrder,
    })
    .returning();
  if (created) companies.value.unshift(created);
}

async function removeCompany(row: TenderCompany): Promise<void> {
  if (row.id === null || row.id === undefined) return;
  await kdb.delete(tenderCompanies).where(eq(tenderCompanies.id, row.id));
  companies.value = companies.value.filter((c) => c.id !== row.id);
}

// ── 场景工作集：行内编辑 + 即时落库 ────────────────────────────
async function persistScenario(row: TenderScenario): Promise<void> {
  if (row.id === null || row.id === undefined) return;
  await kdb
    .update(tenderScenarios)
    .set({
      name: row.name,
      reductionType: row.reductionType,
      reductionValue: Number(row.reductionValue) || 0,
      participationRate: Number(row.participationRate) || 0,
      stdDev: Number(row.stdDev) || 0,
      isActive: row.isActive,
    })
    .where(eq(tenderScenarios.id, row.id));
}

function setReductionType(row: TenderScenario, value: string): void {
  row.reductionType = value;
  void persistScenario(row);
}

function setScenarioActive(row: TenderScenario, value: boolean | 'indeterminate'): void {
  row.isActive = value === true;
  void persistScenario(row);
}

async function addScenario(): Promise<void> {
  // 新行插到表格第一行：sort_order 取现有最小值减 1（升序展示）
  const topOrder =
    scenarios.value.length > 0 ? Math.min(...scenarios.value.map((s) => s.sortOrder)) - 1 : 0;
  const [created] = await kdb
    .insert(tenderScenarios)
    .values({
      name: `场景${scenarios.value.length + 1}`,
      reductionType: 'percent',
      reductionValue: -0.05,
      participationRate: 0.8,
      stdDev: 0.015,
      isActive: true,
      sortOrder: topOrder,
    })
    .returning();
  if (created) scenarios.value.unshift(created);
}

async function removeScenario(row: TenderScenario): Promise<void> {
  if (row.id === null || row.id === undefined) return;
  await kdb.delete(tenderScenarios).where(eq(tenderScenarios.id, row.id));
  scenarios.value = scenarios.value.filter((s) => s.id !== row.id);
}

// ── 清空工作集：删除全部公司与场景，保留评分参数 ───────────────
const clearConfirmOpen = ref(false);

async function clearWorkset(): Promise<void> {
  clearConfirmOpen.value = false;
  await kdb.delete(tenderCompanies);
  await kdb.delete(tenderScenarios);
  companies.value = [];
  scenarios.value = [];
  results.value = null;
  if (paramsRow.value) {
    paramsRow.value.planName = '';
    await persistParams();
  }
}

// ── 评分参数 ───────────────────────────────────────────────────
const PARAM_FIELDS: { key: keyof TenderParams; label: string }[] = [
  { key: 'w1', label: 'W1（区间下限）' },
  { key: 'w2', label: 'W2（区间上限）' },
  { key: 'c', label: 'C（基准价浮动系数）' },
  { key: 'n1', label: 'n1（高于基准价惩罚）' },
  { key: 'n2', label: 'n2（低于基准价惩罚）' },
  { key: 'aggressiveFactor', label: '激进型降幅系数' },
  { key: 'normalFactor', label: '正常型降幅系数' },
  { key: 'conservativeFactor', label: '保守型降幅系数' },
  { key: 'minAuxPriceDiff', label: '辅助公司最小价差（万）' },
];

async function persistParams(): Promise<void> {
  const row = paramsRow.value;
  if (!row) return;
  await kdb
    .update(tenderParams)
    .set({ ...row, id: 1 })
    .where(eq(tenderParams.id, 1));
}

async function resetParams(): Promise<void> {
  const row = paramsRow.value;
  if (!row) return;
  const d = DEFAULT_PARAMS;
  row.w1 = d.w1;
  row.w2 = d.w2;
  row.c = d.c;
  row.n1 = d.n1;
  row.n2 = d.n2;
  row.aggressiveFactor = d.aggressive_factor;
  row.normalFactor = d.normal_factor;
  row.conservativeFactor = d.conservative_factor;
  row.minAuxPriceDiff = d.min_aux_price_diff;
  await persistParams();
}

// ── 模板导入 / 下载 ────────────────────────────────────────────
const importing = ref(false);
const importConfirmOpen = ref(false);
const importDialogOpen = ref(false);
const importPath = ref('');
const importPlanName = ref('');

/** 选完文件后弹出命名框：默认取文件名（去扩展名）作为方案名称 */
async function pickImportFile(): Promise<void> {
  importConfirmOpen.value = false;
  const path = await openFileDialog({
    multiple: false,
    directory: false,
    filters: [{ name: 'Excel 模板', extensions: ['xlsx'] }],
  });
  if (!path) return;
  importPath.value = path;
  const fileName = path.split(/[\\/]/).pop() ?? '';
  importPlanName.value = fileName.replace(/\.xlsx$/i, '');
  importDialogOpen.value = true;
}

async function runImport(): Promise<void> {
  importDialogOpen.value = false;
  importing.value = true;
  importNotice.value = null;
  try {
    const result = await ipc<ImportResult>('tender_optimizer_import_template', {
      path: importPath.value,
      // Tauri v2 命令参数默认 camelCase：plan_name → planName
      planName: importPlanName.value.trim(),
    });
    await loadAll();
    const summary = `导入完成：公司 ${result.companies.length} 家、场景 ${result.scenarios.length} 个`;
    if (result.warnings.length > 0) {
      importNotice.value = {
        kind: 'warning',
        text: `${summary}；另有 ${result.warnings.length} 条告警：${result.warnings[0]}`,
      };
      logger.warn(`模板导入警告: ${result.warnings.join('；')}`);
    } else {
      importNotice.value = { kind: 'info', text: summary };
    }
  } catch (err) {
    const error = normalizeError(err);
    importNotice.value = { kind: 'error', text: `导入失败：${error.message}` };
    logger.error(`模板导入失败: [${error.code}] ${error.message}`);
  } finally {
    importing.value = false;
  }
}

const downloading = ref(false);

async function downloadTemplate(): Promise<void> {
  const path = await saveFileDialog({
    defaultPath: '招标报价模板.xlsx',
    filters: [{ name: 'Excel 模板', extensions: ['xlsx'] }],
  });
  if (!path) return;
  downloading.value = true;
  try {
    await ipc<string>('tender_optimizer_download_template', { path });
    toast.success('模板已另存', { description: path });
  } catch (err) {
    const error = normalizeError(err);
    toast.error(`保存失败：${error.message}`, { description: error.code });
  } finally {
    downloading.value = false;
  }
}

// ── 方案名称（导入时指定，可就地重命名；随测算归档到历史） ─────
const editingPlanName = ref(false);
const planNameDraft = ref('');

function startEditPlanName(): void {
  planNameDraft.value = paramsRow.value?.planName ?? '';
  editingPlanName.value = true;
}

async function savePlanName(): Promise<void> {
  if (!paramsRow.value) return;
  paramsRow.value.planName = planNameDraft.value.trim();
  await persistParams();
  editingPlanName.value = false;
}

// ── 运行测算 ───────────────────────────────────────────────────
const calculating = ref(false);
const results = ref<CalculateResponse | null>(null);

const best = computed(() =>
  results.value ? results.value.results[results.value.best_index] : null
);

function recommendedOf(index: number): { price: number | null; change: number | null } {
  const rec = results.value?.recommended[index];
  return { price: rec?.recommended_price ?? null, change: rec?.change ?? null };
}

function validateLocal(): string | null {
  if (companies.value.length === 0) return '请先添加公司数据';
  if (targetCount.value !== 1) {
    return `必须有且仅有一家目标公司（T），当前 ${targetCount.value} 家`;
  }
  if (activeScenarioCount.value === 0) return '至少需要一个启用的测算场景';
  return null;
}

async function runCalculate(): Promise<void> {
  const problem = validateLocal();
  if (problem) {
    formError.value = problem;
    return;
  }

  formError.value = '';
  calcError.value = '';
  calculating.value = true;
  try {
    const params = paramsRow.value ? paramsToWire(paramsRow.value) : { ...DEFAULT_PARAMS };
    const numSimulations = paramsRow.value?.numSimulations ?? DEFAULT_NUM_SIMULATIONS;
    // await 前快照：发送载荷与历史归档必须同源，避免测算期间编辑导致漂移
    const companyInputs = companies.value.map(toCompanyInput);
    const scenarioInputs = scenarios.value.map(toScenarioInput);

    const response = await ipc<CalculateResponse>('tender_optimizer_calculate', {
      companies: companyInputs,
      scenarios: scenarioInputs,
      params,
      // Tauri v2 命令参数默认 camelCase：num_simulations → numSimulations
      numSimulations: numSimulations,
    });
    results.value = response;

    // 归档到历史（列表页摘要列 + 结果 JSON 快照）
    const bestResult = response.results[response.best_index];
    await kdb.insert(tenderHistory).values({
      planName: paramsRow.value?.planName ?? '',
      numCompanies: companyInputs.length,
      numScenarios: response.results.length,
      numSimulations: numSimulations,
      bestIndex: response.best_index,
      bestScore: bestResult.target_score,
      bestScenario: bestResult.scenario_name,
      targetPrice: bestResult.target_price,
      basePrice: bestResult.base_price,
      companiesJson: JSON.stringify(companyInputs),
      scenariosJson: JSON.stringify(scenarioInputs),
      paramsJson: JSON.stringify(params),
      resultsJson: JSON.stringify(response.results),
    });
  } catch (err) {
    const error = normalizeError(err);
    // 当前操作失败但有明确重试入口 → 就地错误条（docs/design.md §4）
    calcError.value = `测算失败：${error.message}`;
    logger.error(`测算失败: [${error.code}] ${error.message}`);
  } finally {
    calculating.value = false;
  }
}

onMounted(loadAll);
// KeepAlive 场景：从历史页切回时刷新工作集
onActivated(loadAll);
</script>

<template>
  <ToolShell
    title="报价测算"
    description="导入「招标报价模板.xlsx」或直接编辑，蒙特卡洛模拟 100 次取中位数推荐报价"
  >
    <template #actions>
      <Button variant="outline" size="sm" :disabled="downloading" @click="downloadTemplate">
        <Download class="size-3.5" />
        下载模板
      </Button>
      <Button variant="outline" size="sm" :disabled="importing" @click="importConfirmOpen = true">
        <Upload class="size-3.5" />
        {{ importing ? '导入中…' : '导入模板' }}
      </Button>
      <Button variant="outline" size="sm" :disabled="loading" @click="clearConfirmOpen = true">
        <Trash2 class="size-3.5" />
        清空工作集
      </Button>
      <Button size="sm" :disabled="calculating || loading" @click="runCalculate">
        <Play class="size-3.5" />
        {{ calculating ? '测算中…' : '运行测算' }}
      </Button>
    </template>

    <LoadingState v-if="loading" :rows="6" />

    <div v-else class="space-y-4">
      <!-- 就地错误条（校验 / 测算失败可重试） -->
      <ErrorState v-if="formError" :message="formError" />
      <ErrorState v-if="calcError" :message="calcError" :on-retry="runCalculate" />

      <!-- 导入结果 / 告警条 -->
      <div
        v-if="importNotice"
        class="flex items-start justify-between gap-3 rounded-md border px-3 py-2 text-xs"
        :class="noticeClass"
        role="status"
      >
        <span class="min-w-0">{{ importNotice.text }}</span>
        <Button
          variant="ghost"
          size="icon-xs"
          aria-label="关闭提示"
          title="关闭提示"
          @click="importNotice = null"
        >
          <X class="size-3.5" />
        </Button>
      </div>

      <!-- 当前方案：导入时命名，可就地修改，名称随测算归档到历史 -->
      <Panel v-if="paramsRow" title="方案">
        <template #actions>
          <Button v-if="!editingPlanName" variant="ghost" size="sm" @click="startEditPlanName">
            <Pencil class="size-3.5" />
            重命名
          </Button>
        </template>

        <FormRow label="方案名称" description="名称会显示在页头，并随每次测算归档到历史">
          <div class="flex items-center gap-2">
            <Input
              v-if="editingPlanName"
              v-model="planNameDraft"
              class="h-8 max-w-xs"
              placeholder="例如：XX 项目二轮报价"
              @keydown.enter="savePlanName"
            />
            <span v-else class="text-sm">{{ paramsRow.planName || '未命名方案' }}</span>
            <Button v-if="editingPlanName" variant="secondary" size="sm" @click="savePlanName">
              保存
            </Button>
            <Button
              v-if="editingPlanName"
              variant="ghost"
              size="sm"
              @click="editingPlanName = false"
            >
              取消
            </Button>
          </div>
        </FormRow>
      </Panel>

      <!-- 公司数据 -->
      <Panel
        title="公司数据"
        :hint="`${companies.length} 家 · 目标 ${targetCount} 家 · 需有且仅有 1 家目标公司`"
        body-class="space-y-0 p-0"
      >
        <template #actions>
          <Button variant="outline" size="sm" @click="addCompany">
            <Plus class="size-3.5" />
            添加公司
          </Button>
        </template>

        <EmptyState
          v-if="companies.length === 0"
          :icon="Building2"
          title="暂无公司数据"
          description="导入「招标报价模板.xlsx」，或手动添加公司；需有且仅有 1 家目标公司（T）"
        >
          <Button variant="outline" size="sm" @click="addCompany">
            <Plus class="size-3.5" />
            添加公司
          </Button>
        </EmptyState>

        <Table v-else>
          <TableHeader>
            <TableRow class="hover:bg-transparent">
              <TableHead class="min-w-36 pl-3">公司名称</TableHead>
              <TableHead class="w-32 text-right">第一轮报价 (万)</TableHead>
              <TableHead class="w-32 text-right">含税限价 (万)</TableHead>
              <TableHead class="w-44">类型</TableHead>
              <TableHead class="w-56">公司行为</TableHead>
              <template v-if="results">
                <TableHead class="w-32 text-right">推荐报价 (万)</TableHead>
                <TableHead class="w-24 text-right">变化 (万)</TableHead>
              </template>
              <TableHead class="w-14 pr-3 text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow v-for="(row, index) in companies" :key="row.id" class="hover:bg-accent/40">
              <TableCell class="pl-3">
                <Input
                  v-model="row.name"
                  class="h-8"
                  placeholder="公司名称"
                  @change="persistCompany(row)"
                />
              </TableCell>
              <TableCell>
                <Input
                  v-model.number="row.round1Price"
                  class="h-8 text-right font-mono tabular-nums"
                  type="number"
                  step="any"
                  @change="persistCompany(row)"
                />
              </TableCell>
              <TableCell>
                <Input
                  v-model.number="row.priceLimit"
                  class="h-8 text-right font-mono tabular-nums"
                  type="number"
                  step="any"
                  @change="persistCompany(row)"
                />
              </TableCell>
              <TableCell>
                <Segmented
                  size="sm"
                  :model-value="row.companyType"
                  :segments="COMPANY_TYPE_SEGMENTS"
                  @update:model-value="(value) => setCompanyType(row, value)"
                />
              </TableCell>
              <TableCell>
                <Segmented
                  size="sm"
                  :model-value="row.behavior ?? ''"
                  :segments="BEHAVIOR_SEGMENTS"
                  @update:model-value="(value) => setBehavior(row, value)"
                />
              </TableCell>
              <template v-if="results">
                <TableCell class="text-right font-mono text-xs tabular-nums">
                  {{
                    recommendedOf(index).price === null
                      ? '-'
                      : recommendedOf(index).price?.toFixed(4)
                  }}
                </TableCell>
                <TableCell
                  class="text-right font-mono text-xs tabular-nums"
                  :class="
                    (recommendedOf(index).change ?? 0) < 0
                      ? 'text-success'
                      : 'text-muted-foreground'
                  "
                >
                  {{
                    recommendedOf(index).change === null
                      ? '-'
                      : recommendedOf(index).change?.toFixed(4)
                  }}
                </TableCell>
              </template>
              <TableCell class="pr-3 text-right">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="删除公司"
                  title="删除公司"
                  @click="removeCompany(row)"
                >
                  <Trash2 class="size-4 text-muted-foreground" />
                </Button>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </Panel>

      <!-- 测算场景 -->
      <Panel
        title="测算场景"
        :hint="`${scenarios.length} 个 · 启用 ${activeScenarioCount} 个`"
        body-class="space-y-0 p-0"
      >
        <template #actions>
          <Button variant="outline" size="sm" @click="addScenario">
            <Plus class="size-3.5" />
            添加场景
          </Button>
        </template>

        <EmptyState
          v-if="scenarios.length === 0"
          :icon="FlaskConical"
          title="暂无测算场景"
          description="添加场景后设置降价方式与幅度；降价数值为负数表示降价"
        >
          <Button variant="outline" size="sm" @click="addScenario">
            <Plus class="size-3.5" />
            添加场景
          </Button>
        </EmptyState>

        <Table v-else>
          <TableHeader>
            <TableRow class="hover:bg-transparent">
              <TableHead class="min-w-32 pl-3">场景名称</TableHead>
              <TableHead class="w-40">降价方式</TableHead>
              <TableHead class="w-32 text-right">降价数值</TableHead>
              <TableHead class="w-28 text-right">参与率 (0-1)</TableHead>
              <TableHead class="w-28 text-right">标准差</TableHead>
              <TableHead class="w-16 text-center">有效</TableHead>
              <TableHead class="w-14 pr-3 text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow v-for="row in scenarios" :key="row.id" class="hover:bg-accent/40">
              <TableCell class="pl-3">
                <Input v-model="row.name" class="h-8" @change="persistScenario(row)" />
              </TableCell>
              <TableCell>
                <Segmented
                  size="sm"
                  :model-value="row.reductionType"
                  :segments="REDUCTION_TYPE_SEGMENTS"
                  @update:model-value="(value) => setReductionType(row, value)"
                />
              </TableCell>
              <TableCell>
                <Input
                  v-model.number="row.reductionValue"
                  class="h-8 text-right font-mono tabular-nums"
                  type="number"
                  step="any"
                  @change="persistScenario(row)"
                />
              </TableCell>
              <TableCell>
                <Input
                  v-model.number="row.participationRate"
                  class="h-8 text-right font-mono tabular-nums"
                  type="number"
                  step="any"
                  @change="persistScenario(row)"
                />
              </TableCell>
              <TableCell>
                <Input
                  v-model.number="row.stdDev"
                  class="h-8 text-right font-mono tabular-nums"
                  type="number"
                  step="any"
                  @change="persistScenario(row)"
                />
              </TableCell>
              <TableCell>
                <div class="flex justify-center">
                  <Checkbox
                    :model-value="row.isActive"
                    aria-label="启用该场景"
                    @update:model-value="(value) => setScenarioActive(row, value)"
                  />
                </div>
              </TableCell>
              <TableCell class="pr-3 text-right">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="删除场景"
                  title="删除场景"
                  @click="removeScenario(row)"
                >
                  <Trash2 class="size-4 text-muted-foreground" />
                </Button>
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </Panel>

      <!-- 评分参数 -->
      <Panel title="评分参数" hint="基准价 = 有效均价 × (1 − C)">
        <template #actions>
          <Button variant="ghost" size="sm" @click="resetParams">
            <RotateCcw class="size-3.5" />
            恢复默认
          </Button>
        </template>

        <div v-if="paramsRow" class="grid grid-cols-2 gap-4 md:grid-cols-3 lg:grid-cols-5">
          <FormRow v-for="field in PARAM_FIELDS" :key="field.key" :label="field.label">
            <template #default="{ id }">
              <Input
                :id="id"
                v-model.number="paramsRow[field.key]"
                class="h-8 text-right font-mono tabular-nums"
                type="number"
                step="any"
                @change="persistParams"
              />
            </template>
          </FormRow>
          <FormRow label="模拟次数">
            <template #default="{ id }">
              <Input
                :id="id"
                v-model.number="paramsRow.numSimulations"
                class="h-8 text-right font-mono tabular-nums"
                type="number"
                :min="1"
                :max="2000"
                @change="persistParams"
              />
            </template>
          </FormRow>
        </div>
      </Panel>

      <!-- 测算结果 -->
      <Panel
        v-if="results && best"
        title="测算结果"
        :hint="`${best.num_simulations} 次模拟 · 各场景取中位数`"
        body-class="space-y-0 p-0"
      >
        <div class="space-y-3 p-4">
          <div class="flex flex-wrap items-center gap-2">
            <Badge>最优方案</Badge>
            <span class="text-sm font-medium">{{ best.scenario_name }}</span>
            <span class="text-xs text-muted-foreground">
              降价 {{ formatReduction(best.reduction_type as ReductionType, best.reduction_value) }}
              {{ formatStd(best.reduction_type as ReductionType, best.std_dev) }} · 参与
              {{ formatRate(best.participation_rate) }}
            </span>
          </div>

          <div class="grid grid-cols-2 gap-4 md:grid-cols-4">
            <div>
              <p class="text-xs text-muted-foreground">推荐目标报价</p>
              <p class="font-mono text-lg font-semibold tabular-nums">
                {{ best.target_price.toFixed(4) }} 万
              </p>
            </div>
            <div>
              <p class="text-xs text-muted-foreground">预期得分</p>
              <p class="font-mono text-lg font-semibold tabular-nums text-success">
                {{ best.target_score.toFixed(2) }}
              </p>
            </div>
            <div>
              <p class="text-xs text-muted-foreground">基准价</p>
              <p class="font-mono text-lg font-semibold tabular-nums">
                {{ best.base_price.toFixed(2) }} 万
              </p>
            </div>
            <div>
              <p class="text-xs text-muted-foreground">预测均价 / 搜索范围</p>
              <p class="font-mono text-lg font-semibold tabular-nums">
                {{ best.search_center.toFixed(2) }}
                <span class="text-sm text-muted-foreground"
                  >±{{ best.search_range.toFixed(2) }}</span
                >
              </p>
            </div>
          </div>
        </div>

        <!-- 场景对比表：表头粘性、行分隔走表格自带 hairline -->
        <div class="max-h-96 overflow-y-auto border-t">
          <Table>
            <TableHeader class="sticky top-0 z-10 bg-card">
              <TableRow class="hover:bg-transparent">
                <TableHead class="pl-3">场景</TableHead>
                <TableHead class="w-28">降幅</TableHead>
                <TableHead class="w-24">标准差</TableHead>
                <TableHead class="w-20 text-right">参与率</TableHead>
                <TableHead class="w-28 text-right">预测均价</TableHead>
                <TableHead class="w-32 text-right">目标报价 (万)</TableHead>
                <TableHead class="w-24 text-right">基准价</TableHead>
                <TableHead class="w-20 text-right">得分</TableHead>
                <TableHead class="w-16 pr-3 text-right">结果</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow
                v-for="(r, index) in results.results"
                :key="r.scenario_name"
                :class="index === results.best_index ? 'bg-muted' : ''"
                class="hover:bg-accent/40"
              >
                <TableCell class="pl-3 font-medium">{{ r.scenario_name }}</TableCell>
                <TableCell class="font-mono text-xs tabular-nums">
                  {{ formatReduction(r.reduction_type, r.reduction_value) }}
                </TableCell>
                <TableCell class="text-xs text-muted-foreground">
                  {{ formatStd(r.reduction_type, r.std_dev) }}
                </TableCell>
                <TableCell class="text-right text-xs text-muted-foreground tabular-nums">
                  {{ formatRate(r.participation_rate) }}
                </TableCell>
                <TableCell class="text-right font-mono text-xs tabular-nums">
                  {{ r.search_center.toFixed(2) }}
                </TableCell>
                <TableCell class="text-right font-mono text-xs font-semibold tabular-nums">
                  {{ r.target_price.toFixed(4) }}
                </TableCell>
                <TableCell class="text-right font-mono text-xs tabular-nums">
                  {{ r.base_price.toFixed(2) }}
                </TableCell>
                <TableCell class="text-right font-mono text-xs tabular-nums">
                  {{ r.target_score.toFixed(2) }}
                </TableCell>
                <TableCell class="pr-3 text-right">
                  <Badge v-if="index === results.best_index" variant="default">最优</Badge>
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </div>

        <p class="border-t px-4 py-2 text-xs text-muted-foreground">
          共 {{ best.num_simulations }} 次模拟 · 各场景取中位数推荐 · 步长
          {{ formatWan(best.search_step, 4) }} 万
        </p>
      </Panel>
    </div>

    <!-- 导入确认（导入将整体替换工作集） -->
    <AlertDialog :open="importConfirmOpen" @update:open="(v) => (importConfirmOpen = v)">
      <AlertDialogContent class="sm:max-w-sm">
        <AlertDialogHeader>
          <AlertDialogTitle>导入模板？</AlertDialogTitle>
          <AlertDialogDescription>
            导入将整体替换当前的公司、场景与评分参数工作集，该操作不可撤销。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>取消</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="pickImportFile"
          >
            选择文件并导入
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>

    <!-- 导入命名：默认取文件名，名称将随测算归档到历史 -->
    <Dialog :open="importDialogOpen" @update:open="(v) => (importDialogOpen = v)">
      <DialogContent class="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>导入模板</DialogTitle>
          <DialogDescription>
            {{ importPath.split(/[\\/]/).pop() }} · 导入将替换当前工作集
          </DialogDescription>
        </DialogHeader>
        <FormRow label="方案名称" required description="名称会显示在页头，并随每次测算记录到历史">
          <template #default="{ id }">
            <Input
              :id="id"
              v-model="importPlanName"
              placeholder="例如：XX 项目二轮报价"
              @keydown.enter="runImport"
            />
          </template>
        </FormRow>
        <DialogFooter>
          <Button variant="outline" size="sm" @click="importDialogOpen = false">取消</Button>
          <Button size="sm" :disabled="importing || !importPlanName.trim()" @click="runImport">
            {{ importing ? '导入中…' : '确认导入' }}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>

    <!-- 清空确认（清空公司与场景，保留评分参数） -->
    <AlertDialog :open="clearConfirmOpen" @update:open="(v) => (clearConfirmOpen = v)">
      <AlertDialogContent class="sm:max-w-sm">
        <AlertDialogHeader>
          <AlertDialogTitle>清空工作集？</AlertDialogTitle>
          <AlertDialogDescription>
            公司数据与测算场景将被全部删除（评分参数保留），该操作不可撤销。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>取消</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="clearWorkset"
          >
            清空
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </ToolShell>
</template>
