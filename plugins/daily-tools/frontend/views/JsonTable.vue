<!--
  JSON 表格工具页：JSON / Markdown / CSV / Excel 表格互转（键为表头）。

  统一中间表示见 `frontend/table/`：文本类格式在前端解析与生成，xlsx 走 Rust（calamine /
  rust_xlsxwriter）。右侧预览截断，复制与导出始终使用全量数据。
-->
<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog';
import { ArrowLeftRight, FileDown, FileUp, RotateCcw, Table as TableIcon } from '@lucide/vue';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { SettingsRow, SettingsSection } from '@/components/settings';
import { Textarea } from '@/components/ui/textarea';
import Panel from '@/components/tool/Panel.vue';
import EmptyState from '@/components/native/EmptyState.vue';
import ToolShell from '@/components/tool/ToolShell.vue';
import { ipc } from '@/core/ipc';
import { onOpenFiles } from '@/core/open-with';
import TablePreview from '../components/table/TablePreview.vue';
import { errorMessage } from '../shared';
import { parseTextSource, previewTable, serializeTextTarget } from '../table/convert';
import { TableError, type ArrayPathCandidate, type Table } from '../table/model';
import {
  DATE_MODES,
  DEFAULT_DATE_FORMAT,
  FORMAT_LABELS,
  MAX_ROWS,
  PREVIEW_ROWS,
  TABLE_FORMATS,
  type DateMode,
  type TableFormat,
} from '../table/options';
import { listSheets, readSheet, writeWorkbook } from '../table/xlsx';

// ── 格式选择 ──────────────────────────────────────────────────────

const sourceFormat = ref<TableFormat>('json');
const targetFormat = ref<TableFormat>('markdown');

function swapFormats(): void {
  const previous = sourceFormat.value;
  sourceFormat.value = targetFormat.value;
  targetFormat.value = previous;
}

// ── 输入：文本源 ──────────────────────────────────────────────────

const text = ref('');

async function openTextFile(): Promise<void> {
  const extensions: Record<string, string[]> = {
    json: ['json'],
    markdown: ['md', 'markdown'],
    csv: ['csv'],
  };
  const format = sourceFormat.value;
  const path = await openFileDialog({
    filters: [{ name: FORMAT_LABELS[format], extensions: extensions[format] ?? [] }],
  });
  if (!path) return;

  try {
    text.value = await ipc<string>('file_read_text', { path: String(path) });
  } catch (err) {
    toast.error(`读取失败：${errorMessage(err)}`);
  }
}

// ── 输入：xlsx ───────────────────────────────────────────────────

const xlsxPath = ref('');
const xlsxName = ref('');
const sheets = ref<string[]>([]);
const selectedSheet = ref('');

async function openXlsxFile(): Promise<void> {
  const path = await openFileDialog({
    filters: [{ name: 'Excel 工作簿', extensions: ['xlsx'] }],
  });
  if (!path) return;
  await loadWorkbook(String(path));
}

async function loadWorkbook(path: string): Promise<void> {
  loading.value = true;
  error.value = '';
  try {
    xlsxPath.value = path;
    xlsxName.value = path.split(/[/\\]/).pop() ?? '';
    sheets.value = await listSheets(path);
    selectedSheet.value = sheets.value[0] ?? '';
    await loadSheet();
  } catch (err) {
    tables.value = [];
    error.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}

async function loadSheet(): Promise<void> {
  if (!xlsxPath.value) return;
  loading.value = true;
  error.value = '';
  try {
    const table = await readSheet(xlsxPath.value, {
      sheet: selectedSheet.value,
      hasHeader: hasHeader.value,
      fillMerged: fillMerged.value,
      trimEmpty: trimEmpty.value,
    });
    tables.value = [table];
    activeTable.value = 0;
  } catch (err) {
    tables.value = [];
    error.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}

// ── 选项 ─────────────────────────────────────────────────────────

const jsonPath = ref('');
const inferTypes = ref(false);
const hasHeader = ref(true);
const restoreBreaks = ref(true);
const escapeNewlines = ref(true);
const preventInjection = ref(true);
const fillMerged = ref(false);
const trimEmpty = ref(true);
const styleWorkbook = ref(true);
const dateMode = ref<DateMode>('iso');
const dateFormat = ref(DEFAULT_DATE_FORMAT);

const candidates = ref<ArrayPathCandidate[]>([]);
const showJsonPath = computed(() => sourceFormat.value === 'json');
const showInferTypes = computed(
  () => sourceFormat.value === 'markdown' || sourceFormat.value === 'csv'
);
const showTargetTextOptions = computed(() => targetFormat.value !== 'xlsx');

// ── 转换 ─────────────────────────────────────────────────────────

const tables = ref<Table[]>([]);
const activeTable = ref(0);
const error = ref('');
const loading = ref(false);

const tableOptions = computed(() => ({
  dateMode: dateMode.value,
  dateFormat: dateFormat.value,
}));

const selectedTable = computed<Table | null>(() => tables.value[activeTable.value] ?? null);
const totalRows = computed(() => tables.value.reduce((sum, table) => sum + table.rows.length, 0));

function selectTable(value: unknown): void {
  const index = Number(value);
  if (Number.isInteger(index) && index >= 0 && index < tables.value.length) {
    activeTable.value = index;
  }
}

function convert(): void {
  if (sourceFormat.value === 'xlsx') return;
  if (text.value.trim() === '') {
    tables.value = [];
    error.value = '';
    candidates.value = [];
    return;
  }

  error.value = '';
  candidates.value = [];
  try {
    tables.value = parseTextSource(sourceFormat.value, text.value, {
      jsonPath: jsonPath.value,
      inferTypes: inferTypes.value,
      hasHeader: hasHeader.value,
      restoreBreaks: restoreBreaks.value,
    });
    if (activeTable.value >= tables.value.length) activeTable.value = 0;
  } catch (err) {
    tables.value = [];
    if (err instanceof TableError) {
      error.value = err.message;
      candidates.value = err.candidates ?? [];
    } else {
      error.value = errorMessage(err);
    }
  }
}

let timer: ReturnType<typeof setTimeout> | undefined;

function schedule(action: () => void): void {
  if (timer) clearTimeout(timer);
  timer = setTimeout(action, 200);
}

watch([text, jsonPath, inferTypes, hasHeader, restoreBreaks], () => {
  if (sourceFormat.value === 'xlsx') return;
  schedule(convert);
});

watch(sourceFormat, (format) => {
  error.value = '';
  candidates.value = [];
  activeTable.value = 0;
  if (format === 'xlsx') {
    if (xlsxPath.value) schedule(() => void loadSheet());
    else tables.value = [];
    return;
  }
  schedule(convert);
});

watch([selectedSheet, hasHeader, fillMerged, trimEmpty], () => {
  if (sourceFormat.value !== 'xlsx') return;
  schedule(() => void loadSheet());
});

watch(targetFormat, () => {
  if (activeTable.value >= tables.value.length) activeTable.value = 0;
});

onUnmounted(() => {
  if (timer) clearTimeout(timer);
});

// ── 输出 ─────────────────────────────────────────────────────────

const previewTables = computed<Table[]>(() => {
  if (targetFormat.value === 'json')
    return tables.value.map((table) => previewTable(table, PREVIEW_ROWS));
  return selectedTable.value ? [previewTable(selectedTable.value, PREVIEW_ROWS)] : [];
});

const previewText = computed(() => {
  if (targetFormat.value === 'xlsx' || previewTables.value.length === 0) return '';
  try {
    return serializeTextTarget(targetFormat.value, previewTables.value, {
      ...tableOptions.value,
      escapeNewlines: escapeNewlines.value,
      preventInjection: preventInjection.value,
    });
  } catch (err) {
    return err instanceof Error ? err.message : String(err);
  }
});

const truncated = computed(() => tables.value.some((table) => table.rows.length > PREVIEW_ROWS));

/** 输出头部补充信息：格式 + 行列统计（无数据时不显示） */
const outputHint = computed(() => {
  const format = FORMAT_LABELS[targetFormat.value];
  if (tables.value.length === 0) return format;
  const stats = `${tables.value.length} 张表 · ${totalRows.value} 行${truncated.value ? `，预览前 ${PREVIEW_ROWS} 行` : ''}`;
  return `${format} · ${stats}`;
});

function targetTables(): Table[] {
  if (targetFormat.value === 'json') return tables.value;
  return selectedTable.value ? [selectedTable.value] : [];
}

function buildFullText(): string {
  const list = targetTables();
  if (list.length === 0) return '';
  return serializeTextTarget(targetFormat.value, list, {
    ...tableOptions.value,
    escapeNewlines: escapeNewlines.value,
    preventInjection: preventInjection.value,
  });
}

async function copyOutput(): Promise<void> {
  const content = buildFullText();
  if (!content) return;
  try {
    await navigator.clipboard.writeText(content);
    toast.success('已复制完整结果');
  } catch {
    toast.error('复制失败');
  }
}

const saving = ref(false);

const TARGET_EXTENSIONS: Record<TableFormat, string> = {
  json: 'json',
  markdown: 'md',
  csv: 'csv',
  xlsx: 'xlsx',
};

async function exportOutput(): Promise<void> {
  const target = targetFormat.value;
  if (target === 'xlsx' && tables.value.some((table) => table.rows.length > MAX_ROWS)) {
    toast.error(`行数超过上限（${MAX_ROWS} 行），请拆分后再导出`);
    return;
  }
  if (target !== 'xlsx' && buildFullText() === '') {
    toast.error('没有可导出的内容');
    return;
  }

  const baseName = xlsxName.value.replace(/\.[^.]+$/, '') || 'table';
  const extension = TARGET_EXTENSIONS[target];
  const path = await saveFileDialog({
    defaultPath: `${baseName}.${extension}`,
    filters: [{ name: FORMAT_LABELS[target], extensions: [extension] }],
  });
  if (!path) return;

  saving.value = true;
  try {
    if (target === 'xlsx') {
      await writeWorkbook(String(path), tables.value, styleWorkbook.value);
    } else {
      await ipc('file_write_text', { path: String(path), contents: buildFullText() });
    }
    toast.success('已导出文件');
  } catch (err) {
    toast.error(`导出失败：${errorMessage(err)}`);
  } finally {
    saving.value = false;
  }
}

// ── 拖拽 / 文件关联 ───────────────────────────────────────────────

const TEXT_FORMAT_BY_EXTENSION: Record<string, TableFormat> = {
  json: 'json',
  md: 'markdown',
  markdown: 'markdown',
  csv: 'csv',
};

const offOpenFiles = onOpenFiles(async (paths) => {
  const path = paths[0];
  if (!path) return;
  const extension = path.split('.').pop()?.toLowerCase() ?? '';

  if (extension === 'xlsx') {
    sourceFormat.value = 'xlsx';
    await loadWorkbook(path);
    return;
  }

  const format = TEXT_FORMAT_BY_EXTENSION[extension];
  if (!format) {
    toast.warning(`仅支持 .json / .md / .csv / .xlsx，收到 .${extension}`);
    return;
  }
  try {
    text.value = await ipc<string>('file_read_text', { path });
    sourceFormat.value = format;
  } catch (err) {
    toast.error(`读取失败：${errorMessage(err)}`);
  }
});

onUnmounted(offOpenFiles);
</script>

<template>
  <ToolShell
    title="JSON 表格"
    description="JSON 与 Markdown / CSV / Excel 表格互转，JSON 的键为表头"
  >
    <div class="grid w-full grid-cols-12 gap-4">
      <!-- 转换设置 -->
      <Panel class="col-span-12" title="转换设置" body-class="space-y-4">
        <template #actions>
          <Button variant="ghost" size="sm" @click="swapFormats">
            <ArrowLeftRight class="mr-1 size-3.5" />
            交换源与目标
          </Button>
        </template>

        <!-- 格式 -->
        <SettingsSection title="格式" class="border-0 bg-transparent">
          <SettingsRow title="源格式">
            <Select v-model="sourceFormat">
              <SelectTrigger class="h-9 w-56"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in TABLE_FORMATS" :key="item" :value="item">
                  {{ FORMAT_LABELS[item] }}
                </SelectItem>
              </SelectContent>
            </Select>
          </SettingsRow>

          <SettingsRow title="目标格式">
            <Select v-model="targetFormat">
              <SelectTrigger class="h-9 w-56"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in TABLE_FORMATS" :key="item" :value="item">
                  {{ FORMAT_LABELS[item] }}
                </SelectItem>
              </SelectContent>
            </Select>
          </SettingsRow>
        </SettingsSection>

        <!-- 输入选项：一行一个设置，标题在左、控件在右（HIG：开关只放在列表行里） -->
        <SettingsSection title="输入选项" class="border-0 bg-transparent">
          <SettingsRow v-if="showJsonPath" title="数据路径" description="根节点就是数组时留空">
            <Input
              v-model="jsonPath"
              class="w-72 font-mono"
              placeholder="data.list / items[0].rows"
            />
          </SettingsRow>

          <SettingsRow
            v-if="sourceFormat === 'csv' || sourceFormat === 'xlsx'"
            title="首行是表头"
            description="关闭时列名为 column1、column2 …"
          >
            <Switch v-model="hasHeader" size="sm" />
          </SettingsRow>

          <SettingsRow
            v-if="showInferTypes"
            title="类型推断"
            description="识别数字与 true / false，默认按文本处理"
          >
            <Switch v-model="inferTypes" size="sm" />
          </SettingsRow>

          <SettingsRow v-if="sourceFormat === 'markdown'" title="还原 &lt;br&gt;">
            <Switch v-model="restoreBreaks" size="sm" />
          </SettingsRow>

          <SettingsRow v-if="sourceFormat === 'xlsx'" title="合并单元格填充">
            <Switch v-model="fillMerged" size="sm" />
          </SettingsRow>

          <SettingsRow v-if="sourceFormat === 'xlsx'" title="裁除空行列">
            <Switch v-model="trimEmpty" size="sm" />
          </SettingsRow>
        </SettingsSection>

        <!-- 输出选项 -->
        <SettingsSection title="输出选项" class="border-0 bg-transparent">
          <SettingsRow v-if="targetFormat === 'markdown'" title="换行转 &lt;br&gt;">
            <Switch v-model="escapeNewlines" size="sm" />
          </SettingsRow>

          <SettingsRow
            v-if="targetFormat === 'csv'"
            title="防公式注入"
            description="以 = + - @ 开头的文本前置 ' 前缀"
          >
            <Switch v-model="preventInjection" size="sm" />
          </SettingsRow>

          <SettingsRow v-if="targetFormat === 'xlsx'" title="表头样式">
            <Switch v-model="styleWorkbook" size="sm" />
          </SettingsRow>

          <SettingsRow v-if="showTargetTextOptions" title="日期输出">
            <Select v-model="dateMode">
              <SelectTrigger class="h-9 w-56"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in DATE_MODES" :key="item.value" :value="item.value">
                  {{ item.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </SettingsRow>

          <SettingsRow v-if="showTargetTextOptions && dateMode === 'custom'" title="日期格式">
            <Input v-model="dateFormat" class="w-56 font-mono" placeholder="yyyy-MM-dd HH:mm:ss" />
          </SettingsRow>

          <!-- 说明走分组页脚（灰色说明文字），不占设置行、也不做成填充色块 -->
          <template v-if="!showTargetTextOptions" #footer>
            Excel 目标写入的是原生日期单元格，因此不需要「日期输出」选项。
          </template>
        </SettingsSection>
      </Panel>

      <!-- 输入 -->
      <Panel
        class="col-span-12 min-w-0 lg:col-span-6"
        title="输入"
        :hint="FORMAT_LABELS[sourceFormat]"
      >
        <template #actions>
          <Button
            v-if="sourceFormat === 'xlsx'"
            variant="secondary"
            size="sm"
            :disabled="loading"
            @click="openXlsxFile"
          >
            <FileUp class="mr-1 size-3.5" />
            选择 .xlsx
          </Button>
          <template v-else>
            <Button variant="secondary" size="sm" @click="openTextFile">
              <FileUp class="mr-1 size-3.5" />
              选择文件
            </Button>
            <Button variant="ghost" size="sm" :disabled="!text" @click="text = ''">
              <RotateCcw class="mr-1 size-3.5" />
              清空
            </Button>
          </template>
        </template>

        <!-- xlsx 输入 -->
        <template v-if="sourceFormat === 'xlsx'">
          <button
            type="button"
            class="flex min-h-[180px] w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-md bg-sunken px-6 py-8 text-center transition-colors hover:bg-accent"
            @click="openXlsxFile"
          >
            <FileUp class="size-8 text-muted-foreground" />
            <p class="text-sm text-muted-foreground">
              {{ xlsxName || '点击选择 .xlsx 文件，或把文件拖到窗口' }}
            </p>
            <p v-if="loading" class="text-xs text-muted-foreground">读取中…</p>
          </button>

          <div v-if="sheets.length > 0" class="space-y-1.5">
            <Label>工作表</Label>
            <Select v-model="selectedSheet">
              <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="item in sheets" :key="item" :value="item">
                  {{ item }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </template>

        <!-- 文本输入 -->
        <Textarea
          v-else
          v-model="text"
          class="h-80 resize-y overflow-y-auto font-mono text-sm"
          style="field-sizing: fixed"
          :placeholder="
            sourceFormat === 'json'
              ? '粘贴 JSON 数组，或粘贴完整 JSON 后用「数据路径」指定数组位置…'
              : '粘贴内容，或点击「选择文件」/ 拖拽文件到窗口…'
          "
        />

        <p v-if="sourceFormat !== 'xlsx'" class="text-xs text-muted-foreground">
          支持直接拖拽文件到窗口；CSV 按 UTF-8 解析。
        </p>
      </Panel>

      <!-- 输出 -->
      <Panel class="col-span-12 min-w-0 lg:col-span-6" title="输出" :hint="outputHint">
        <template #actions>
          <Button variant="ghost" size="sm" :disabled="tables.length === 0" @click="copyOutput">
            复制
          </Button>
          <Button size="sm" :disabled="tables.length === 0 || saving" @click="exportOutput">
            <FileDown class="mr-1 size-3.5" />
            导出 .{{ TARGET_EXTENSIONS[targetFormat] }}
          </Button>
        </template>

        <div v-if="tables.length > 1" class="space-y-1.5">
          <Label>选择要转换的表（Markdown / CSV 一次一张）</Label>
          <Select :model-value="String(activeTable)" @update:model-value="selectTable">
            <SelectTrigger class="h-9 w-full"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem v-for="(table, index) in tables" :key="index" :value="String(index)">
                {{ table.name }}（{{ table.rows.length }} 行）
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        <p
          v-if="error"
          class="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive"
        >
          {{ error }}
        </p>

        <div v-if="candidates.length > 0" class="space-y-1.5">
          <p class="text-xs text-muted-foreground">点击填入「数据路径」：</p>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="item in candidates"
              :key="item.path"
              type="button"
              class="rounded-full border px-3 py-1 font-mono text-xs transition-colors hover:bg-muted"
              @click="jsonPath = item.path"
            >
              {{ item.path }}（{{ item.length }} 项{{ item.keys ? ` · ${item.keys} 键` : '' }}）
            </button>
          </div>
        </div>

        <div v-if="loading" class="flex items-center gap-2 text-sm text-muted-foreground">
          <div
            class="size-4 animate-spin rounded-full border-2 border-current border-t-transparent"
          />
          读取中…
        </div>

        <template v-else-if="tables.length > 0">
          <TablePreview
            v-if="targetFormat === 'xlsx' && selectedTable"
            :table="selectedTable"
            :options="tableOptions"
            :limit="PREVIEW_ROWS"
          />
          <pre
            v-else
            class="max-h-[460px] overflow-auto font-mono text-sm leading-relaxed whitespace-pre-wrap break-words"
            >{{ previewText }}</pre>
        </template>

        <EmptyState
          v-else-if="!error"
          :icon="TableIcon"
          title="还没有数据"
          description="在左侧粘贴 JSON 或选择文件，转换结果会显示在这里"
        />
      </Panel>
    </div>
  </ToolShell>
</template>
