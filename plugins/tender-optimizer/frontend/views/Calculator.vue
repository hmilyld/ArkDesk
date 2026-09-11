<!--
  报价测算：导入统一模板 → 编辑公司 / 场景 / 参数 → 蒙特卡洛测算 → 结果展示。
  覆盖：Excel 导入导出（Rust 命令 + 文件对话框）、行内编辑即时落库、
  结果对比表（最优行高亮）、测算历史自动归档。
-->
<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue';
import { toast } from 'vue-sonner';
import { save as saveFileDialog, open as openFileDialog } from '@tauri-apps/plugin-dialog';
import { Download, FileText, Pencil, Play, Plus, RotateCcw, Trash2, Upload } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
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

// ── 工作集加载 ─────────────────────────────────────────────────
const loading = ref(false);
const companies = ref<TenderCompany[]>([]);
const scenarios = ref<TenderScenario[]>([]);
const paramsRow = ref<TenderParams | null>(null);

const activeScenarioCount = computed(() => scenarios.value.filter((s) => s.isActive).length);
const targetCount = computed(() => companies.value.filter((c) => c.companyType === 'T').length);

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
  toast.success('已清空公司与场景数据', { description: '评分参数保持不变' });
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
  toast.success('评分参数已恢复默认值');
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
  try {
    const result = await ipc<ImportResult>('tender_optimizer_import_template', {
      path: importPath.value,
      // Tauri v2 命令参数默认 camelCase：plan_name → planName
      planName: importPlanName.value.trim(),
    });
    await loadAll();
    toast.success(
      `导入成功：公司 ${result.companies.length} 家、场景 ${result.scenarios.length} 个`,
      {
        description:
          result.warnings.length > 0
            ? `警告 ${result.warnings.length} 条：${result.warnings[0]}`
            : result.plan_name || undefined,
        duration: 6000,
      }
    );
    if (result.warnings.length > 1) {
      logger.warn(`模板导入警告: ${result.warnings.join('；')}`);
    }
  } catch (err) {
    const error = normalizeError(err);
    toast.error(`导入失败：${error.message}`, { description: error.code, duration: 6000 });
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
  toast.success('方案名称已更新', { description: '后续测算将以此名称归档' });
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
    toast.error(problem);
    return;
  }

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

    toast.success('测算完成', {
      description: `最优场景「${bestResult.scenario_name}」，预期得分 ${bestResult.target_score.toFixed(2)}`,
    });
  } catch (err) {
    const error = normalizeError(err);
    toast.error(`测算失败：${error.message}`, { description: error.code, duration: 6000 });
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
      <Badge v-if="paramsRow?.planName" variant="outline" class="mr-1 max-w-48 truncate">
        {{ paramsRow.planName }}
      </Badge>
      <Button variant="outline" size="sm" :disabled="downloading" @click="downloadTemplate">
        <Download class="size-4" />
        下载模板
      </Button>
      <Button variant="secondary" size="sm" :disabled="importing" @click="importConfirmOpen = true">
        <Upload class="size-4" />
        {{ importing ? '导入中…' : '导入模板' }}
      </Button>
      <Button variant="outline" size="sm" :disabled="loading" @click="clearConfirmOpen = true">
        <Trash2 class="size-4" />
        清空
      </Button>
      <Button size="sm" :disabled="calculating || loading" @click="runCalculate">
        <Play class="size-4" />
        {{ calculating ? '测算中…' : '运行测算' }}
      </Button>
    </template>

    <div v-if="loading" class="py-10 text-center text-xs text-muted-foreground">加载中…</div>

    <template v-else>
      <!-- 当前方案：导入时命名，可就地修改，名称随测算归档到历史 -->
      <section
        v-if="paramsRow"
        class="mb-5 flex items-center gap-2.5 rounded-lg border border-border bg-card px-4 py-3"
      >
        <FileText class="size-4 shrink-0 text-muted-foreground" />
        <span class="shrink-0 text-xs text-muted-foreground">当前方案</span>
        <template v-if="editingPlanName">
          <Input
            v-model="planNameDraft"
            class="h-8 max-w-xs"
            placeholder="例如：XX 项目二轮报价"
            @keydown.enter="savePlanName"
          />
          <Button variant="outline" size="sm" @click="savePlanName">保存</Button>
          <Button variant="ghost" size="sm" @click="editingPlanName = false">取消</Button>
        </template>
        <template v-else>
          <span class="truncate text-sm font-medium">
            {{ paramsRow.planName || '未命名方案' }}
          </span>
          <Button variant="ghost" size="icon-sm" aria-label="重命名方案" @click="startEditPlanName">
            <Pencil class="size-3.5" />
          </Button>
          <span class="ml-auto shrink-0 text-xs text-muted-foreground">
            名称会随每次测算归档到历史
          </span>
        </template>
      </section>

      <!-- 公司数据 -->
      <section class="mb-5">
        <div class="mb-2 flex items-center justify-between">
          <h3 class="text-sm font-medium">
            公司数据
            <span class="ml-1.5 text-xs text-muted-foreground">
              {{ companies.length }} 家 · 目标 {{ targetCount }} 家 · 需有且仅有 1 家目标公司
            </span>
          </h3>
          <Button variant="outline" size="sm" @click="addCompany">
            <Plus class="size-4" />
            添加公司
          </Button>
        </div>
        <div class="overflow-hidden rounded-lg border border-border bg-card">
          <Table>
            <TableHeader>
              <TableRow class="hover:bg-transparent">
                <TableHead class="min-w-36 pl-3">公司名称</TableHead>
                <TableHead class="w-32">第一轮报价(万)</TableHead>
                <TableHead class="w-32">含税限价(万)</TableHead>
                <TableHead class="w-28">类型</TableHead>
                <TableHead class="w-28">公司行为</TableHead>
                <template v-if="results">
                  <TableHead class="w-28">推荐报价(万)</TableHead>
                  <TableHead class="w-24">变化(万)</TableHead>
                </template>
                <TableHead class="w-14 pr-3 text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow v-if="companies.length === 0">
                <TableCell colspan="100" class="py-8 text-center text-xs text-muted-foreground">
                  暂无公司数据，点右上角「导入模板」或「添加公司」
                </TableCell>
              </TableRow>
              <TableRow
                v-for="(row, index) in companies"
                :key="row.id"
                class="hover:bg-transparent"
              >
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
                    class="h-8 font-mono"
                    type="number"
                    step="any"
                    @change="persistCompany(row)"
                  />
                </TableCell>
                <TableCell>
                  <Input
                    v-model.number="row.priceLimit"
                    class="h-8 font-mono"
                    type="number"
                    step="any"
                    @change="persistCompany(row)"
                  />
                </TableCell>
                <TableCell>
                  <Select v-model="row.companyType" @update:model-value="persistCompany(row)">
                    <SelectTrigger class="h-8 w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem
                        v-for="option in COMPANY_TYPE_OPTIONS"
                        :key="option.value"
                        :value="option.value"
                      >
                        {{ option.label }}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </TableCell>
                <TableCell>
                  <Select v-model="row.behavior" @update:model-value="persistCompany(row)">
                    <SelectTrigger class="h-8 w-full">
                      <SelectValue placeholder="未设置（按正常型）" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem
                        v-for="option in BEHAVIOR_OPTIONS"
                        :key="option.value"
                        :value="option.value"
                      >
                        {{ option.label }}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </TableCell>
                <template v-if="results">
                  <TableCell class="font-mono text-xs">
                    {{
                      recommendedOf(index).price === null
                        ? '-'
                        : recommendedOf(index).price?.toFixed(4)
                    }}
                  </TableCell>
                  <TableCell
                    class="font-mono text-xs"
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
                    @click="removeCompany(row)"
                  >
                    <Trash2 class="size-4 text-muted-foreground" />
                  </Button>
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </div>
      </section>

      <!-- 测算场景 -->
      <section class="mb-5">
        <div class="mb-2 flex items-center justify-between">
          <h3 class="text-sm font-medium">
            测算场景
            <span class="ml-1.5 text-xs text-muted-foreground">
              {{ scenarios.length }} 个 · 启用 {{ activeScenarioCount }} 个
            </span>
          </h3>
          <Button variant="outline" size="sm" @click="addScenario">
            <Plus class="size-4" />
            添加场景
          </Button>
        </div>
        <div class="overflow-hidden rounded-lg border border-border bg-card">
          <Table>
            <TableHeader>
              <TableRow class="hover:bg-transparent">
                <TableHead class="min-w-32 pl-3">场景名称</TableHead>
                <TableHead class="w-28">降价方式</TableHead>
                <TableHead class="w-32">降价数值</TableHead>
                <TableHead class="w-28">参与率(0-1)</TableHead>
                <TableHead class="w-28">标准差</TableHead>
                <TableHead class="w-20">有效</TableHead>
                <TableHead class="w-14 pr-3 text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow v-if="scenarios.length === 0">
                <TableCell colspan="100" class="py-8 text-center text-xs text-muted-foreground">
                  暂无测算场景，点「添加场景」新建（降价数值为负数表示降价）
                </TableCell>
              </TableRow>
              <TableRow v-for="row in scenarios" :key="row.id" class="hover:bg-transparent">
                <TableCell class="pl-3">
                  <Input v-model="row.name" class="h-8" @change="persistScenario(row)" />
                </TableCell>
                <TableCell>
                  <Select v-model="row.reductionType" @update:model-value="persistScenario(row)">
                    <SelectTrigger class="h-8 w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem
                        v-for="option in REDUCTION_TYPE_OPTIONS"
                        :key="option.value"
                        :value="option.value"
                      >
                        {{ option.label }}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </TableCell>
                <TableCell>
                  <Input
                    v-model.number="row.reductionValue"
                    class="h-8 font-mono"
                    type="number"
                    step="any"
                    @change="persistScenario(row)"
                  />
                </TableCell>
                <TableCell>
                  <Input
                    v-model.number="row.participationRate"
                    class="h-8 font-mono"
                    type="number"
                    step="any"
                    @change="persistScenario(row)"
                  />
                </TableCell>
                <TableCell>
                  <Input
                    v-model.number="row.stdDev"
                    class="h-8 font-mono"
                    type="number"
                    step="any"
                    @change="persistScenario(row)"
                  />
                </TableCell>
                <TableCell>
                  <Switch v-model="row.isActive" @update:model-value="persistScenario(row)" />
                </TableCell>
                <TableCell class="pr-3 text-right">
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label="删除场景"
                    @click="removeScenario(row)"
                  >
                    <Trash2 class="size-4 text-muted-foreground" />
                  </Button>
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </div>
      </section>

      <!-- 评分参数 -->
      <section class="mb-5">
        <div class="mb-2 flex items-center justify-between">
          <h3 class="text-sm font-medium">
            评分参数
            <span class="ml-1.5 text-xs text-muted-foreground">基准价 = 有效均价 × (1 − C)</span>
          </h3>
          <Button variant="outline" size="sm" @click="resetParams">
            <RotateCcw class="size-4" />
            恢复默认
          </Button>
        </div>
        <div v-if="paramsRow" class="rounded-lg border border-border bg-card p-4">
          <div class="grid grid-cols-2 gap-x-6 gap-y-3 md:grid-cols-5">
            <div v-for="field in PARAM_FIELDS" :key="field.key" class="space-y-1">
              <Label :for="`param-${field.key}`" class="text-xs text-muted-foreground">
                {{ field.label }}
              </Label>
              <Input
                :id="`param-${field.key}`"
                v-model.number="paramsRow[field.key]"
                class="h-8 font-mono"
                type="number"
                step="any"
                @change="persistParams"
              />
            </div>
            <div class="space-y-1">
              <Label for="param-numSimulations" class="text-xs text-muted-foreground"
                >模拟次数</Label
              >
              <Input
                id="param-numSimulations"
                v-model.number="paramsRow.numSimulations"
                class="h-8 font-mono"
                type="number"
                :min="1"
                :max="2000"
                @change="persistParams"
              />
            </div>
          </div>
        </div>
      </section>

      <!-- 测算结果 -->
      <section v-if="results && best" class="mb-2">
        <h3 class="mb-2 text-sm font-medium">测算结果</h3>

        <!-- 最优方案卡片 -->
        <div class="mb-3 rounded-lg border border-border bg-card p-4">
          <div class="mb-3 flex items-center gap-2">
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
              <p class="font-mono text-2xl font-semibold">{{ best.target_price.toFixed(4) }} 万</p>
            </div>
            <div>
              <p class="text-xs text-muted-foreground">预期得分</p>
              <p class="font-mono text-2xl font-semibold text-success">
                {{ best.target_score.toFixed(2) }}
              </p>
            </div>
            <div>
              <p class="text-xs text-muted-foreground">基准价</p>
              <p class="font-mono text-2xl font-semibold">{{ best.base_price.toFixed(2) }} 万</p>
            </div>
            <div>
              <p class="text-xs text-muted-foreground">预测均价 / 搜索范围</p>
              <p class="font-mono text-2xl font-semibold">
                {{ best.search_center.toFixed(2) }}
                <span class="text-sm text-muted-foreground"
                  >±{{ best.search_range.toFixed(2) }}</span
                >
              </p>
            </div>
          </div>
        </div>

        <!-- 场景对比表 -->
        <div class="overflow-hidden rounded-lg border border-border bg-card">
          <Table>
            <TableHeader>
              <TableRow class="hover:bg-transparent">
                <TableHead class="pl-3">场景</TableHead>
                <TableHead class="w-32">降幅</TableHead>
                <TableHead class="w-28">标准差</TableHead>
                <TableHead class="w-20">参与率</TableHead>
                <TableHead class="w-24">预测均价</TableHead>
                <TableHead class="w-28">目标报价(万)</TableHead>
                <TableHead class="w-24">基准价</TableHead>
                <TableHead class="w-20">得分</TableHead>
                <TableHead class="w-16 pr-3"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow
                v-for="(r, index) in results.results"
                :key="r.scenario_name"
                :class="index === results.best_index ? 'bg-primary/5' : ''"
                class="hover:bg-transparent"
              >
                <TableCell class="pl-3 font-medium">{{ r.scenario_name }}</TableCell>
                <TableCell class="font-mono text-xs">
                  {{ formatReduction(r.reduction_type, r.reduction_value) }}
                </TableCell>
                <TableCell class="text-xs text-muted-foreground">
                  {{ formatStd(r.reduction_type, r.std_dev) }}
                </TableCell>
                <TableCell class="text-xs text-muted-foreground">
                  {{ formatRate(r.participation_rate) }}
                </TableCell>
                <TableCell class="font-mono text-xs">{{ r.search_center.toFixed(2) }}</TableCell>
                <TableCell class="font-mono text-xs font-semibold">
                  {{ r.target_price.toFixed(4) }}
                </TableCell>
                <TableCell class="font-mono text-xs">{{ r.base_price.toFixed(2) }}</TableCell>
                <TableCell class="font-mono text-xs">{{ r.target_score.toFixed(2) }}</TableCell>
                <TableCell class="pr-3 text-right">
                  <Badge v-if="index === results.best_index" variant="default">最优</Badge>
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </div>
        <p class="mt-2 text-xs text-muted-foreground">
          共 {{ best.num_simulations }} 次模拟 · 各场景取中位数推荐 · 步长
          {{ formatWan(best.search_step, 4) }} 万
        </p>
      </section>
    </template>

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
          <AlertDialogAction @click="pickImportFile">选择文件并导入</AlertDialogAction>
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
        <div class="space-y-1 py-2">
          <Label for="import-plan-name">方案名称</Label>
          <Input
            id="import-plan-name"
            v-model="importPlanName"
            placeholder="例如：XX 项目二轮报价"
            @keydown.enter="runImport"
          />
          <p class="text-xs text-muted-foreground">名称会显示在页头，并随每次测算记录到历史</p>
        </div>
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
