<!--
  视频生成页：
  - 手动输入：填写标题 / 作者 / 正文，可存草稿或直接生成
  - AI 生成：结构化 / 自由提示词 → 生成文章填入手动表单
  - 环境检查 → 生成（进度/日志/取消）→ 结果
-->
<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { toast } from 'vue-sonner';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ipc } from '@/core/ipc';
import { notify } from '@/core/notify';
import { useSettingsStore } from '@/stores/settings';
import {
  RefreshCw,
  Play,
  Square,
  FolderOpen,
  Film,
  CheckCircle2,
  XCircle,
  Info,
  PenLine,
  Sparkles,
  Save,
  Eraser,
} from '@lucide/vue';
import ToolShell from '@/components/tool/ToolShell.vue';
import ArticleForm from '../components/ArticleForm.vue';
import { useGeneration } from '../composables/useGeneration';
import {
  normalizeSettings,
  settings,
  statusLabel,
  STAGE_LABELS,
  THEME_OPTIONS,
  type ArticleValue,
  type AiArticle,
  type EnvStatus,
  type RunSummary,
} from '../shared';

const globalSettings = useSettingsStore();
const gen = useGeneration();
const { running, progress, logs, summary, progressPercent, begin, finish, cancel, openPath } = gen;

const sourceMode = ref<'manual' | 'ai'>('manual');
const env = ref<EnvStatus | null>(null);
const envLoading = ref(false);
const manual = ref<{ title: string; author: string; content: string; source: string }>({
  title: '',
  author: '',
  content: '',
  source: 'manual',
});

/** 生成页手动表单：与草稿箱共用 ArticleForm */
const manualForm = computed<ArticleValue>({
  get: () => ({
    title: manual.value.title,
    author: manual.value.author,
    content: manual.value.content,
  }),
  set: (value) => {
    manual.value = { ...manual.value, ...value };
  },
});

// AI
const aiMode = ref<'structured' | 'free'>('structured');
const aiTopic = ref('');
const aiStyle = ref('');
const aiWordCount = ref(900);
const aiLoading = ref(false);

const AI_PROMPT_KEY = 'text2video.aiPrompt';
const aiPrompt = ref(localStorage.getItem(AI_PROMPT_KEY) ?? '');
watch(aiPrompt, (value) => {
  if (value) localStorage.setItem(AI_PROMPT_KEY, value);
  else localStorage.removeItem(AI_PROMPT_KEY);
});

async function checkEnv(): Promise<void> {
  envLoading.value = true;
  try {
    const result = await ipc<EnvStatus>('text2video_check_env', {
      settings: normalizeSettings(settings.value),
    });
    env.value = result;
    if (result.ffmpegOk && result.fontsOk) {
      toast.success('环境就绪：ffmpeg 与中文字体均可用');
    } else {
      const missing = [
        !result.ffmpegOk ? 'ffmpeg 未找到' : '',
        !result.fontsOk ? '中文字体缺失' : '',
      ]
        .filter(Boolean)
        .join('、');
      toast.error(`环境不完整：${missing}${result.message ? `（${result.message}）` : ''}`);
    }
  } catch (err) {
    toast.error(`环境检查失败: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    envLoading.value = false;
  }
}

async function start(): Promise<void> {
  if (running.value) return;
  if (!env.value?.ffmpegOk) {
    toast.error('未检测到 ffmpeg，请先在设置中配置或安装系统 ffmpeg');
    return;
  }
  if (!manual.value.title.trim()) {
    toast.error('请填写标题');
    return;
  }
  if (!manual.value.content.trim()) {
    toast.error('请填写正文');
    return;
  }

  const channel = begin(1);
  try {
    const result = await ipc<RunSummary>('text2video_generate_manual', {
      options: { template: 'scroll', settings: normalizeSettings(settings.value) },
      input: {
        title: manual.value.title.trim(),
        author: manual.value.author.trim(),
        content: manual.value.content,
        source: manual.value.source ?? 'manual',
      },
      channel,
    });
    summary.value = result;
    if (result.cancelled) {
      toast.info(`已取消，本次生成 ${result.rendered} 条`);
    } else if (result.rendered > 0) {
      toast.success(`生成完成：成功 ${result.rendered} 条`);
      void notify('视频生成完成', `成功生成 ${result.rendered} 条视频`);
    } else {
      toast.warning('未生成视频，请查看日志');
    }
  } catch (err) {
    toast.error(`生成失败: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    finish();
  }
}

async function saveDraft(): Promise<void> {
  if (!manual.value.title.trim() || !manual.value.content.trim()) {
    toast.error('请先填写标题与正文');
    return;
  }
  try {
    await ipc('text2video_draft_save', {
      draft: {
        title: manual.value.title.trim(),
        author: manual.value.author.trim(),
        content: manual.value.content,
        source: manual.value.source ?? 'manual',
      },
    });
    toast.success('已保存到草稿箱');
  } catch (err) {
    toast.error(`保存草稿失败: ${err instanceof Error ? err.message : String(err)}`);
  }
}

/** 空输入框（''）兜底为默认字数 */
function safeWordCount(value: number | string): number {
  const n = Number(value);
  return Number.isFinite(n) && n > 0 ? Math.round(n) : 900;
}

async function generateArticle(): Promise<void> {
  const config = {
    baseUrl: globalSettings.aiBaseUrl,
    apiKey: globalSettings.aiApiKey,
    model: globalSettings.aiModel,
  };
  if (!config.baseUrl || !config.apiKey || !config.model) {
    toast.error('请先在「系统设置 → AI」中配置接口地址、API Key 与模型');
    return;
  }
  if (aiMode.value === 'free' && !aiPrompt.value.trim()) {
    toast.error('请填写提示词');
    return;
  }
  if (aiMode.value === 'structured' && !aiTopic.value.trim()) {
    toast.error('请填写主题');
    return;
  }

  aiLoading.value = true;
  try {
    const article = await ipc<AiArticle>('text2video_ai_generate', {
      config,
      request:
        aiMode.value === 'free'
          ? { mode: 'free', prompt: aiPrompt.value }
          : {
              mode: 'structured',
              topic: aiTopic.value,
              style: aiStyle.value,
              wordCount: safeWordCount(aiWordCount.value),
            },
    });
    manual.value = { title: article.title, author: '', content: article.body, source: 'ai' };
    sourceMode.value = 'manual';
    toast.success('AI 文章已生成，可编辑后存草稿或直接生成');
    void notify('AI 文章已生成', article.title);
  } catch (err) {
    toast.error(`AI 生成失败: ${err instanceof Error ? err.message : String(err)}`);
  } finally {
    aiLoading.value = false;
  }
}

onMounted(checkEnv);
</script>

<template>
  <ToolShell title="视频生成" description="输入或 AI 创作文章 → 竖屏滚动短视频">
    <template #actions>
      <Button variant="outline" size="sm" :disabled="envLoading" @click="checkEnv">
        <RefreshCw class="mr-1 size-3.5" :class="{ 'animate-spin': envLoading }" />
        环境检查
      </Button>
    </template>

    <div class="mx-auto grid w-full grid-cols-12 gap-4">
      <div class="col-span-12 space-y-4 lg:col-start-2 lg:col-span-10">
        <!-- 环境状态 -->
        <div
          class="flex flex-wrap items-center gap-x-5 gap-y-2 rounded-lg border bg-card px-4 py-3 text-sm"
        >
          <span class="flex items-center gap-1.5">
            <component
              :is="env?.ffmpegOk ? CheckCircle2 : XCircle"
              class="size-4"
              :class="env?.ffmpegOk ? 'text-success' : 'text-destructive'"
            />
            ffmpeg
            <span class="text-xs text-muted-foreground">{{ env?.ffmpegPath || '未找到' }}</span>
          </span>
          <span class="flex items-center gap-1.5">
            <component
              :is="env?.fontsOk ? CheckCircle2 : XCircle"
              class="size-4"
              :class="env?.fontsOk ? 'text-success' : 'text-destructive'"
            />
            中文字体
          </span>
          <span class="flex min-w-0 items-center gap-1.5">
            <Info class="size-4 text-muted-foreground" />
            输出
            <span class="truncate text-xs text-muted-foreground">{{ env?.outputDir }}</span>
          </span>
        </div>

        <!-- 来源切换 -->
        <div class="inline-flex rounded-lg border bg-card p-1">
          <Button
            :variant="sourceMode === 'manual' ? 'default' : 'ghost'"
            size="sm"
            @click="sourceMode = 'manual'"
          >
            <PenLine class="mr-1 size-3.5" />
            手动输入
          </Button>
          <Button
            :variant="sourceMode === 'ai' ? 'default' : 'ghost'"
            size="sm"
            @click="sourceMode = 'ai'"
          >
            <Sparkles class="mr-1 size-3.5" />
            AI 生成
          </Button>
        </div>

        <!-- 手动输入 -->
        <div v-if="sourceMode === 'manual'" class="space-y-3 rounded-lg border bg-card p-4">
          <ArticleForm v-model="manualForm" />
          <div class="flex items-center gap-2">
            <Button variant="outline" :disabled="running" @click="saveDraft">
              <Save class="mr-1 size-4" />
              存为草稿
            </Button>
            <span v-if="manual.source === 'ai'" class="text-xs text-muted-foreground">
              当前内容来自 AI 创作
            </span>
          </div>
        </div>

        <!-- AI 生成 -->
        <div v-else class="space-y-3 rounded-lg border bg-card p-4">
          <div class="inline-flex rounded-lg border p-0.5">
            <Button
              :variant="aiMode === 'structured' ? 'secondary' : 'ghost'"
              size="sm"
              @click="aiMode = 'structured'"
            >
              结构化
            </Button>
            <Button
              :variant="aiMode === 'free' ? 'secondary' : 'ghost'"
              size="sm"
              @click="aiMode = 'free'"
            >
              自由提示词
            </Button>
          </div>

          <div v-if="aiMode === 'structured'" class="grid grid-cols-1 gap-4 sm:grid-cols-3">
            <div class="space-y-1.5">
              <Label for="ai-topic">主题</Label>
              <Input id="ai-topic" v-model="aiTopic" placeholder="如：坚持的意义" />
            </div>
            <div class="space-y-1.5">
              <Label for="ai-style">风格</Label>
              <Input id="ai-style" v-model="aiStyle" placeholder="如：励志、散文、科普" />
            </div>
            <div class="space-y-1.5">
              <Label for="ai-words">字数</Label>
              <Input id="ai-words" v-model.number="aiWordCount" type="number" min="100" />
            </div>
          </div>
          <div v-else class="space-y-1.5">
            <div class="flex items-center justify-between">
              <Label for="ai-prompt">提示词</Label>
              <Button
                v-if="aiPrompt"
                variant="ghost"
                size="sm"
                class="h-7 text-xs"
                @click="aiPrompt = ''"
              >
                <Eraser class="mr-1 size-3.5" />
                清空
              </Button>
            </div>
            <Textarea
              id="ai-prompt"
              v-model="aiPrompt"
              :rows="6"
              placeholder="描述你想要的文章，例如：写一篇关于「早起的价值」的励志短文，约 900 字"
            />
            <p class="text-xs text-muted-foreground">提示词会自动保存，下次打开仍会显示。</p>
          </div>

          <Button :disabled="aiLoading" @click="generateArticle">
            <Sparkles class="mr-1 size-4" :class="{ 'animate-pulse': aiLoading }" />
            {{ aiLoading ? '生成中…' : '生成文章' }}
          </Button>
          <p class="text-xs text-muted-foreground">
            生成后会填入「手动输入」表单，可编辑、存草稿或直接出片。AI 配置在「系统设置 → AI」。
          </p>
        </div>

        <!-- 通用控制 -->
        <div class="flex flex-wrap items-center gap-3 rounded-lg border bg-card p-4">
          <div class="w-40 space-y-1.5">
            <Label>主题</Label>
            <Select v-model="settings.themeMode">
              <SelectTrigger class="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="opt in THEME_OPTIONS" :key="opt.value" :value="opt.value">
                  {{ opt.label }}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="flex items-end gap-2 pt-5">
            <Button :disabled="running || sourceMode === 'ai'" @click="start">
              <Play class="mr-1 size-4" />
              开始生成
            </Button>
            <Button v-if="running" variant="destructive" @click="cancel">
              <Square class="mr-1 size-4" />
              取消
            </Button>
          </div>
        </div>

        <!-- 进度 -->
        <div v-if="running || logs.length" class="space-y-2 rounded-lg border bg-card p-4">
          <div class="flex items-center justify-between text-sm">
            <span class="font-medium">
              {{ STAGE_LABELS[progress.stage] ?? (progress.stage || '就绪') }}
            </span>
            <span v-if="progress.total" class="text-xs text-muted-foreground">
              {{ progress.current }} / {{ progress.total }}
            </span>
          </div>
          <div class="h-1.5 w-full overflow-hidden rounded-full bg-muted">
            <div
              class="h-full rounded-full bg-primary transition-all"
              :style="{ width: `${progressPercent}%` }"
            />
          </div>
          <pre
            class="max-h-48 overflow-auto whitespace-pre-wrap break-words rounded bg-console p-3 font-mono text-xs leading-relaxed text-muted-foreground"
            >{{ logs.join('\n') }}</pre>
        </div>

        <!-- 结果 -->
        <div v-if="summary" class="space-y-2">
          <h3 class="text-sm font-semibold">
            本次结果
            <span class="ml-2 text-xs font-normal text-muted-foreground">
              {{ summary.cancelled ? '已取消' : '完成' }} · 输出到
              {{ summary.outputDir }}
            </span>
          </h3>
          <div class="space-y-2">
            <div
              v-for="item in summary.results"
              :key="item.refId"
              class="flex items-center justify-between gap-3 rounded-lg border bg-card px-4 py-2.5"
            >
              <div class="flex min-w-0 items-center gap-3">
                <Film class="size-4 shrink-0 text-muted-foreground" />
                <div class="min-w-0">
                  <p class="truncate text-sm">{{ item.title }}</p>
                  <p v-if="item.detail" class="truncate text-xs text-muted-foreground">
                    {{ item.detail }}
                  </p>
                </div>
              </div>
              <div class="flex shrink-0 items-center gap-2">
                <Badge :variant="item.status === 'done' ? 'default' : 'secondary'">
                  {{ statusLabel(item.status) }}
                </Badge>
                <template v-if="item.status === 'done'">
                  <Button variant="ghost" size="sm" @click="openPath(item.video, false)">
                    打开视频
                  </Button>
                  <Button variant="ghost" size="sm" @click="openPath(item.video, true)">
                    <FolderOpen class="mr-1 size-3.5" />
                    位置
                  </Button>
                </template>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </ToolShell>
</template>
