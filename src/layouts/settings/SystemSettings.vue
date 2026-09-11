<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { ChevronRight } from '@lucide/vue';
import { openPath } from '@tauri-apps/plugin-opener';
import { appDataDir, appLogDir } from '@tauri-apps/api/path';
import { getVersion } from '@tauri-apps/api/app';
import { confirm, open, save } from '@tauri-apps/plugin-dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { toast } from 'vue-sonner';
import { normalizeError } from '@/core/errors';
import { logger } from '@/core/logger';
import { ipc } from '@/core/ipc';
import { getPluginGroups } from '@/core/plugins';
import { checkForUpdates, updaterState } from '@/core/updater';
import { exportDiagnostics } from '@/core/diagnostics';
import { exportSettings, importSettings, restartApp } from '@/core/settings-transfer';
import { useSettingsStore, type LogLevel } from '@/stores/settings';
import { globalShortcutError } from '@/core/global-shortcut';
import { notify } from '@/core/notify';
import ShortcutRecorder from './ShortcutRecorder.vue';
import {
  ACCENTS,
  applyAccent,
  applyCustomAccent,
  applyFontSize,
  CUSTOM_ACCENT_ID,
  FONT_SIZES,
  readStoredAccent,
  readStoredCustomAccent,
  readStoredFontSize,
  type FontSize,
  type ThemeMode,
} from '@/core/theme';

const settings = useSettingsStore();

/** 打开目录：失败时明确提示（错误同时写入日志） */
async function openDir(resolvePath: () => Promise<string>): Promise<void> {
  try {
    await openPath(await resolvePath());
  } catch (err) {
    const error = normalizeError(err);
    logger.error(`打开目录失败: [${error.code}] ${error.message}`);
    toast.error(`打开目录失败：${error.message}`, { description: error.code });
  }
}

const openLogDir = () => openDir(appLogDir);
const openDataDir = () => openDir(appDataDir);

// 更新：当前版本 + 手动检查
const appVersion = ref('');
onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch (err) {
    logger.debug(`读取应用版本失败: ${String(err)}`);
  }
});

async function onCheckUpdate(): Promise<void> {
  try {
    const info = await checkForUpdates();
    if (info) {
      toast.info(`发现新版本 ${info.version}`);
    } else {
      toast.success('当前已是最新版本');
    }
  } catch (err) {
    const error = normalizeError(err);
    logger.error(`检查更新失败: [${error.code}] ${error.message}`);
    toast.error(`检查更新失败：${error.message}`, { description: error.code });
  }
}

function reportFailure(prefix: string, err: unknown): void {
  const error = normalizeError(err);
  logger.error(`${prefix}失败: [${error.code}] ${error.message}`);
  toast.error(`${prefix}失败：${error.message}`, { description: error.code });
}

/** 发送一条系统通知用于验证权限与投递（首次会触发系统授权弹窗） */
async function onTestNotification(): Promise<void> {
  await notify('测试通知', '这是一条测试系统通知；能看到即表示通知功能正常。');
  toast.info('已发出系统通知', {
    description: '若未弹出，请检查系统「通知」权限与「专注/免打扰」模式',
  });
}

// ── 数据：备份 / 恢复 / 重置 ──
async function onBackupDb(): Promise<void> {
  try {
    const path = await save({
      defaultPath: 'arkdesk-backup.db',
      filters: [{ name: 'SQLite', extensions: ['db'] }],
    });
    if (!path) return;
    await ipc<string>('db_backup', { path });
    toast.success('数据库已备份');
  } catch (err) {
    reportFailure('备份', err);
  }
}

async function onRestoreDb(): Promise<void> {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'SQLite', extensions: ['db'] }],
    });
    if (typeof selected !== 'string') return;
    const ok = await confirm('恢复数据库将覆盖当前全部数据并重启应用，确定继续？', {
      title: '恢复数据库',
      kind: 'warning',
    });
    if (!ok) return;
    await ipc('db_restore', { path: selected });
  } catch (err) {
    reportFailure('恢复', err);
  }
}

async function onResetDb(): Promise<void> {
  try {
    const ok = await confirm('重置将清空全部数据并重启应用，且不可恢复。确定继续？', {
      title: '重置数据库',
      kind: 'warning',
    });
    if (!ok) return;
    await ipc('db_reset');
  } catch (err) {
    reportFailure('重置', err);
  }
}

// ── 设置导入 / 导出 ──
async function onExportSettings(): Promise<void> {
  try {
    const path = await exportSettings();
    if (path) toast.success('设置已导出');
  } catch (err) {
    reportFailure('导出设置', err);
  }
}

async function onImportSettings(): Promise<void> {
  try {
    const ok = await importSettings();
    if (!ok) return;
    const doRestart = await confirm('设置已导入，重启后生效。现在重启？', { title: '导入设置' });
    if (doRestart) await restartApp();
  } catch (err) {
    reportFailure('导入设置', err);
  }
}

// ── 诊断 ──
async function onExportDiagnostics(): Promise<void> {
  try {
    const path = await exportDiagnostics();
    if (path) toast.success('诊断报告已导出');
  } catch (err) {
    reportFailure('导出诊断', err);
  }
}

const themeOptions: { value: ThemeMode; label: string }[] = [
  { value: 'light', label: '亮色' },
  { value: 'dark', label: '暗色' },
  { value: 'system', label: '跟随系统' },
];

const logLevelOptions: { value: LogLevel; label: string }[] = [
  { value: 'trace', label: 'Trace（最详细）' },
  { value: 'debug', label: 'Debug' },
  { value: 'info', label: 'Info' },
  { value: 'warn', label: 'Warn' },
  { value: 'error', label: 'Error' },
];

const currentAccent = ref(readStoredAccent());
const currentFontSize = ref<FontSize>(readStoredFontSize());
const customAccent = ref(readStoredCustomAccent() ?? '#6366f1');

function selectAccent(id: string): void {
  currentAccent.value = id;
  applyAccent(id);
}

function onCustomAccent(event: Event): void {
  const value = (event.target as HTMLInputElement).value;
  customAccent.value = value;
  currentAccent.value = CUSTOM_ACCENT_ID;
  applyCustomAccent(value);
}

function selectFontSize(size: FontSize): void {
  currentFontSize.value = size;
  applyFontSize(size);
}

// 工具管理树形：插件 → 工具（默认全部收起）
const pluginGroups = getPluginGroups();
const expanded = ref<Set<string>>(new Set());

function isExpanded(id: string): boolean {
  return expanded.value.has(id);
}

function toggleExpanded(id: string): void {
  const next = new Set(expanded.value);
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  expanded.value = next;
}
</script>

<template>
  <div class="space-y-6">
    <!-- 外观 -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">外观</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <span class="text-sm">主题</span>
          <div class="flex gap-0.5 rounded-[7px] border border-border p-0.5">
            <Button
              v-for="option in themeOptions"
              :key="option.value"
              variant="ghost"
              size="sm"
              class="h-7 px-2.5 text-xs"
              :class="
                settings.themeMode === option.value
                  ? 'bg-primary text-primary-foreground hover:bg-primary hover:text-primary-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              "
              @click="settings.themeMode = option.value"
            >
              {{ option.label }}
            </Button>
          </div>
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <span class="text-sm">主题色</span>
          <div class="flex items-center gap-2">
            <button
              v-for="accent in ACCENTS"
              :key="accent.id"
              type="button"
              class="size-5 rounded-full border border-black/10 transition-transform hover:scale-110 dark:border-white/20"
              :class="
                currentAccent === accent.id
                  ? 'ring-2 ring-foreground/60 ring-offset-2 ring-offset-background'
                  : ''
              "
              :style="{ backgroundColor: accent.preview }"
              :title="accent.label"
              :aria-label="`主题色：${accent.label}`"
              @click="selectAccent(accent.id)"
            />
            <input
              type="color"
              :value="customAccent"
              class="size-5 cursor-pointer rounded-full border border-black/10 bg-transparent p-0 dark:border-white/20"
              title="自定义主题色"
              aria-label="自定义主题色"
              @input="onCustomAccent"
            />
          </div>
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">字号</p>
            <p class="text-xs text-muted-foreground">界面整体缩放，立即生效</p>
          </div>
          <Select
            :model-value="currentFontSize"
            @update:model-value="(value) => selectFontSize(Number(value) as FontSize)"
          >
            <SelectTrigger id="font-size" class="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem v-for="option in FONT_SIZES" :key="option.value" :value="option.value">
                {{ option.label }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>
    </section>

    <!-- 通用 -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">通用</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">关闭窗口时隐藏到托盘</p>
            <p class="text-xs text-muted-foreground">关闭后应用保留在系统托盘，可随时唤起</p>
          </div>
          <Switch
            :model-value="settings.closeToTray"
            @update:model-value="(value) => (settings.closeToTray = value === true)"
          />
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">开机自启</p>
            <p class="text-xs text-muted-foreground">系统启动时自动运行本应用</p>
          </div>
          <Switch
            :model-value="settings.autoStart"
            @update:model-value="(value) => (settings.autoStart = value === true)"
          />
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">系统通知</p>
            <p class="text-xs text-muted-foreground">允许应用发送系统级通知</p>
          </div>
          <div class="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              :disabled="!settings.notificationEnabled"
              @click="onTestNotification"
            >
              测试
            </Button>
            <Switch
              :model-value="settings.notificationEnabled"
              @update:model-value="(value) => (settings.notificationEnabled = value === true)"
            />
          </div>
        </div>
      </div>
    </section>

    <!-- 快捷键 -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">快捷键</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div class="min-w-0">
            <p class="text-sm">全局快捷键（唤起窗口）</p>
            <p class="text-xs text-muted-foreground">
              点击后按下按键组合录制；Esc 取消，Backspace/Delete 清除。需含 Cmd/Ctrl/Alt，
              被占用或与菜单快捷键冲突时注册失败（见日志）
            </p>
          </div>
          <div class="flex flex-col items-end gap-1">
            <ShortcutRecorder v-model="settings.globalShortcut" />
            <p v-if="globalShortcutError" class="text-xs text-destructive">
              {{ globalShortcutError }}
            </p>
          </div>
        </div>
      </div>
    </section>

    <!-- 网络 -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">网络</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div class="min-w-0">
            <p class="text-sm">HTTP 代理</p>
            <p class="text-xs text-muted-foreground">如 http://127.0.0.1:7890；留空直连</p>
          </div>
          <Input v-model="settings.proxyUrl" class="w-64" placeholder="留空直连" />
        </div>
      </div>
    </section>

    <!-- AI -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">AI</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div class="min-w-0">
            <p class="text-sm">接口地址</p>
            <p class="text-xs text-muted-foreground">
              OpenAI 兼容 Base URL（如 https://api.openai.com/v1）
            </p>
          </div>
          <Input
            v-model="settings.aiBaseUrl"
            class="w-72"
            placeholder="https://api.openai.com/v1"
          />
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div class="min-w-0">
            <p class="text-sm">API Key</p>
            <p class="text-xs text-muted-foreground">明文保存在本机设置文件</p>
          </div>
          <Input v-model="settings.aiApiKey" type="password" class="w-72" placeholder="sk-..." />
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div class="min-w-0">
            <p class="text-sm">模型</p>
            <p class="text-xs text-muted-foreground">如 gpt-4o-mini / deepseek-chat</p>
          </div>
          <Input v-model="settings.aiModel" class="w-72" placeholder="gpt-4o-mini" />
        </div>
      </div>
    </section>

    <!-- 更新 -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">更新</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">启用在线更新</p>
            <p class="text-xs text-muted-foreground">
              当前版本{{ appVersion ? ` v${appVersion}` : '' }}
              <template v-if="settings.updateLastCheckAt">
                · 上次检查 {{ new Date(settings.updateLastCheckAt).toLocaleString() }}
              </template>
            </p>
          </div>
          <Switch
            :model-value="settings.updateEnabled"
            @update:model-value="(value) => (settings.updateEnabled = value === true)"
          />
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div class="min-w-0">
            <p class="text-sm">更新服务器地址</p>
            <p class="text-xs text-muted-foreground">
              HTTPS；可填清单地址或目录（目录自动补 /latest.json）
            </p>
          </div>
          <Input
            v-model="settings.updateServerUrl"
            class="w-72"
            placeholder="https://releases.example.com"
            :disabled="!settings.updateEnabled"
          />
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">启动时自动检查</p>
            <p class="text-xs text-muted-foreground">应用启动后自动检查一次更新</p>
          </div>
          <Switch
            :model-value="settings.updateAutoCheck"
            :disabled="!settings.updateEnabled"
            @update:model-value="(value) => (settings.updateAutoCheck = value === true)"
          />
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <span class="text-sm">检查更新</span>
          <Button
            variant="outline"
            size="sm"
            :disabled="
              !settings.updateEnabled || !settings.updateServerUrl || updaterState.checking
            "
            @click="onCheckUpdate"
          >
            {{ updaterState.checking ? '检查中…' : '立即检查' }}
          </Button>
        </div>
      </div>
    </section>

    <!-- 日志 -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">日志</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <span class="text-sm">日志级别</span>
          <Select
            :model-value="settings.logLevel"
            @update:model-value="(value) => (settings.logLevel = value as LogLevel)"
          >
            <SelectTrigger id="log-level" class="w-40">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem
                v-for="option in logLevelOptions"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <span class="text-sm">日志文件目录</span>
          <Button variant="outline" size="sm" @click="openLogDir">打开目录</Button>
        </div>
      </div>
    </section>

    <!-- 工具管理：树形（插件 → 工具），默认收起 -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">工具管理</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div v-for="group in pluginGroups" :key="group.id">
          <!-- 插件行：点击展开/收起 -->
          <button
            type="button"
            class="flex w-full items-center justify-between gap-2 px-3 py-2.5 text-left transition-colors hover:bg-accent/50"
            @click="toggleExpanded(group.id)"
          >
            <div class="flex min-w-0 items-center gap-2">
              <ChevronRight
                class="size-4 shrink-0 text-muted-foreground transition-transform"
                :class="{ 'rotate-90': isExpanded(group.id) }"
              />
              <span class="truncate text-sm font-medium">{{ group.name }}</span>
              <span class="shrink-0 text-xs text-muted-foreground">
                {{ group.tools.length }} 个工具
              </span>
            </div>
          </button>

          <!-- 工具行：独立启停开关 -->
          <div
            v-show="isExpanded(group.id)"
            class="divide-y divide-border/60 border-t border-border/60"
          >
            <div
              v-for="tool in group.tools"
              :key="tool.meta.id"
              class="flex items-center justify-between gap-3 py-2.5 pl-8 pr-3"
            >
              <div class="min-w-0">
                <p class="text-sm">{{ tool.meta.name }}</p>
                <p v-if="tool.meta.description" class="truncate text-xs text-muted-foreground">
                  {{ tool.meta.description }}
                </p>
              </div>
              <Switch
                :model-value="settings.isToolEnabled(tool.meta.id)"
                @update:model-value="
                  (value) => settings.setToolEnabled(tool.meta.id, value === true)
                "
              />
            </div>
          </div>
        </div>
      </div>
      <p class="px-1 pt-1.5 text-xs text-muted-foreground">禁用的工具将从侧边导航隐藏</p>
    </section>

    <!-- 数据 -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">数据</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">应用数据目录</p>
            <p class="text-xs text-muted-foreground">SQLite 数据库与设置文件所在位置</p>
          </div>
          <Button variant="outline" size="sm" @click="openDataDir">打开目录</Button>
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">备份数据库</p>
            <p class="text-xs text-muted-foreground">导出当前数据库副本到指定文件</p>
          </div>
          <Button variant="outline" size="sm" @click="onBackupDb">备份</Button>
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">恢复数据库</p>
            <p class="text-xs text-muted-foreground">从备份文件覆盖当前数据（重启生效）</p>
          </div>
          <Button variant="outline" size="sm" @click="onRestoreDb">恢复</Button>
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">导出设置</p>
            <p class="text-xs text-muted-foreground">将全部设置导出为 JSON 文件</p>
          </div>
          <Button variant="outline" size="sm" @click="onExportSettings">导出</Button>
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">导入设置</p>
            <p class="text-xs text-muted-foreground">从 JSON 文件导入设置（重启生效）</p>
          </div>
          <Button variant="outline" size="sm" @click="onImportSettings">导入</Button>
        </div>
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm text-destructive">重置数据库</p>
            <p class="text-xs text-muted-foreground">清空全部数据并重启，不可恢复</p>
          </div>
          <Button variant="outline" size="sm" class="text-destructive" @click="onResetDb">
            重置
          </Button>
        </div>
      </div>
    </section>

    <!-- 诊断 -->
    <section>
      <h2 class="px-1 pb-1.5 text-xs font-medium text-muted-foreground">诊断</h2>
      <div class="divide-y divide-border rounded-lg border border-border bg-card">
        <div class="flex items-center justify-between gap-3 px-3 py-2.5">
          <div>
            <p class="text-sm">导出诊断报告</p>
            <p class="text-xs text-muted-foreground">环境信息 + 最近日志，便于反馈问题时附上</p>
          </div>
          <Button variant="outline" size="sm" @click="onExportDiagnostics">导出</Button>
        </div>
      </div>
    </section>
  </div>
</template>
